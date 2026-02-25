//! HTTP client with TLS fingerprint emulation and per-domain sessions.
//!
//! Uses wreq (reqwest fork) with Chrome TLS/HTTP2 fingerprint emulation
//! to bypass Cloudflare's TLS fingerprint checks.

use std::collections::HashMap;
use std::time::Duration;

use wreq::header::{HeaderName, HeaderValue, CONTENT_TYPE, COOKIE, REFERER, USER_AGENT};
use tracing::{debug, warn};
use url::Url;
use wreq_util::Emulation;

use hagitori_core::error::{HagitoriError, Result};

use crate::session_store::DomainSessionStore;

/// default UA sent when the session store has no UA for the domain.
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub headers: Option<HashMap<String, String>>,
    pub timeout: Option<Duration>,
    pub referer: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct HttpClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

/// HTTP client with TLS emulation and per-domain session management.
///
#[derive(Clone)]
pub struct HttpClient {
    client: wreq::Client,
    session_store: DomainSessionStore,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        Self::with_config(HttpClientConfig::default())
    }

    pub fn with_config(config: HttpClientConfig) -> Result<Self> {
        let client = wreq::Client::builder()
            .emulation(Emulation::Chrome145)
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .cookie_store(false) // managed manually via session store
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .map_err(|e| HagitoriError::http(format!("failed to create HTTP client: {e}")))?;

        tracing::info!("HttpClient created (wreq, Chrome145 TLS emulation)");

        Ok(Self {
            client,
            session_store: DomainSessionStore::new(),
        })
    }

    pub fn session_store(&self) -> &DomainSessionStore {
        &self.session_store
    }

    // ─── GET ────────────────────────────────────────────────

    pub async fn get(
        &self,
        url: &str,
        options: Option<RequestOptions>,
    ) -> Result<wreq::Response> {
        let request = self.client.get(url);
        let response = self
            .send_request("GET", url, request, options.unwrap_or_default())
            .await?;

        if response.status() == wreq::StatusCode::FORBIDDEN {
            debug!(url = url, "403 Forbidden   may need Cloudflare bypass");
        }

        Ok(response)
    }

    pub async fn get_text(&self, url: &str, options: Option<RequestOptions>) -> Result<String> {
        let response = self.get(url, options).await?;
        let status = response.status();

        if !status.is_success() {
            return Err(HagitoriError::http(format!(
                "GET {url} returned status {status}"
            )));
        }

        response
            .text()
            .await
            .map_err(|e| HagitoriError::http(format!("failed to read body from {url}: {e}")))
    }

    pub async fn get_bytes(&self, url: &str, options: Option<RequestOptions>) -> Result<Vec<u8>> {
        let response = self.get(url, options).await?;
        let status = response.status();

        if !status.is_success() {
            return Err(HagitoriError::http(format!(
                "GET {url} returned status {status}"
            )));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| HagitoriError::http(format!("failed to read bytes from {url}: {e}")))
    }

    // ─── POST ───────────────────────────────────────────────

    pub async fn post(
        &self,
        url: &str,
        body: &serde_json::Value,
        options: Option<RequestOptions>,
    ) -> Result<wreq::Response> {
        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| HagitoriError::http(format!("failed to serialize JSON body: {e}")))?;

        let request = self
            .client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_bytes);

        self.send_request("POST", url, request, options.unwrap_or_default())
            .await
    }

    pub async fn post_form(
        &self,
        url: &str,
        form_data: &HashMap<String, String>,
        options: Option<RequestOptions>,
    ) -> Result<wreq::Response> {
        let body: String = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(form_data.iter())
            .finish();

        let request = self
            .client
            .post(url)
            .header(
                CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .body(body);

        self.send_request("POST form", url, request, options.unwrap_or_default())
            .await
    }

    pub async fn post_empty(
        &self,
        url: &str,
        options: Option<RequestOptions>,
    ) -> Result<wreq::Response> {
        let request = self.client.post(url).header("Content-Length", "0");

        self.send_request("POST empty", url, request, options.unwrap_or_default())
            .await
    }

    // ─── internals ──────────────────────────────────────────

    /// applies session data (UA, cookies, headers), timeout, sends the request, and logs the result.  
    /// all public HTTP methods delegate to this.
    async fn send_request(
        &self,
        method: &str,
        url: &str,
        request: wreq::RequestBuilder,
        opts: RequestOptions,
    ) -> Result<wreq::Response> {
        let parsed_url = Url::parse(url)?;
        let domain = Self::extract_domain(&parsed_url);

        debug!(url = url, domain = domain, "HTTP {method}");

        let mut request = self.apply_session_data(request, domain, &opts);

        debug!(
            url = url,
            domain = domain,
            has_session = self.session_store.has_session(domain),
            "HTTP {method}   session data applied"
        );

        if let Some(timeout) = opts.timeout {
            request = request.timeout(timeout);
        }

        let response = request
            .send()
            .await
            .map_err(|e| HagitoriError::http(format!("{method} {url} failed: {e}")))?;

        debug!(
            url = url,
            status = response.status().as_u16(),
            "HTTP {method} response"
        );
        Ok(response)
    }

    fn apply_session_data(
        &self,
        request: wreq::RequestBuilder,
        domain: &str,
        opts: &RequestOptions,
    ) -> wreq::RequestBuilder {
        // build a HeaderMap so we use .headers() (insert/replace) instead of .header()
        let mut headers = wreq::header::HeaderMap::new();

        let session = self.session_store.get(domain);

        // User-Agent: session store override > default constant
        let ua = session
            .as_ref()
            .and_then(|s| s.user_agent.clone())
            .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        if let Ok(val) = HeaderValue::from_str(&ua) {
            headers.insert(USER_AGENT, val);
        } else {
            warn!(domain = domain, ua = ua.as_str(), "session UA contains invalid header characters   skipped");
        }
        debug!(domain = domain, user_agent = ua.as_str(), "session UA applied");

        // cookies from session snapshot
        if let Some(cookies) = session.as_ref().map(|s| &s.cookies).filter(|c| !c.is_empty()) {
            let cookie_str: String = cookies
                .iter()
                .filter(|(k, _)| {
                    let valid = !k.contains('=') && !k.contains(';') && !k.contains('\0');
                    if !valid {
                        warn!(domain = domain, key = k.as_str(), "cookie name contains invalid characters   skipped");
                    }
                    valid
                })
                .map(|(k, v)| {
                    let sanitized = v.replace(';', "%3B");
                    format!("{k}={sanitized}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            debug!(
                domain = domain,
                cookie_count = cookies.len(),
                cookie_header = cookie_str.as_str(),
                "applying session cookies"
            );
            if let Ok(val) = HeaderValue::from_str(&cookie_str) {
                headers.insert(COOKIE, val);
            } else {
                warn!(domain = domain, "session cookie string contains invalid header characters   skipped");
            }
        } else {
            debug!(domain = domain, "no session cookies found");
        }

        // headers reserved for internal management skip if set in session or opts
        const RESERVED: &[wreq::header::HeaderName] = &[USER_AGENT, COOKIE];

        // custom headers from session snapshot
        if let Some(ref sess) = session {
            for (key, value) in &sess.headers {
                match (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    (Ok(name), Ok(val)) => {
                        if RESERVED.contains(&name) {
                            warn!(domain = domain, key = key.as_str(), "session header conflicts with managed header   skipped");
                            continue;
                        }
                        headers.insert(name, val);
                    }
                    (Err(e), _) => warn!(domain = domain, key = key.as_str(), error = %e, "invalid session header name   skipped"),
                    (_, Err(e)) => warn!(domain = domain, key = key.as_str(), error = %e, "invalid session header value   skipped"),
                }
            }
        }

        // extra headers from request options
        if let Some(ref extra_headers) = opts.headers {
            for (key, value) in extra_headers {
                match (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    (Ok(name), Ok(val)) => { headers.insert(name, val); }
                    (Err(e), _) => warn!(key = key.as_str(), error = %e, "invalid request header name   skipped"),
                    (_, Err(e)) => warn!(key = key.as_str(), error = %e, "invalid request header value   skipped"),
                }
            }
        }

        // referer
        if let Some(ref referer) = opts.referer {
            match HeaderValue::from_str(referer) {
                Ok(val) => { headers.insert(REFERER, val); }
                Err(e) => warn!(referer = referer.as_str(), error = %e, "invalid referer header value   skipped"),
            }
        }

        // apply all headers at once using .headers() which does insert (replace)
        // instead of .header() which does append (duplicate)
        request.headers(headers)
    }

    fn extract_domain(url: &Url) -> &str {
        url.host_str().unwrap_or("unknown")
    }
}

impl std::fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("session_store", &self.session_store)
            .finish()
    }
}

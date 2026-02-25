//! QuickJS HTTP API bindings for extensions (fetch, request methods).

#![expect(clippy::needless_pass_by_value, reason = "rquickjs FromJs requires String, not &str")]

use std::collections::HashMap;
use std::sync::Arc;

use rquickjs::{Class, Ctx, Function, Object, Value};
use rquickjs::class::Trace;
use rquickjs::prelude::{Async, Opt};

use hagitori_http::RequestOptions;

use crate::runtime::RuntimeData;

// ─── FetchResponse: native JS class via #[rquickjs::class] ────────────────

#[derive(Trace, rquickjs::JsLifetime)]
#[rquickjs::class(rename = "FetchResponse")]
pub struct FetchResponse {
    status_val: i32,
    body: String,
    headers_map: HashMap<String, String>,
}

#[rquickjs::methods]
impl FetchResponse {
    /// response.status -> number
    #[qjs(get)]
    pub fn status(&self) -> i32 {
        self.status_val
    }

    /// response.headers -> Object with key/value header pairs
    #[qjs(get)]
    pub fn headers<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let obj = Object::new(ctx)?;
        for (k, v) in &self.headers_map {
            obj.set::<&str, String>(k.as_str(), v.clone())?;
        }
        Ok(obj)
    }

    /// response.text() -> String (full body)
    pub fn text(&self) -> String {
        self.body.clone()
    }

    /// response.json() -> JS value parsed from body
    pub fn json<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let parsed: serde_json::Value =
            serde_json::from_str(&self.body).map_err(|e| {
                rquickjs::Error::new_from_js_message(
                    "string",
                    "json",
                    &format!("JSON parse error: {e}"),
                )
            })?;
        crate::runtime::json_to_js_value(&ctx, &parsed).map_err(|e| {
            rquickjs::Error::new_from_js_message("json", "value", &e)
        })
    }

    /// response.bytes() -> Array<number>
    pub fn bytes(&self) -> Vec<i32> {
        self.body.as_bytes().iter().map(|&b| b as i32).collect()
    }
}

/// fetch options extracted from the JS Object (all Send)
struct FetchOptions {
    method: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    form_data: Option<HashMap<String, String>>,
    referer: Option<String>,
}

/// extracts options from the JS Object synchronously
fn parse_fetch_options<'js>(opts: &Opt<Object<'js>>) -> rquickjs::Result<FetchOptions> {
    let mut result = FetchOptions {
        method: "GET".to_string(),
        headers: None,
        body: None,
        form_data: None,
        referer: None,
    };

    if let Some(ref opts_obj) = opts.0 {
        let m: Value = opts_obj.get("method")?;
        if let Some(s) = m.as_string() {
            result.method = s.to_string()?.to_uppercase();
        }

        let h: Value = opts_obj.get("headers")?;
        if let Some(headers_obj) = h.as_object() {
            let mut map = HashMap::new();
            for key in headers_obj.keys::<String>() {
                let key = key?;
                let val: rquickjs::Coerced<String> = headers_obj.get(&key)?;
                map.insert(key, val.0);
            }
            result.headers = Some(map);
        }

        let b: Value = opts_obj.get("body")?;
        if let Some(s) = b.as_string() {
            result.body = Some(s.to_string()?);
        }

        let f: Value = opts_obj.get("form")?;
        if let Some(form_obj) = f.as_object() {
            let mut map = HashMap::new();
            for key in form_obj.keys::<String>() {
                let key = key?;
                let val: rquickjs::Coerced<String> = form_obj.get(&key)?;
                map.insert(key, val.0);
            }
            result.form_data = Some(map);
        }

        let r: Value = opts_obj.get("referer")?;
        if let Some(s) = r.as_string() {
            result.referer = Some(s.to_string()?);
        }
    }

    Ok(result)
}

/// extracts status + headers from an HTTP response
fn extract_response_meta(resp: &wreq::Response) -> (u16, HashMap<String, String>) {
    let status = resp.status().as_u16();
    let headers: HashMap<String, String> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    (status, headers)
}

/// extracts status, body text, and headers from an HTTP response in one step.
async fn finalize_response(
    resp: wreq::Response,
) -> std::result::Result<(u16, String, HashMap<String, String>), String> {
    let (status, headers) = extract_response_meta(&resp);
    let body = resp
        .text()
        .await
        .map_err(|e| format!("failed to read response body: {e}"))?;
    Ok((status, body, headers))
}

/// executes the actual HTTP request (async). Supports GET & POST
async fn execute_request(
    data: &RuntimeData,
    url: &str,
    opts: &FetchOptions,
) -> std::result::Result<(u16, String, HashMap<String, String>), String> {
    tracing::debug!(
        url = url,
        method = opts.method.as_str(),
        has_headers = opts.headers.is_some(),
        has_body = opts.body.is_some(),
        has_form = opts.form_data.is_some(),
        "fetch() starting request"
    );

    // log actual headers from the extension (debug level)
    if let Some(ref hdrs) = opts.headers {
        for (k, v) in hdrs {
            tracing::debug!(key = k.as_str(), value = v.as_str(), "fetch() header from extension");
        }
    }
    if let Some(ref form) = opts.form_data {
        tracing::debug!(form_keys = ?form.keys().collect::<Vec<_>>(), "fetch() form_data keys");
    }

    let request_opts = RequestOptions {
        headers: opts.headers.clone(),
        timeout: None,
        referer: opts.referer.clone(),
    };

    let result = match opts.method.as_str() {
        "POST" => {
            if let Some(ref form) = opts.form_data {
                let resp = data.http_client
                    .post_form(url, form, Some(request_opts))
                    .await
                    .map_err(|e| format!("HTTP POST form error: {e}"))?;
                finalize_response(resp).await
            } else if let Some(ref body_str) = opts.body {
                let content_type = opts.headers.as_ref()
                    .and_then(|h| h.iter().find(|(k, _)| k.eq_ignore_ascii_case("content-type")))
                    .map(|(_, v)| v.as_str());

                let is_form = match content_type {
                    Some(ct) if ct.contains("application/x-www-form-urlencoded") => true,
                    Some(ct) if ct.contains("application/json") => false,
                    // fallback: heuristic for legacy extensions without content-type
                    _ => body_str.contains('=') && body_str.contains('&') && !body_str.starts_with('{'),
                };

                if is_form {
                    let form: HashMap<String, String> =
                        url::form_urlencoded::parse(body_str.as_bytes())
                            .into_owned()
                            .collect();
                    let resp = data.http_client
                        .post_form(url, &form, Some(request_opts))
                        .await
                        .map_err(|e| format!("HTTP POST form error: {e}"))?;
                    finalize_response(resp).await
                } else {
                    let json_body: serde_json::Value = match serde_json::from_str(body_str) {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(
                                url = url,
                                error = %e,
                                "POST body is not valid JSON   wrapping as JSON string"
                            );
                            serde_json::Value::String(body_str.clone())
                        }
                    };
                    let resp = data.http_client
                        .post(url, &json_body, Some(request_opts))
                        .await
                        .map_err(|e| format!("HTTP POST error: {e}"))?;
                    finalize_response(resp).await
                }
            } else {
                let resp = data.http_client
                    .post_empty(url, Some(request_opts))
                    .await
                    .map_err(|e| format!("HTTP POST error: {e}"))?;
                finalize_response(resp).await
            }
        }
        "PUT" | "DELETE" | "PATCH" | "HEAD" => {
            Err(format!("HTTP method {} is not supported   use GET or POST", opts.method))
        }
        _ => {
            let resp = data.http_client
                .get(url, Some(request_opts))
                .await
                .map_err(|e| format!("HTTP GET error: {e}"))?;
            finalize_response(resp).await
        }
    };

    match &result {
        Ok((status, body, _)) => {
            tracing::info!(
                url = url,
                status = *status,
                body_len = body.len(),
                "fetch() completed"
            );
        }
        Err(e) => {
            tracing::error!(url = url, error = e.as_str(), "fetch() failed");
        }
    }

    result
}

pub fn register<'js>(ctx: &Ctx<'js>, data: Arc<RuntimeData>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    // register FetchResponse class prototype (without exposing constructor to JS)
    Class::<FetchResponse>::define(&globals)?;

    // fetch(url, options?) -> Promise<FetchResponse>
    globals.set(
        "fetch",
        Function::new(
            ctx.clone(),
            Async(move |url: String, opts: Opt<Object<'_>>| {
                let data = data.clone();

                // extract options synchronously (before the async block)
                let parsed = parse_fetch_options(&opts);

                async move {
                    let opts = parsed.map_err(|e| {
                        rquickjs::Error::new_from_js_message("fetch", "options", &format!("{e}"))
                    })?;

                    // execute HTTP request
                    let (status, response_body, response_headers) =
                        execute_request(&data, &url, &opts).await.map_err(|e| {
                            rquickjs::Error::new_from_js_message("fetch", "response", &e)
                        })?;

                    Ok::<_, rquickjs::Error>(FetchResponse {
                        status_val: status as i32,
                        body: response_body,
                        headers_map: response_headers,
                    })
                }
            }),
        )?,
    )?;

    Ok(())
}

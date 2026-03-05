#![expect(clippy::needless_pass_by_value, reason = "rquickjs FromJs requires String, not &str")]

//! Extension JS API for browser automation.
//!
//! ## Exposed functions
//! - `browser.interceptRequests(url, patterns, options?)` -> `InterceptedRequest[]`
//! - `browser.interceptResponses(url, patterns, options?)` -> `InterceptedResponse[]`
//! - `browser.intercept(url, options)` -> `{ requests, responses }`
//! - `browser.getCookies(url)` -> `{ [name]: value }`
//! - `browser.bypassCloudflare(url, options?)` -> `CloudflareResult`
//! - `browser.close()` -> `void`

use std::collections::HashMap;
use std::sync::Arc;

use rquickjs::class::Trace;
use rquickjs::prelude::{Async, Opt};
use rquickjs::{Class, Ctx, Function, Object, Value};

use crate::runtime::RuntimeData;

// ─── JS class: CloudflareResult ─────────────────────────────────────────────
// Returned by browser.bypassCloudflare(). Stores cookies + user-agent.

#[derive(Trace, rquickjs::JsLifetime)]
#[rquickjs::class(rename = "CloudflareResult")]
pub struct JsCloudflareResult {
    cookies_json: String,
    user_agent_val: String,
    has_clearance: bool,
}

#[rquickjs::methods]
impl JsCloudflareResult {
    /// result.cookies -> { [name]: value }
    #[qjs(get)]
    pub fn cookies<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let parsed: serde_json::Value = serde_json::from_str(&self.cookies_json)
            .map_err(|e| rquickjs::Error::new_from_js_message("CloudflareResult", "cookies", &format!("{e}")))?;
        crate::runtime::json_to_js_value(&ctx, &parsed)
            .map_err(|e| rquickjs::Error::new_from_js_message("CloudflareResult", "cookies", &e))
    }

    /// result.userAgent -> string
    #[qjs(get, rename = "userAgent")]
    pub fn user_agent(&self) -> String {
        self.user_agent_val.clone()
    }

    /// result.hasCfClearance -> bool
    #[qjs(get, rename = "hasCfClearance")]
    pub fn has_cf_clearance(&self) -> bool {
        self.has_clearance
    }

    /// result.cookieHeader -> "name=value; name2=value2"
    #[qjs(get, rename = "cookieHeader")]
    pub fn cookie_header(&self) -> String {
        let map: HashMap<String, String> = serde_json::from_str(&self.cookies_json).unwrap_or_default();
        map.iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

// ─── JS class: InterceptResult ──────────────────────────────────────────────
// Returned by browser.intercept(). Stores requests + responses as JSON.

#[derive(Trace, rquickjs::JsLifetime)]
#[rquickjs::class(rename = "InterceptResult")]
pub struct JsInterceptResult {
    requests_json: String,
    responses_json: String,
}

#[rquickjs::methods]
impl JsInterceptResult {
    /// result.requests -> InterceptedRequest[]
    #[qjs(get)]
    pub fn requests<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let parsed: serde_json::Value = serde_json::from_str(&self.requests_json)
            .map_err(|e| rquickjs::Error::new_from_js_message("InterceptResult", "requests", &format!("{e}")))?;
        crate::runtime::json_to_js_value(&ctx, &parsed)
            .map_err(|e| rquickjs::Error::new_from_js_message("InterceptResult", "requests", &e))
    }

    /// result.responses -> InterceptedResponse[]
    #[qjs(get)]
    pub fn responses<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let parsed: serde_json::Value = serde_json::from_str(&self.responses_json)
            .map_err(|e| rquickjs::Error::new_from_js_message("InterceptResult", "responses", &format!("{e}")))?;
        crate::runtime::json_to_js_value(&ctx, &parsed)
            .map_err(|e| rquickjs::Error::new_from_js_message("InterceptResult", "responses", &e))
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────────

async fn get_browser(
    data: &RuntimeData,
    headless: bool,
) -> Result<Arc<hagitori_browser::BrowserManager>, rquickjs::Error> {
    {
        let guard = data.browser_manager.lock().await;
        if let Some(bm) = guard.as_ref() {
            if !headless && bm.is_headless() {
                tracing::info!("relaunching browser in headful mode");
                drop(guard);
            } else {
                return Ok(bm.clone());
            }
        }
    }

    let options = hagitori_browser::BrowserOptions {
        headless,
        window_width: if headless { 1920 } else { 1280 },
        window_height: if headless { 1080 } else { 800 },
        ..Default::default()
    };

    tracing::info!("launching browser on demand (headless={headless})");
    let new_browser = hagitori_browser::BrowserManager::launch_with_options(options)
        .await
        .map_err(|e| browser_err("launch", e))?;

    let bm = Arc::new(new_browser);
    let mut guard = data.browser_manager.lock().await;
    *guard = Some(bm.clone());
    Ok(bm)
}

fn browser_err(method: &'static str, e: impl std::fmt::Display) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message("browser", method, &format!("{e}"))
}

/// extracts a string array from a JS value.
fn extract_string_array<'js>(val: &Value<'js>) -> rquickjs::Result<Vec<String>> {
    let mut result = Vec::new();
    if let Some(arr) = val.as_array() {
        for i in 0..arr.len() {
            let item: Value = arr.get(i)?;
            if let Some(s) = item.as_string() {
                result.push(s.to_string()?);
            }
        }
    }
    Ok(result)
}

/// serializes `InterceptedRequest[]` to JSON.
fn serialize_requests(requests: &[hagitori_browser::InterceptedRequest]) -> String {
    let arr: Vec<serde_json::Value> = requests
        .iter()
        .map(|r| {
            serde_json::json!({
                "url": r.url,
                "method": r.method,
                "headers": r.headers,
                "postBody": r.post_body,
                "resourceType": r.resource_type,
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// serializes `InterceptedResponse[]` to JSON, auto-decoding base64 bodies.
fn serialize_responses(responses: &[hagitori_browser::InterceptedResponse]) -> String {
    let arr: Vec<serde_json::Value> = responses
        .iter()
        .map(|r| {
            let body_val = if r.base64_encoded {
                use base64::Engine;
                match base64::engine::general_purpose::STANDARD.decode(&r.body) {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        serde_json::from_str::<serde_json::Value>(&text)
                            .unwrap_or(serde_json::Value::String(text))
                    }
                    Err(_) => serde_json::Value::String(r.body.clone()),
                }
            } else {
                serde_json::from_str::<serde_json::Value>(&r.body)
                    .unwrap_or(serde_json::Value::String(r.body.clone()))
            };
            serde_json::json!({
                "url": r.url,
                "status": r.status,
                "body": body_val,
                "headers": r.headers,
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

// ─── browser.* API registration ─────────────────────────────────────────────

pub fn register<'js>(ctx: &Ctx<'js>, data: Arc<RuntimeData>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    Class::<JsCloudflareResult>::define(&globals)?;
    Class::<JsInterceptResult>::define(&globals)?;

    let browser_obj = Object::new(ctx.clone())?;

    // browser.interceptRequests(url, patterns, options?) -> Promise<Value>
    // Returns array of intercepted requests matching the patterns.
    //
    // options: { waitTime?: number (default 30), headless?: boolean (default false) }
    let data_ir = data.clone();
    browser_obj.set(
        "interceptRequests",
        Function::new(
            ctx.clone(),
            Async(move |url: String, patterns_val: Value<'_>, opts: Opt<Object<'_>>| {
                let data = data_ir.clone();
                let patterns = extract_string_array(&patterns_val);
                let wait_time = opts.0.as_ref()
                    .and_then(|o| o.get::<_, Value>("waitTime").ok())
                    .and_then(|v| v.as_int().or_else(|| v.as_float().map(|f| f as i32)))
                    .map(|n| n.max(1) as u64)
                    .unwrap_or(30);
                let headless = opts.0.as_ref()
                    .and_then(|o| o.get::<_, Value>("headless").ok())
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                async move {
                    let patterns = patterns?;

                    let browser = get_browser(&data, headless).await?;
                    let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();

                    let requests = hagitori_browser::intercept_requests(
                        &browser, &url, &pattern_refs, wait_time,
                    )
                    .await
                    .map_err(|e| browser_err("interceptRequests", e))?;

                    let json = serialize_requests(&requests);

                    // return raw JSON string   extension parses with JSON.parse()
                    Ok::<_, rquickjs::Error>(json)
                }
            }),
        )?,
    )?;

    // browser.interceptResponses(url, patterns, options?) -> Promise<Value>
    // Returns array of intercepted responses matching the patterns.
    //
    // options: { waitTime?: number (default 30), headless?: boolean (default false) }
    let data_iresp = data.clone();
    browser_obj.set(
        "interceptResponses",
        Function::new(
            ctx.clone(),
            Async(move |url: String, patterns_val: Value<'_>, opts: Opt<Object<'_>>| {
                let data = data_iresp.clone();
                let patterns = extract_string_array(&patterns_val);
                let wait_time = opts.0.as_ref()
                    .and_then(|o| o.get::<_, Value>("waitTime").ok())
                    .and_then(|v| v.as_int().or_else(|| v.as_float().map(|f| f as i32)))
                    .map(|n| n.max(1) as u64)
                    .unwrap_or(30);
                let headless = opts.0.as_ref()
                    .and_then(|o| o.get::<_, Value>("headless").ok())
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                async move {
                    let patterns = patterns?;

                    let browser = get_browser(&data, headless).await?;
                    let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();

                    let responses = hagitori_browser::intercept_responses(
                        &browser, &url, &pattern_refs, wait_time,
                    )
                    .await
                    .map_err(|e| browser_err("interceptResponses", e))?;

                    let json = serialize_responses(&responses);
                    Ok::<_, rquickjs::Error>(json)
                }
            }),
        )?,
    )?;

    // browser.intercept(url, options) -> Promise<InterceptResult>
    // Intercepts both requests and responses simultaneously.
    //
    // options: {
    //   requests?: string[],   // URL patterns for requests
    //   responses?: string[],  // URL patterns for responses
    //   waitTime?: number,     // timeout in seconds (default 30)
    //   headless?: boolean,    // use headless mode (default false)
    // }
    let data_ia = data.clone();
    browser_obj.set(
        "intercept",
        Function::new(
            ctx.clone(),
            Async(move |url: String, opts: Opt<Object<'_>>| {
                let data = data_ia.clone();

                let parsed = opts.0.as_ref().map(|o| {
                    let req_val: Value = o.get("requests").unwrap_or_else(|_| Value::new_undefined(o.ctx().clone()));
                    let resp_val: Value = o.get("responses").unwrap_or_else(|_| Value::new_undefined(o.ctx().clone()));
                    let wt: Value = o.get("waitTime").unwrap_or_else(|_| Value::new_undefined(o.ctx().clone()));
                    let hl: Value = o.get("headless").unwrap_or_else(|_| Value::new_undefined(o.ctx().clone()));

                    let req_patterns = extract_string_array(&req_val).unwrap_or_default();
                    let resp_patterns = extract_string_array(&resp_val).unwrap_or_default();
                    let wait_time = wt.as_int()
                        .or_else(|| wt.as_float().map(|f| f as i32))
                        .map(|n| n.max(1) as u64)
                        .unwrap_or(30);
                    let headless = hl.as_bool().unwrap_or(false);

                    (req_patterns, resp_patterns, wait_time, headless)
                });

                async move {
                    let (req_patterns, resp_patterns, wait_time, headless) = parsed.unwrap_or_default();

                    let browser = get_browser(&data, headless).await?;
                    let req_refs: Vec<&str> = req_patterns.iter().map(|s| s.as_str()).collect();
                    let resp_refs: Vec<&str> = resp_patterns.iter().map(|s| s.as_str()).collect();

                    let page_data = hagitori_browser::intercept_all(
                        &browser, &url, &req_refs, &resp_refs, wait_time,
                    )
                    .await
                    .map_err(|e| browser_err("intercept", e))?;

                    Ok::<_, rquickjs::Error>(JsInterceptResult {
                        requests_json: serialize_requests(&page_data.requests),
                        responses_json: serialize_responses(&page_data.responses),
                    })
                }
            }),
        )?,
    )?;

    // browser.getCookies(url) -> Promise<string>
    // returns JSON string of { name: value } cookie map.
    // always uses headful.
    let data_cookies = data.clone();
    browser_obj.set(
        "getCookies",
        Function::new(
            ctx.clone(),
            Async(move |url: String| {
                let data = data_cookies.clone();
                async move {
                    let browser = get_browser(&data, false).await?;
                    let cookies = browser
                        .get_cookies(&url)
                        .await
                        .map_err(|e| browser_err("getCookies", e))?;

                    let json = serde_json::to_string(&cookies)
                        .unwrap_or_else(|_| "{}".to_string());
                    Ok::<_, rquickjs::Error>(json)
                }
            }),
        )?,
    )?;

    // browser.bypassCloudflare(url, options?) -> Promise<CloudflareResult>
    //
    // options: { timeout?: number (seconds, default 60) }
    let data_cf = data.clone();
    browser_obj.set(
        "bypassCloudflare",
        Function::new(
            ctx.clone(),
            Async(move |url: String, opts: Opt<Object<'_>>| {
                let data = data_cf.clone();

                let auto_click = opts.0.as_ref()
                    .and_then(|o| o.get::<_, Value>("autoClick").ok())
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                async move {
                    let cf_opts = hagitori_browser::CloudflareBypassOptions {
                        auto_click,
                    };

                    // always use a persistent headful browser for bypass  
                    // CF cookies are tied to the browser fingerprint, so the same
                    // browser must be reused for subsequent intercept() calls.
                    let browser = get_browser(&data, false).await?;
                    let result = browser
                        .bypass_cloudflare_with_options(&url, &cf_opts)
                        .await
                        .map_err(|e| browser_err("bypassCloudflare", e))?;

                    // propagate cookies + user-agent to HTTP session store
                    if let Ok(parsed) = url::Url::parse(&url) {
                        let domain = parsed.host_str().unwrap_or("").to_string();

                        if !result.cookies.is_empty() {
                            data.http_client
                                .session_store()
                                .set_cookies(&domain, result.cookies.clone());
                            tracing::info!(
                                "propagated {} cookies to HTTP session (domain: {})",
                                result.cookies.len(),
                                domain,
                            );
                        }

                        // propagate browser UA so subsequent fetch() calls use the
                        // same User-Agent that obtained the cf_clearance cookie.
                        if !result.user_agent.is_empty() {
                            data.http_client
                                .session_store()
                                .set_user_agent(&domain, &result.user_agent);
                            tracing::info!(
                                "propagated browser User-Agent to HTTP session (domain: {})",
                                domain,
                            );
                        }
                    }

                    let cookies_json = serde_json::to_string(&result.cookies)
                        .unwrap_or_else(|_| "{}".to_string());
                    let has_clearance = result.has_cf_clearance();

                    Ok::<_, rquickjs::Error>(JsCloudflareResult {
                        cookies_json,
                        user_agent_val: result.user_agent,
                        has_clearance,
                    })
                }
            }),
        )?,
    )?;

    // browser.close() -> Promise<void>
    let data_close = data.clone();
    browser_obj.set(
        "close",
        Function::new(
            ctx.clone(),
            Async(move || {
                let data = data_close.clone();
                async move {
                    let mut guard = data.browser_manager.lock().await;
                    *guard = None;
                    Ok::<_, rquickjs::Error>(())
                }
            }),
        )?,
    )?;

    globals.set("browser", browser_obj)?;
    Ok(())
}

/// registers a stub that throws errors when browser.* is called without the capability.
pub fn register_stub<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    let browser_obj = Object::new(ctx.clone())?;

    let stub_methods = [
        "interceptRequests",
        "interceptResponses",
        "intercept",
        "getCookies",
        "bypassCloudflare",
    ];

    for method in stub_methods {
        let method_name = method.to_string();
        let msg = format!("browser.{method_name}: extension does not have 'browser' capability enabled");
        browser_obj.set(
            method,
            Function::new(ctx.clone(), move || -> rquickjs::Result<String> {
                Err(rquickjs::Error::new_from_js_message(
                    "browser",
                    "stub",
                    &msg,
                ))
            })?,
        )?;
    }

    browser_obj.set("close", Function::new(ctx.clone(), || {})?)?;

    globals.set("browser", browser_obj)?;
    Ok(())
}

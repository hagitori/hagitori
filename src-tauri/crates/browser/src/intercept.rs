//! network interception via CDP events.

use std::collections::HashMap;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::network::{
    EventRequestWillBeSent, EventResponseReceived, GetResponseBodyParams,
};
use chromiumoxide::Page;
use futures::StreamExt;

use crate::error::BrowserError;
use crate::manager::{close_page_quietly, BrowserManager};
use crate::types::{InterceptedPageData, InterceptedRequest, InterceptedResponse};

/// downloads image bytes by navigating a pre-created page to the URL
/// and capturing the first successful response body via CDP.
///
/// the page is NOT closed, caller manages its lifecycle.
pub async fn download_image_with_page(
    page: &Page,
    url: &str,
    timeout_seconds: u64,
) -> Result<Vec<u8>, BrowserError> {
    let mut response_events = page.event_listener::<EventResponseReceived>().await?;

    page.goto(url).await?;

    let deadline = tokio::time::sleep(Duration::from_secs(timeout_seconds));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => {
                return Err(BrowserError::Interaction(format!(
                    "timeout waiting for image response: {url}"
                )));
            }
            event = response_events.next() => {
                match event {
                    Some(ev) => {
                        let status: u16 = ev.response.status.try_into().unwrap_or(0);
                        if status != 200 {
                            continue;
                        }

                        match page.execute(GetResponseBodyParams::new(ev.request_id.clone())).await {
                            Ok(body) => {
                                let bytes = if body.result.base64_encoded {
                                    use base64::Engine;
                                    base64::engine::general_purpose::STANDARD
                                        .decode(&body.result.body)
                                        .unwrap_or_default()
                                } else {
                                    body.result.body.into_bytes()
                                };

                                if !bytes.is_empty() {
                                    return Ok(bytes);
                                }
                            }
                            Err(e) => {
                                tracing::debug!("failed to get response body for {url}: {e:?}");
                            }
                        }
                    }
                    None => {
                        return Err(BrowserError::Interaction(format!(
                            "CDP event stream ended while downloading: {url}"
                        )));
                    }
                }
            }
        }
    }
}

/// intercepts HTTP requests matching the given URL patterns.
/// thin wrapper over `intercept_all` passes empty response patterns.
pub async fn intercept_requests(
    browser: &BrowserManager,
    url: &str,
    patterns: &[&str],
    timeout_seconds: u64,
) -> Result<Vec<InterceptedRequest>, BrowserError> {
    let data = intercept_all(browser, url, patterns, &[], timeout_seconds).await?;
    Ok(data.requests)
}

/// intercepts HTTP responses matching the given URL patterns.
/// thin wrapper over `intercept_all` passes empty request patterns.
pub async fn intercept_responses(
    browser: &BrowserManager,
    url: &str,
    patterns: &[&str],
    timeout_seconds: u64,
) -> Result<Vec<InterceptedResponse>, BrowserError> {
    let data = intercept_all(browser, url, &[], patterns, timeout_seconds).await?;
    Ok(data.responses)
}

/// intercepts both HTTP requests and responses simultaneously.
/// the caller is responsible for any pre-navigation setup (cookies, CF bypass, etc.).
pub async fn intercept_all(
    browser: &BrowserManager,
    url: &str,
    request_patterns: &[&str],
    response_patterns: &[&str],
    timeout_seconds: u64,
) -> Result<InterceptedPageData, BrowserError> {
    let page = browser.new_page(None).await?;

    let mut request_events = page.event_listener::<EventRequestWillBeSent>().await?;
    let mut response_events = page.event_listener::<EventResponseReceived>().await?;

    page.goto(url).await?;

    let hard_timeout = std::time::Duration::from_secs(timeout_seconds);
    let idle_timeout = std::time::Duration::from_millis(2000);
    let start_time = tokio::time::Instant::now();

    let req_patterns = request_patterns;
    let resp_patterns = response_patterns;

    let mut captured_requests: Vec<InterceptedRequest> = Vec::new();
    let mut matching_responses: Vec<(
        chromiumoxide::cdp::browser_protocol::network::RequestId,
        String,
        u16,
        HashMap<String, String>,
    )> = Vec::new();

    let hard_sleep = tokio::time::sleep(hard_timeout);
    tokio::pin!(hard_sleep);

    let idle_sleep = tokio::time::sleep(hard_timeout);
    tokio::pin!(idle_sleep);
    let mut idle_armed = false;

    loop {
        tokio::select! {
            _ = &mut hard_sleep => {
                tracing::info!("intercept_all: hard timeout ({timeout_seconds}s)");
                break;
            }
            _ = &mut idle_sleep, if idle_armed => {
                tracing::info!("intercept_all: network idle (2s)");
                break;
            }
            req = request_events.next() => {
                if let Some(event) = req {
                    let req_url = &event.request.url;
                    if req_patterns.iter().any(|&p| req_url.contains(p)) {
                        captured_requests.push(build_intercepted_request(&event));

                        idle_armed = true;
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                    }
                } else {
                    break;
                }
            }
            resp = response_events.next() => {
                if let Some(event) = resp {
                    if resp_patterns.iter().any(|&p| event.response.url.contains(p)) {
                        let status = event.response.status.try_into().unwrap_or(0u16);
                        let headers = extract_response_headers(&event);

                        tracing::info!(
                            "response intercepted: {} (status: {})",
                            event.response.url,
                            status,
                        );

                        matching_responses.push((
                            event.request_id.clone(),
                            event.response.url.clone(),
                            status,
                            headers,
                        ));

                        idle_armed = true;
                        idle_sleep.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                    }
                } else {
                    break;
                }
            }
        }
    }

    let response_results = fetch_response_bodies(&page, matching_responses).await;

    tracing::info!(
        "intercept_all: {} requests, {} responses (duration: {:.2}s)",
        captured_requests.len(),
        response_results.len(),
        start_time.elapsed().as_secs_f64()
    );

    close_page_quietly(page, "intercept_all").await;

    Ok(InterceptedPageData {
        requests: captured_requests,
        responses: response_results,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn build_intercepted_request(event: &EventRequestWillBeSent) -> InterceptedRequest {
    let post_body = event
        .request
        .post_data_entries
        .as_ref()
        .and_then(|entries| {
            use base64::Engine;
            let mut body = String::new();
            for entry in entries {
                if let Some(ref bytes) = entry.bytes {
                    let b64: &str = bytes.as_ref();
                    match base64::engine::general_purpose::STANDARD.decode(b64) {
                        Ok(decoded) => {
                            if let Ok(s) = std::str::from_utf8(&decoded) {
                                body.push_str(s);
                            } else {
                                body.push_str(b64);
                            }
                        }
                        Err(_) => body.push_str(b64),
                    }
                }
            }
            if body.is_empty() {
                None
            } else {
                Some(body)
            }
        });

    let headers: HashMap<String, String> = event
        .request
        .headers
        .inner()
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    tracing::info!(
        "request intercepted: {} {} (body: {} bytes)",
        event.request.method,
        event.request.url,
        post_body.as_ref().map_or(0, |b| b.len()),
    );

    InterceptedRequest {
        url: event.request.url.clone(),
        method: event.request.method.clone(),
        post_body,
        headers,
        resource_type: event.r#type.as_ref().map(|t| format!("{:?}", t)),
    }
}

fn extract_response_headers(event: &EventResponseReceived) -> HashMap<String, String> {
    event
        .response
        .headers
        .inner()
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

async fn fetch_response_bodies(
    page: &Page,
    matching: Vec<(
        chromiumoxide::cdp::browser_protocol::network::RequestId,
        String,
        u16,
        HashMap<String, String>,
    )>,
) -> Vec<InterceptedResponse> {
    let mut results = Vec::new();
    for (request_id, url, status, headers) in matching {
        let params = GetResponseBodyParams::new(request_id);
        match page.execute(params).await {
            Ok(body_result) => {
                results.push(InterceptedResponse {
                    url,
                    status,
                    body: body_result.result.body,
                    base64_encoded: body_result.result.base64_encoded,
                    headers,
                });
            }
            Err(e) => {
                tracing::warn!("failed to get body of {}: {:?}", url, e);
            }
        }
    }
    results
}

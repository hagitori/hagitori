use std::collections::HashMap;
use std::time::{Duration, Instant};

use chromiumoxide::Page;

use crate::error::BrowserError;

const CLOUDFLARE_CHALLENGE_TITLES: &[&str] = &[
    "Just a moment",
    "Checking your browser",
    "Attention Required",
    "Please Wait",
    "Um momento",
];

const CLOUDFLARE_TIMEOUT: Duration = Duration::from_secs(90);
const TITLE_POLL_INTERVAL: Duration = Duration::from_millis(500);

// iframe polling: wait at least 1s for page render, then check every 300ms
const IFRAME_MIN_WAIT: Duration = Duration::from_millis(1000);
const IFRAME_POLL_INTERVAL: Duration = Duration::from_millis(300);

// after clicking, poll title every 500ms for up to 5s before retrying
const POST_CLICK_POLL: Duration = Duration::from_millis(500);
const POST_CLICK_MAX_WAIT: Duration = Duration::from_secs(5);

// poll for cf_clearance cookie every 200ms, max 2s
const COOKIE_POLL_INTERVAL: Duration = Duration::from_millis(200);
const COOKIE_POLL_MAX: Duration = Duration::from_millis(2000);

const MAX_CLICK_ATTEMPTS: u32 = 8;

// checkbox position inside the turnstile iframe (ratios relative to iframe size)
const TURNSTILE_CHECKBOX_X_RATIO: f64 = 0.093;
const TURNSTILE_CHECKBOX_Y_RATIO: f64 = 0.43;

#[derive(Debug, Clone, Default)]
pub struct CloudflareBypassOptions {
    pub auto_click: bool,
}

#[derive(Debug, Clone)]
pub struct CloudflareBypassResult {
    pub cookies: HashMap<String, String>,
    pub user_agent: String,
}

impl CloudflareBypassResult {
    pub fn has_cf_clearance(&self) -> bool {
        self.cookies.contains_key("cf_clearance")
    }

    pub fn cookies_as_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

pub fn is_cloudflare_challenge(title: &str) -> bool {
    CLOUDFLARE_CHALLENGE_TITLES
        .iter()
        .any(|&challenge| title.contains(challenge))
}

pub async fn solve_cloudflare_if_present(page: &Page) -> Result<bool, BrowserError> {
    let title = page.get_title().await?.unwrap_or_default();
    if !is_cloudflare_challenge(&title) {
        return Ok(false);
    }

    tracing::info!("cloudflare challenge detected in-place   title: {}", title);
    let start = Instant::now();
    tokio::time::sleep(IFRAME_MIN_WAIT).await;

    for attempt in 1..=MAX_CLICK_ATTEMPTS {
        if start.elapsed() > CLOUDFLARE_TIMEOUT {
            return Err(BrowserError::CloudflareTimeout(
                "cloudflare in-place solve timed out".into(),
            ));
        }

        // poll for turnstile iframe
        let clicked = loop {
            if start.elapsed() > CLOUDFLARE_TIMEOUT {
                break false;
            }
            match try_click_turnstile_realistic(page).await {
                Ok(true) => {
                    tracing::info!("turnstile click #{} sent (elapsed={}s)", attempt, start.elapsed().as_secs());
                    break true;
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::debug!("turnstile search error: {:?}", e);
                }
            }
            tokio::time::sleep(IFRAME_POLL_INTERVAL).await;
        };

        if !clicked {
            continue;
        }

        // fast-poll title for resolution
        let click_time = Instant::now();
        while click_time.elapsed() < POST_CLICK_MAX_WAIT && start.elapsed() < CLOUDFLARE_TIMEOUT {
            tokio::time::sleep(POST_CLICK_POLL).await;
            let title = page.get_title().await?.unwrap_or_default();
            if !title.is_empty() && !is_cloudflare_challenge(&title) {
                tracing::info!(
                    "cloudflare solved in-place! title: {} (took {:?})",
                    title, start.elapsed()
                );
                return Ok(true);
            }
        }
    }

    Err(BrowserError::CloudflareTimeout(
        "cloudflare in-place solve: max click attempts reached".into(),
    ))
}

pub async fn bypass_cloudflare(
    page: &Page,
    url: &str,
    options: &CloudflareBypassOptions,
) -> Result<CloudflareBypassResult, BrowserError> {
    tracing::info!(
        "starting cloudflare bypass for: {} (auto_click={})",
        url, options.auto_click
    );

    page.goto(url).await?;

    let start = Instant::now();
    let mut challenge_detected = false;
    let mut click_count: u32 = 0;

    loop {
        if start.elapsed() > CLOUDFLARE_TIMEOUT {
            if challenge_detected {
                return Err(BrowserError::CloudflareTimeout(format!(
                    "cloudflare bypass timeout after {:?}   challenge was not solved. {}",
                    CLOUDFLARE_TIMEOUT,
                    if options.auto_click {
                        "auto-click was active but couldn't solve it."
                    } else {
                        "hint: use autoClick: true to automatically click the Turnstile."
                    }
                )));
            }

            tracing::info!("no cloudflare challenge detected, continuing...");
            break;
        }

        let title = page.get_title().await?.unwrap_or_default();

        if is_cloudflare_challenge(&title) {
            if !challenge_detected {
                challenge_detected = true;
                click_count = 0;
                tracing::info!("cloudflare challenge detected   title: {}", title);
            }

            if options.auto_click && click_count < MAX_CLICK_ATTEMPTS {
                // poll for turnstile iframe
                let iframe_search_start = Instant::now();
                tokio::time::sleep(IFRAME_MIN_WAIT).await;

                let clicked = loop {
                    if start.elapsed() > CLOUDFLARE_TIMEOUT {
                        break false;
                    }

                    match try_click_turnstile_realistic(page).await {
                        Ok(true) => {
                            click_count += 1;
                            tracing::info!(
                                "turnstile click #{} sent (elapsed={}s, iframe found in {:.1}s)",
                                click_count,
                                start.elapsed().as_secs(),
                                iframe_search_start.elapsed().as_secs_f64()
                            );
                            break true;
                        }
                        Ok(false) => {
                            tracing::debug!(
                                "turnstile iframe not found yet (elapsed={}s)",
                                start.elapsed().as_secs()
                            );
                        }
                        Err(e) => {
                            tracing::debug!("error searching for turnstile: {:?}", e);
                        }
                    }

                    tokio::time::sleep(IFRAME_POLL_INTERVAL).await;
                };

                if clicked {
                    // fast-poll title for resolution
                    let click_time = Instant::now();
                    let mut solved = false;

                    while click_time.elapsed() < POST_CLICK_MAX_WAIT
                        && start.elapsed() < CLOUDFLARE_TIMEOUT
                    {
                        tokio::time::sleep(POST_CLICK_POLL).await;

                        let title = page.get_title().await?.unwrap_or_default();
                        if !title.is_empty() && !is_cloudflare_challenge(&title) {
                            tracing::info!(
                                "cloudflare challenge solved! title: {} (took {:?})",
                                title,
                                start.elapsed()
                            );
                            solved = true;
                            break;
                        }
                    }

                    if solved {
                        return extract_cookies_fast(page).await;
                    }
                    // not solved yet retry click in next outer loop iteration
                    continue;
                }
            }

            tokio::time::sleep(TITLE_POLL_INTERVAL).await;
            continue;
        }

        if title.is_empty() {
            tokio::time::sleep(TITLE_POLL_INTERVAL).await;
            continue;
        }

        if challenge_detected {
            tracing::info!(
                "cloudflare challenge solved! title: {} (took {:?})",
                title,
                start.elapsed()
            );
        } else {
            tracing::info!("page loaded without challenge. title: {}", title);
        }
        break;
    }

    extract_cookies_fast(page).await
}

// poll for cf_clearance cookie
async fn extract_cookies_fast(page: &Page) -> Result<CloudflareBypassResult, BrowserError> {
    let start = Instant::now();

    loop {
        let browser_cookies = page.get_cookies().await?;
        let cookies: HashMap<String, String> = browser_cookies
            .into_iter()
            .map(|c| (c.name, c.value))
            .collect();

        if cookies.contains_key("cf_clearance") {
            tracing::info!(
                "cf_clearance found: {} cookies extracted (took {:.1}s)",
                cookies.len(),
                start.elapsed().as_secs_f64()
            );

            let user_agent = page.user_agent().await?;
            return Ok(CloudflareBypassResult {
                cookies,
                user_agent,
            });
        }

        if start.elapsed() > COOKIE_POLL_MAX {
            return Err(BrowserError::CloudflareTimeout(
                "cf_clearance cookie not found after bypass".into(),
            ));
        }

        tokio::time::sleep(COOKIE_POLL_INTERVAL).await;
    }
}

/// realistic mouse click simulation using raw CDP Input.dispatchMouseEvent.
/// simulates human-like behavior: move -> hover -> pause -> mouseDown -> short pause -> mouseUp.
async fn try_click_turnstile_realistic(page: &Page) -> Result<bool, BrowserError> {
    use chromiumoxide::cdp::browser_protocol::dom::{
        GetDocumentParams, GetBoxModelParams, Node, NodeId,
    };
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
    };

    let doc = page.execute(
        GetDocumentParams::builder()
            .depth(-1)
            .pierce(true)
            .build()
    ).await?;

    fn find_turnstile_iframe(node: &Node) -> Option<NodeId> {
        if node.node_name == "IFRAME" {
            let src = node.attributes.as_ref().and_then(|attrs| {
                let pos = attrs.iter().position(|a| a == "src")?;
                attrs.get(pos + 1).cloned()
            });
            if let Some(ref s) = src
                && (s.contains("challenges.cloudflare.com") || s.contains("turnstile"))
            {
                tracing::info!("turnstile iframe found: src={}", s);
                return Some(node.node_id);
            }
        }

        if let Some(shadows) = &node.shadow_roots {
            for shadow in shadows {
                if let Some(id) = find_turnstile_iframe(shadow) { return Some(id); }
            }
        }
        if let Some(children) = &node.children {
            for child in children {
                if let Some(id) = find_turnstile_iframe(child) { return Some(id); }
            }
        }
        if let Some(content) = &node.content_document
            && let Some(id) = find_turnstile_iframe(content)
        {
            return Some(id);
        }

        None
    }

    let node_id = match find_turnstile_iframe(&doc.root) {
        Some(id) => id,
        None => return Ok(false),
    };

    match page.execute(
        GetBoxModelParams::builder()
            .node_id(node_id)
            .build()
    ).await {
        Ok(box_result) => {
            let quad = &box_result.model.border;
            let x = quad.inner()[0];
            let y = quad.inner()[1];
            let x2 = quad.inner()[2];
            let y2 = quad.inner()[5];
            let w = x2 - x;
            let h = y2 - y;

            let click_x = x + w * TURNSTILE_CHECKBOX_X_RATIO;
            let click_y = y + h * TURNSTILE_CHECKBOX_Y_RATIO;

            // add small random offset to avoid exact same coordinates each time
            let jitter_x = (rand_jitter() * 3.0) - 1.5;
            let jitter_y = (rand_jitter() * 3.0) - 1.5;
            let final_x = click_x + jitter_x;
            let final_y = click_y + jitter_y;

            tracing::info!(
                "clicking turnstile at ({:.1}, {:.1}) [iframe {:.0}x{:.0} at ({:.0}, {:.0})]",
                final_x, final_y, w, h, x, y
            );

            // move mouse to a random starting point
            let start_x = final_x + 50.0 + (rand_jitter() * 30.0);
            let start_y = final_y - 20.0 + (rand_jitter() * 15.0);
            dispatch_mouse_move(page, start_x, start_y).await?;
            random_sleep(50, 120).await;

            // move mouse to intermediate point
            let mid_x = (start_x + final_x) / 2.0 + (rand_jitter() * 10.0);
            let mid_y = (start_y + final_y) / 2.0 + (rand_jitter() * 5.0);
            dispatch_mouse_move(page, mid_x, mid_y).await?;
            random_sleep(30, 80).await;

            // move mouse to target
            dispatch_mouse_move(page, final_x, final_y).await?;
            random_sleep(80, 200).await;

            // mouseDown
            page.execute(DispatchMouseEventParams {
                r#type: DispatchMouseEventType::MousePressed,
                x: final_x,
                y: final_y,
                modifiers: None,
                timestamp: None,
                button: Some(MouseButton::Left),
                buttons: None,
                click_count: Some(1),
                force: None,
                tangential_pressure: None,
                tilt_x: None,
                tilt_y: None,
                twist: None,
                delta_x: None,
                delta_y: None,
                pointer_type: Some(
                    chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventPointerType::Mouse
                ),
            }).await?;
            
            // short pause between press and release
            random_sleep(50, 150).await;

            // mouseUp
            page.execute(DispatchMouseEventParams {
                r#type: DispatchMouseEventType::MouseReleased,
                x: final_x,
                y: final_y,
                modifiers: None,
                timestamp: None,
                button: Some(MouseButton::Left),
                buttons: None,
                click_count: Some(1),
                force: None,
                tangential_pressure: None,
                tilt_x: None,
                tilt_y: None,
                twist: None,
                delta_x: None,
                delta_y: None,
                pointer_type: Some(
                    chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventPointerType::Mouse
                ),
            }).await?;

            tracing::info!("realistic click sequence completed at ({:.1}, {:.1})", final_x, final_y);
            Ok(true)
        }
        Err(e) => {
            tracing::warn!("failed to get box model of turnstile iframe: {:?}", e);
            Ok(false)
        }
    }
}

/// helper to dispatch a mouse move event via CDP.
async fn dispatch_mouse_move(page: &Page, x: f64, y: f64) -> Result<(), BrowserError> {
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType,
    };
    page.execute(DispatchMouseEventParams {
        r#type: DispatchMouseEventType::MouseMoved,
        x,
        y,
        modifiers: None,
        timestamp: None,
        button: None,
        buttons: None,
        click_count: None,
        force: None,
        tangential_pressure: None,
        tilt_x: None,
        tilt_y: None,
        twist: None,
        delta_x: None,
        delta_y: None,
        pointer_type: Some(
            chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventPointerType::Mouse
        ),
    }).await?;
    Ok(())
}

/// simple pseudo-random jitter (0.0 to 1.0) using system time nanos
fn rand_jitter() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

/// sleep for a random duration between min_ms and max_ms
async fn random_sleep(min_ms: u64, max_ms: u64) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let range = max_ms - min_ms;
    let sleep_ms = min_ms + (nanos % range);
    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
}

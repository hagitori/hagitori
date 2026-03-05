pub mod browser;
pub mod cookies;
pub mod crypto;
pub mod date;
pub mod entities;
pub mod html;
pub mod http;
pub mod session;
pub mod utils;

use std::sync::Arc;

use rquickjs::Ctx;

use crate::runtime::RuntimeData;

/// registers all native APIs in the QuickJS context.
/// `features` controls which conditional APIs are enabled.
pub fn register_all<'js>(
    ctx: &Ctx<'js>,
    data: &Arc<RuntimeData>,
    requires_browser: bool,
    requires_crypto: bool,
    allowed_domains: Arc<Vec<String>>,
) -> std::result::Result<(), String> {
    // always-available APIs
    utils::register(ctx).map_err(|e| format!("utils: {e}"))?;
    entities::register(ctx).map_err(|e| format!("entities: {e}"))?;
    http::register(ctx, data.clone(), allowed_domains).map_err(|e| format!("http: {e}"))?;
    html::register(ctx).map_err(|e| format!("html: {e}"))?;
    cookies::register(ctx, data.clone()).map_err(|e| format!("cookies: {e}"))?;
    session::register(ctx, data.clone()).map_err(|e| format!("session: {e}"))?;
    date::register(ctx).map_err(|e| format!("date: {e}"))?;

    // conditional APIs
    if requires_browser {
        browser::register(ctx, data.clone()).map_err(|e| format!("browser: {e}"))?;
    } else {
        browser::register_stub(ctx).map_err(|e| format!("browser stub: {e}"))?;
    }

    if requires_crypto {
        crypto::register(ctx).map_err(|e| format!("crypto: {e}"))?;
    } else {
        crypto::register_stub(ctx).map_err(|e| format!("crypto stub: {e}"))?;
    }

    Ok(())
}

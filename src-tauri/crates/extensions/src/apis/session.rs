//! QuickJS session and headers management API for extensions.

#![expect(clippy::needless_pass_by_value, reason = "rquickjs FromJs requires String, not &str")]

use std::collections::HashMap;
use std::sync::Arc;

use rquickjs::{Ctx, Function, Object};

use crate::runtime::RuntimeData;

pub fn register<'js>(ctx: &Ctx<'js>, data: Arc<RuntimeData>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    let session_obj = Object::new(ctx.clone())?;

    // session.setHeaders(domain, headers)
    let data_headers = data.clone();
    session_obj.set(
        "setHeaders",
        Function::new(ctx.clone(), move |domain: String, headers: Object<'js>| -> rquickjs::Result<()> {
            let mut map = HashMap::new();
            for key in headers.keys::<String>() {
                let key = key?;
                let val: rquickjs::Coerced<String> = headers.get(&key)?;
                map.insert(key, val.0);
            }
            data_headers.http_client.session_store().set_headers(&domain, map);
            Ok(())
        })?,
    )?;

    // session.setUserAgent(domain, ua)
    let data_ua = data.clone();
    session_obj.set(
        "setUserAgent",
        Function::new(ctx.clone(), move |domain: String, ua: String| {
            data_ua.http_client.session_store().set_user_agent(&domain, &ua);
        })?,
    )?;

    globals.set("session", session_obj)?;
    Ok(())
}

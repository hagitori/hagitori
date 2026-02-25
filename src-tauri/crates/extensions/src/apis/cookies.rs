//! QuickJS cookie management API for extensions.

#![expect(clippy::needless_pass_by_value, reason = "rquickjs FromJs requires String, not &str")]

use std::sync::Arc;

use rquickjs::{Ctx, Function, Object};

use crate::runtime::RuntimeData;

pub fn register<'js>(ctx: &Ctx<'js>, data: Arc<RuntimeData>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    let cookies_obj = Object::new(ctx.clone())?;

    // cookies.set(domain, cookies_obj)   sets cookies for a domain
    let data_set = data.clone();
    cookies_obj.set(
        "set",
        Function::new(ctx.clone(), move |domain: String, cookies: Object<'js>| {
            for key in cookies.keys::<String>() {
                let key = key?;
                let val: rquickjs::prelude::Coerced<String> = cookies.get(&key)?;
                data_set.http_client.session_store().set_cookie(&domain, &key, &val.0);
            }
            Ok::<_, rquickjs::Error>(())
        })?,
    )?;

    // cookies.get(domain) -> { name: value, ... }
    let data_get = data.clone();
    cookies_obj.set(
        "get",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, domain: String| {
            let cookies = data_get.http_client.session_store().get_cookies(&domain);
            let obj = Object::new(ctx)?;
            for (k, v) in cookies {
                obj.set(k.as_str(), v)?;
            }
            Ok::<_, rquickjs::Error>(obj)
        })?,
    )?;

    // cookies.remove(domain, name)
    let data_remove = data.clone();
    cookies_obj.set(
        "remove",
        Function::new(ctx.clone(), move |domain: String, name: String| {
            data_remove.http_client.session_store().remove_cookie(&domain, &name);
        })?,
    )?;

    // cookies.clear(domain)
    let data_clear = data.clone();
    cookies_obj.set(
        "clear",
        Function::new(ctx.clone(), move |domain: String| {
            data_clear.http_client.session_store().clear_cookies(&domain);
        })?,
    )?;

    globals.set("cookies", cookies_obj)?;
    Ok(())
}

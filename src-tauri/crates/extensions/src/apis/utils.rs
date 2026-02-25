//! Utility API registration (console, globals, timers) for the QuickJS runtime.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rquickjs::{Ctx, Function, Object, Value, Array};
use rquickjs::prelude::{This, Rest, Coerced, Opt, Async};

/// registers console, globals (`atob`, `btoa`, `setTimeout`, etc.) into the JS context.
pub fn register<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    register_console(ctx)?;
    register_globals(ctx)?;
    Ok(())
}

/// joins console arguments into a single space-separated string without
/// allocating an intermediate `Vec`.
fn format_console_args(args: &Rest<Coerced<String>>) -> String {
    args.0.iter().enumerate().fold(String::new(), |mut acc, (i, s)| {
        if i > 0 { acc.push(' '); }
        acc.push_str(s.0.as_str());
        acc
    })
}

fn register_console<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;

    console.set(
        "log",
        Function::new(ctx.clone(), |args: Rest<Coerced<String>>| {
            let msg = format_console_args(&args);
            tracing::info!(target: "extension", "{msg}");
        })?,
    )?;

    console.set(
        "warn",
        Function::new(ctx.clone(), |args: Rest<Coerced<String>>| {
            let msg = format_console_args(&args);
            tracing::warn!(target: "extension", "{msg}");
        })?,
    )?;

    console.set(
        "error",
        Function::new(ctx.clone(), |args: Rest<Coerced<String>>| {
            let msg = format_console_args(&args);
            tracing::error!(target: "extension", "{msg}");
        })?,
    )?;

    globals.set("console", console)?;
    Ok(())
}

fn register_globals<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    // atob(base64String) -> decodedString
    globals.set(
        "atob",
        Function::new(ctx.clone(), |input: String| -> rquickjs::Result<String> {
            let decoded = BASE64
                .decode(input.as_bytes())
                .map_err(|e| rquickjs::Error::new_from_js_message("string", "string", &format!("atob: invalid base64 input: {e}")))?;
            String::from_utf8(decoded)
                .map_err(|e| rquickjs::Error::new_from_js_message("string", "string", &format!("atob: invalid UTF-8: {e}")))
        })?,
    )?;

    // btoa(string) -> base64String
    globals.set(
        "btoa",
        Function::new(ctx.clone(), |input: String| -> String {
            BASE64.encode(input.as_bytes())
        })?,
    )?;

    // setTimeout(fn, ms) -> Promise (async uses tokio::time::sleep)
    globals.set(
        "setTimeout",
        Function::new(ctx.clone(), Async(set_timeout_impl))?,
    )?;

    // clearTimeout
    globals.set("clearTimeout", Function::new(ctx.clone(), || {})?)?;

    // clearInterval
    globals.set("clearInterval", Function::new(ctx.clone(), || {})?)?;

    // sleep(ms) -> Promise (async uses tokio::time::sleep)
    globals.set(
        "sleep",
        Function::new(ctx.clone(), Async(sleep_impl))?,
    )?;

    // URLSearchParams(init?) simplified stub 
    register_url_search_params(ctx)?;

    Ok(())
}

fn register_url_search_params<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    globals.set(
        "URLSearchParams",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, init: Opt<Value<'js>>| {
            let mut params: Vec<(String, String)> = Vec::new();

            if let Some(init_val) = init.0 {
                if let Some(s) = init_val.as_string() {
                    let s = s.to_string()?;
                    let s = s.strip_prefix('?').unwrap_or(&s);
                    for pair in s.split('&') {
                        if pair.is_empty() {
                            continue;
                        }
                        let mut parts = pair.splitn(2, '=');
                        let key = parts.next().unwrap_or("");
                        let val = parts.next().unwrap_or("");
                        params.push((
                            url::form_urlencoded::parse(key.as_bytes())
                                .next()
                                .map(|(k, _)| k.to_string())
                                .unwrap_or_default(),
                            url::form_urlencoded::parse(val.as_bytes())
                                .next()
                                .map(|(k, _)| k.to_string())
                                .unwrap_or_default(),
                        ));
                    }
                } else if let Some(obj) = init_val.as_object() {
                    for key in obj.keys::<String>() {
                        let key = key?;
                        let val: Coerced<String> = obj.get(&key)?;
                        params.push((key, val.0));
                    }
                }
            }

            let serialized = serialize_params(&params);

            let usp = Object::new(ctx.clone())?;
            usp.set("_data", serialized)?;

            // get(key) -> string | null
            usp.set("get", Function::new(ctx.clone(), |this: This<Object<'js>>, key: String| {
                let data: String = this.0.get("_data")?;
                let params = parse_usp_data(&data);
                for (k, v) in &params {
                    if k == &key {
                        let ctx = this.0.ctx().clone();
                        let s = rquickjs::String::from_str(ctx.clone(), v)?;
                        return Ok::<_, rquickjs::Error>(Value::from(s));
                    }
                }
                Ok::<_, rquickjs::Error>(Value::new_null(this.0.ctx().clone()))
            })?)?;

            // has(key) -> boolean
            usp.set("has", Function::new(ctx.clone(), |this: This<Object<'js>>, key: String| {
                let data: String = this.0.get("_data")?;
                let params = parse_usp_data(&data);
                Ok::<_, rquickjs::Error>(params.iter().any(|(k, _)| k == &key))
            })?)?;

            // set(key, value)
            usp.set("set", Function::new(ctx.clone(), |this: This<Object<'js>>, key: String, val: String| {
                let data: String = this.0.get("_data")?;
                let mut params = parse_usp_data(&data);
                params.retain(|(k, _)| k != &key);
                params.push((key, val));
                this.0.set("_data", serialize_params(&params))?;
                Ok::<_, rquickjs::Error>(())
            })?)?;

            // append(key, value)
            usp.set("append", Function::new(ctx.clone(), |this: This<Object<'js>>, key: String, val: String| {
                let data: String = this.0.get("_data")?;
                let mut params = parse_usp_data(&data);
                params.push((key, val));
                this.0.set("_data", serialize_params(&params))?;
                Ok::<_, rquickjs::Error>(())
            })?)?;

            // delete(key)
            usp.set("delete", Function::new(ctx.clone(), |this: This<Object<'js>>, key: String| {
                let data: String = this.0.get("_data")?;
                let mut params = parse_usp_data(&data);
                params.retain(|(k, _)| k != &key);
                this.0.set("_data", serialize_params(&params))?;
                Ok::<_, rquickjs::Error>(())
            })?)?;

            // toString()
            usp.set("toString", Function::new(ctx.clone(), |this: This<Object<'js>>| {
                let data: String = this.0.get("_data")?;
                Ok::<_, rquickjs::Error>(data)
            })?)?;

            // getAll(key) -> string[]
            usp.set("getAll", Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>, key: String| {
                let data: String = this.0.get("_data")?;
                let params = parse_usp_data(&data);
                let arr = Array::new(ctx.clone())?;
                let mut i = 0;
                for (k, v) in &params {
                    if k == &key {
                        arr.set(i, v.clone())?;
                        i += 1;
                    }
                }
                Ok::<_, rquickjs::Error>(arr)
            })?)?;

            // keys() -> string[]
            usp.set("keys", Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>| {
                let data: String = this.0.get("_data")?;
                let params = parse_usp_data(&data);
                let arr = Array::new(ctx.clone())?;
                for (i, (k, _)) in params.iter().enumerate() {
                    arr.set(i, k.clone())?;
                }
                Ok::<_, rquickjs::Error>(arr)
            })?)?;

            // values() -> string[]
            usp.set("values", Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>| {
                let data: String = this.0.get("_data")?;
                let params = parse_usp_data(&data);
                let arr = Array::new(ctx.clone())?;
                for (i, (_, v)) in params.iter().enumerate() {
                    arr.set(i, v.clone())?;
                }
                Ok::<_, rquickjs::Error>(arr)
            })?)?;

            // entries() -> [string, string][]
            usp.set("entries", Function::new(ctx.clone(), |ctx: Ctx<'js>, this: This<Object<'js>>| {
                let data: String = this.0.get("_data")?;
                let params = parse_usp_data(&data);
                let arr = Array::new(ctx.clone())?;
                for (i, (k, v)) in params.iter().enumerate() {
                    let pair = Array::new(ctx.clone())?;
                    pair.set(0, k.clone())?;
                    pair.set(1, v.clone())?;
                    arr.set(i, pair)?;
                }
                Ok::<_, rquickjs::Error>(arr)
            })?)?;

            Ok::<_, rquickjs::Error>(usp)
        })?,
    )?;

    Ok(())
}

fn parse_usp_data(data: &str) -> Vec<(String, String)> {
    if data.is_empty() {
        return Vec::new();
    }
    data.split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let k = parts.next()?;
            let v = parts.next().unwrap_or("");
            Some((
                url::form_urlencoded::parse(k.as_bytes())
                    .next()
                    .map(|(k, _)| k.to_string())
                    .unwrap_or_default(),
                url::form_urlencoded::parse(v.as_bytes())
                    .next()
                    .map(|(k, _)| k.to_string())
                    .unwrap_or_default(),
            ))
        })
        .collect()
}

fn serialize_params(params: &[(String, String)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params.iter())
        .finish()
}

// ─── Named async functions (avoids lifetime issues in closures with edition 2024) ───

async fn set_timeout_impl<'js>(callback: Function<'js>, ms: Opt<u32>) -> rquickjs::Result<i32> {
    let ms = ms.0.unwrap_or(0);
    if ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
    callback.call::<_, Value>(())?;
    Ok(0)
}

async fn sleep_impl(ms: u32) -> rquickjs::Result<()> {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    Ok(())
}

//! Utility API registration (console, globals, timers) for the QuickJS runtime.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI32, Ordering},
};
use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rquickjs::{Ctx, Function, Object, Value, Array};
use rquickjs::prelude::{This, Rest, Coerced, Opt, Async};
use tokio::sync::Mutex;

static NEXT_TIMER_ID: AtomicI32 = AtomicI32::new(1);

// shared map of interval cancellation flags
static INTERVAL_CANCELS: std::sync::LazyLock<Mutex<HashMap<i32, Arc<AtomicBool>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

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

    // clearTimeout: no-op (QuickJS runs single-threaded therefore timer cancellation not supported)
    globals.set("clearTimeout", Function::new(ctx.clone(), |_id: Opt<i32>| {})?)?;

    // sleep(ms) -> Promise (async uses tokio::time::sleep)
    globals.set(
        "sleep",
        Function::new(ctx.clone(), Async(sleep_impl))?,
    )?;

    // setInterval(fn, ms) -> id
    globals.set(
        "setInterval",
        Function::new(ctx.clone(), Async(set_interval_impl))?,
    )?;

    // clearInterval(id)
    globals.set(
        "clearInterval",
        Function::new(ctx.clone(), Async(clear_interval_impl))?,
    )?;

    // URLSearchParams(init?) simplified stub 
    register_url_search_params(ctx)?;

    // AbortController / AbortSignal
    register_abort_controller(ctx)?;

    // TextEncoder / TextDecoder
    register_text_encoder_decoder(ctx)?;

    // URL class
    register_url_class(ctx)?;

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
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed);
    let ms = ms.0.unwrap_or(0);
    if ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    }
    callback.call::<_, Value>(())?;
    Ok(id)
}

async fn sleep_impl(ms: u32) -> rquickjs::Result<()> {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    Ok(())
}

// spawns a recurring timer, returns an id
async fn set_interval_impl<'js>(callback: Function<'js>, ms: Opt<u32>) -> rquickjs::Result<i32> {
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed);
    let ms = ms.0.unwrap_or(0).max(1) as u64;
    let cancel = Arc::new(AtomicBool::new(false));
    INTERVAL_CANCELS.lock().await.insert(id, cancel.clone());

    let ctx = callback.ctx().clone();
    let cb = callback.clone();

    // schedule the recurring execution as a deferred job
    ctx.spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            if cb.call::<_, Value>(()).is_err() {
                break;
            }
        }
    });

    Ok(id)
}

async fn clear_interval_impl(id: i32) -> rquickjs::Result<()> {
    if let Some(cancel) = INTERVAL_CANCELS.lock().await.remove(&id) {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

// ─── AbortController / AbortSignal ──────────────────────────

fn register_abort_controller<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    globals.set(
        "AbortController",
        Function::new(ctx.clone(), |ctx: Ctx<'js>| -> rquickjs::Result<Object<'js>> {
            let signal = Object::new(ctx.clone())?;
            signal.set("aborted", false)?;
            signal.set("reason", Value::new_undefined(ctx.clone()))?;

            let controller = Object::new(ctx.clone())?;
            controller.set("signal", signal)?;

            let ctrl_ref = controller.clone();
            controller.set(
                "abort",
                Function::new(ctx.clone(), move |ctx: Ctx<'js>, reason: Opt<Value<'js>>| -> rquickjs::Result<()> {
                    let signal: Object = ctrl_ref.get("signal")?;
                    signal.set("aborted", true)?;
                    let reason_val = reason.0.unwrap_or_else(|| {
                        rquickjs::String::from_str(ctx.clone(), "AbortError")
                            .map(Value::from)
                            .unwrap_or_else(|_| Value::new_undefined(ctx.clone()))
                    });
                    signal.set("reason", reason_val)?;
                    Ok(())
                })?,
            )?;

            Ok(controller)
        })?,
    )?;

    Ok(())
}

// ─── TextEncoder / TextDecoder ───────────────────────────────

fn register_text_encoder_decoder<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    // TextEncoder: encode(string) -> Array<number>
    globals.set(
        "TextEncoder",
        Function::new(ctx.clone(), |ctx: Ctx<'js>| -> rquickjs::Result<Object<'js>> {
            let encoder = Object::new(ctx.clone())?;
            encoder.set("encoding", "utf-8")?;
            encoder.set(
                "encode",
                Function::new(ctx.clone(), |ctx: Ctx<'js>, input: Opt<String>| -> rquickjs::Result<Array<'js>> {
                    let s = input.0.unwrap_or_default();
                    let bytes = s.as_bytes();
                    let arr = Array::new(ctx.clone())?;
                    for (i, &b) in bytes.iter().enumerate() {
                        arr.set(i, b as u32)?;
                    }
                    Ok(arr)
                })?,
            )?;
            Ok(encoder)
        })?,
    )?;

    // TextDecoder: decode(array) -> string
    globals.set(
        "TextDecoder",
        Function::new(ctx.clone(), |ctx: Ctx<'js>, _encoding: Opt<String>| -> rquickjs::Result<Object<'js>> {
            let decoder = Object::new(ctx.clone())?;
            decoder.set("encoding", "utf-8")?;
            decoder.set(
                "decode",
                Function::new(ctx.clone(), |_ctx: Ctx<'js>, input: Opt<Value<'js>>| -> rquickjs::Result<String> {
                    let Some(val) = input.0 else {
                        return Ok(String::new());
                    };
                    let Some(arr) = val.as_array() else {
                        return Err(rquickjs::Error::new_from_js_message(
                            "value",
                            "array",
                            "TextDecoder.decode expects an array-like input",
                        ));
                    };
                    let mut bytes = Vec::with_capacity(arr.len());
                    for i in 0..arr.len() {
                        let b: u32 = arr.get(i)?;
                        bytes.push(b as u8);
                    }
                    String::from_utf8(bytes).map_err(|e| {
                        rquickjs::Error::new_from_js_message("bytes", "string", &format!("TextDecoder: invalid UTF-8: {e}"))
                    })
                })?,
            )?;
            Ok(decoder)
        })?,
    )?;

    Ok(())
}

// ─── URL class ───────────────────────────────────────────────

fn register_url_class<'js>(ctx: &Ctx<'js>) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    let globals_inner = globals.clone();

    globals.set(
        "URL",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, input: String, base: Opt<String>| -> rquickjs::Result<Object<'js>> {
            let parsed = if let Some(base_str) = base.0 {
                let base_url = url::Url::parse(&base_str).map_err(|e| {
                    rquickjs::Error::new_from_js_message("string", "URL", &format!("Invalid base URL: {e}"))
                })?;
                base_url.join(&input).map_err(|e| {
                    rquickjs::Error::new_from_js_message("string", "URL", &format!("Invalid URL: {e}"))
                })?
            } else {
                url::Url::parse(&input).map_err(|e| {
                    rquickjs::Error::new_from_js_message("string", "URL", &format!("Invalid URL: {e}"))
                })?
            };

            let obj = Object::new(ctx.clone())?;
            obj.set("href", parsed.as_str())?;
            obj.set("protocol", parsed.scheme().to_string() + ":")?;
            obj.set("hostname", parsed.host_str().unwrap_or(""))?;
            obj.set("port", parsed.port().map(|p| p.to_string()).unwrap_or_default())?;
            obj.set(
                "host",
                match parsed.port() {
                    Some(p) => format!("{}:{}", parsed.host_str().unwrap_or(""), p),
                    None => parsed.host_str().unwrap_or("").to_string(),
                },
            )?;
            obj.set("pathname", parsed.path())?;
            obj.set("search", if parsed.query().is_some() { format!("?{}", parsed.query().unwrap_or("")) } else { String::new() })?;
            obj.set("hash", if parsed.fragment().is_some() { format!("#{}", parsed.fragment().unwrap_or("")) } else { String::new() })?;
            obj.set("origin", parsed.origin().ascii_serialization())?;
            obj.set("username", parsed.username())?;
            obj.set("password", parsed.password().unwrap_or(""))?;

            // searchParams as URLSearchParams-like object
            let search_str = parsed.query().unwrap_or("").to_string();
            let usp_ctor: Function = globals_inner.get("URLSearchParams")?;
            let search_params: Value = usp_ctor.call((search_str,))?;
            obj.set("searchParams", search_params)?;

            // toString() -> href
            let obj_ref = obj.clone();
            obj.set(
                "toString",
                Function::new(ctx.clone(), move || -> rquickjs::Result<String> {
                    let href: String = obj_ref.get("href")?;
                    Ok(href)
                })?,
            )?;

            Ok(obj)
        })?,
    )?;

    Ok(())
}

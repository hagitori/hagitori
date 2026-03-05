//! async QuickJS runtime with worker pool for extension script execution.

use std::sync::Arc;

use rquickjs::{
    Array, AsyncContext, AsyncRuntime, CatchResultExt, Class, Ctx, Function, Object, Value,
    promise::MaybePromise,
};

use crate::apis::entities::{JsChapter, JsManga, JsPages};

use tokio::sync::{mpsc, oneshot, Mutex};

use hagitori_core::error::{HagitoriError, Result};
use hagitori_http::HttpClient;

use crate::apis;

// ---------------------------------------------------------------------------
// RuntimeData shared state passed to every JS API
// ---------------------------------------------------------------------------

pub struct RuntimeData {
    pub http_client: Arc<HttpClient>,
    pub browser_manager: Arc<Mutex<Option<Arc<hagitori_browser::BrowserManager>>>>,
}

pub struct JsRuntime {
    data: Arc<RuntimeData>,
}

impl JsRuntime {
    pub fn new(http_client: Arc<HttpClient>) -> Self {
        Self {
            data: Arc::new(RuntimeData {
                http_client,
                browser_manager: Arc::new(Mutex::new(None)),
            }),
        }
    }

    pub fn with_shared_browser_manager(
        http_client: Arc<HttpClient>,
        browser_manager: Arc<Mutex<Option<Arc<hagitori_browser::BrowserManager>>>>,
    ) -> Self {
        Self {
            data: Arc::new(RuntimeData {
                http_client,
                browser_manager,
            }),
        }
    }

    pub fn browser_manager(&self) -> Arc<Mutex<Option<Arc<hagitori_browser::BrowserManager>>>> {
        self.data.browser_manager.clone()
    }

    pub fn http_client(&self) -> &Arc<HttpClient> {
        &self.data.http_client
    }

    pub(crate) fn to_runtime_data(&self) -> Arc<RuntimeData> {
        self.data.clone()
    }
}

// ─── Worker Commands ────────────────────────────────────────────────

enum WorkerCmd {
    Call {
        method: String,
        args: Vec<serde_json::Value>,
        reply: oneshot::Sender<Result<serde_json::Value>>,
    },
}

// ─── JsWorker: tokio task with persistent AsyncContext ───────────────

pub(crate) struct JsWorker {
    tx: mpsc::UnboundedSender<WorkerCmd>,
}

impl JsWorker {
    pub async fn spawn(
        script: Arc<String>,
        runtime_data: Arc<RuntimeData>,
        requires_browser: bool,
        requires_crypto: bool,
    ) -> Result<Self> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (init_tx, init_rx) = oneshot::channel();

        tokio::spawn(async move {
            // create async QuickJS runtime and context
            let rt = match AsyncRuntime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = init_tx.send(Err(HagitoriError::extension(format!(
                        "failed to create QuickJS AsyncRuntime: {e}"
                    ))));
                    return;
                }
            };

            let ctx = match AsyncContext::full(&rt).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    let _ = init_tx.send(Err(HagitoriError::extension(format!(
                        "failed to create QuickJS AsyncContext: {e}"
                    ))));
                    return;
                }
            };

            // register native APIs and evaluate the script
            let init_result: std::result::Result<(), String> = ctx
                .with(|ctx| {
                    // register APIs (fetch, parseHtml, Manga, etc.)
                    apis::register_all(
                        &ctx,
                        &runtime_data,
                        requires_browser,
                        requires_crypto,
                    )
                    .map_err(|e| format!("failed to register JS APIs: {e}"))?;

                    // evaluate script defines classes and instantiates __extension__
                    ctx.eval::<Value, _>(script.as_bytes())
                        .catch(&ctx)
                        .map_err(|e| format!("error evaluating extension script: {e:?}"))?;

                    Ok(())
                })
                .await;

            match init_result {
                Ok(()) => {
                    let _ = init_tx.send(Ok(()));
                }
                Err(e) => {
                    let _ = init_tx.send(Err(HagitoriError::extension(e)));
                    return;
                }
            }

            // command loop processes async calls
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    WorkerCmd::Call {
                        method,
                        args,
                        reply,
                    } => {
                        // uses async_with! to support Promise resolution
                        let result: std::result::Result<serde_json::Value, String> =
                            rquickjs::async_with!(ctx => |ctx| {
                                let call_result = call_method_raw(&ctx, &method, &args);
                                match call_result {
                                    Ok(value) => {
                                        // resolve if Promise, return directly if not
                                        let maybe = MaybePromise::from_value(value);
                                        match maybe.into_future::<Value<'_>>().await {
                                            Ok(resolved) => js_value_to_json(&resolved),
                                            Err(e) => {
                                                // extract the actual JS exception message
                                                let js_err = extract_js_exception(&ctx, &e);
                                                Err(format!("Promise rejected: {js_err}"))
                                            },
                                        }
                                    }
                                    Err(e) => Err(e),
                                }
                            }).await;

                        let result = result.map_err(HagitoriError::extension);
                        let _ = reply.send(result);
                    }
                }

                // drive pending jobs (resolve promises, timers, etc.)
                let _ = rt.idle().await;
            }
        });

        // wait for initialization result
        init_rx
            .await
            .map_err(|_| HagitoriError::extension("worker task died during initialization"))??;

        Ok(Self { tx })
    }

    pub async fn call(
        &self,
        method: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(WorkerCmd::Call {
                method: method.to_string(),
                args,
                reply: reply_tx,
            })
            .map_err(|_| HagitoriError::extension("worker task is not responding"))?;

        reply_rx
            .await
            .map_err(|_| HagitoriError::extension("worker task died during execution"))?
    }

}

// ─── Method calls in the QuickJS context ─────────────────────

/// calls `__extension__.{method}(args...)` in the JS context.
/// returns the raw Value (may be a Promise resolved by the worker loop).
fn call_method_raw<'js>(
    ctx: &Ctx<'js>,
    method: &str,
    args: &[serde_json::Value],
) -> std::result::Result<Value<'js>, String> {
    let globals = ctx.globals();

    let instance_val: Value = globals
        .get("__extension__")
        .map_err(|e| format!("failed to get __extension__: {e}"))?;

    let instance = instance_val
        .as_object()
        .ok_or_else(|| "__extension__ is not an object".to_string())?;

    let func: Function = instance
        .get(method)
        .map_err(|e| format!("method '{method}' not found in extension: {e}"))?;

    let js_args: Vec<Value> = args
        .iter()
        .map(|a| json_to_js_value(ctx, a))
        .collect::<std::result::Result<_, _>>()?;

    let mut call_args = rquickjs::function::Args::new(ctx.clone(), js_args.len());
    call_args
        .this(instance_val)
        .map_err(|e| format!("error setting this: {e}"))?;
    for arg in js_args {
        call_args
            .push_arg(arg)
            .map_err(|e| format!("error preparing argument: {e}"))?;
    }

    let result: Value = func
        .call_arg(call_args)
        .catch(ctx)
        .map_err(|e| format!("error executing '{method}': {e:?}"))?;

    Ok(result)
}

// ─── JSON <-> JS serialization ─────────────────────────────────────────

pub(crate) fn json_to_js_value<'js>(
    ctx: &Ctx<'js>,
    val: &serde_json::Value,
) -> std::result::Result<Value<'js>, String> {
    match val {
        serde_json::Value::Null => Ok(Value::new_undefined(ctx.clone())),
        serde_json::Value::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    Ok(Value::new_int(ctx.clone(), i as i32))
                } else {
                    Ok(Value::new_float(ctx.clone(), i as f64))
                }
            } else {
                Ok(Value::new_float(
                    ctx.clone(),
                    n.as_f64().ok_or_else(|| format!("number {n} cannot be represented as f64"))?,
                ))
            }
        }
        serde_json::Value::String(s) => {
            let js_str = rquickjs::String::from_str(ctx.clone(), s)
                .map_err(|e| format!("failed to create JS string: {e}"))?;
            Ok(js_str.into())
        }
        serde_json::Value::Array(arr) => {
            let js_arr =
                Array::new(ctx.clone()).map_err(|e| format!("failed to create JS array: {e}"))?;
            for (i, item) in arr.iter().enumerate() {
                let js_val = json_to_js_value(ctx, item)?;
                js_arr
                    .set(i, js_val)
                    .map_err(|e| format!("failed to insert into JS array[{i}]: {e}"))?;
            }
            Ok(js_arr.into())
        }
        serde_json::Value::Object(map) => {
            let js_obj =
                Object::new(ctx.clone()).map_err(|e| format!("failed to create JS object: {e}"))?;
            for (k, v) in map {
                let js_val = json_to_js_value(ctx, v)?;
                js_obj
                    .set(k.as_str(), js_val)
                    .map_err(|e| format!("failed to set property '{k}': {e}"))?;
            }
            Ok(js_obj.into())
        }
    }
}

pub(crate) fn js_value_to_json<'js>(
    val: &Value<'js>,
) -> std::result::Result<serde_json::Value, String> {
    if val.is_undefined() || val.is_null() {
        return Ok(serde_json::Value::Null);
    }

    if let Some(b) = val.as_bool() {
        return Ok(serde_json::Value::Bool(b));
    }

    if let Some(n) = val.as_int() {
        return Ok(serde_json::Value::Number(n.into()));
    }

    if let Some(n) = val.as_float() {
        return Ok(serde_json::json!(n));
    }

    if let Some(s) = val.as_string() {
        let s = s
            .to_string()
            .map_err(|e| format!("failed to convert JS string: {e}"))?;
        return Ok(serde_json::Value::String(s));
    }

    if let Some(arr) = val.as_array() {
        let mut result = Vec::new();
        for i in 0..arr.len() {
            let item: Value = arr
                .get(i)
                .map_err(|e| format!("failed to read index {i}: {e}"))?;
            result.push(js_value_to_json(&item)?);
        }
        return Ok(serde_json::Value::Array(result));
    }

    if let Some(obj) = val.as_object() {
        // skip functions
        if obj.is_function() {
            return Ok(serde_json::Value::Null);
        }

        // native entity classes -> serialize directly via serde
        if let Some(cls) = Class::<JsManga>::from_object(obj) {
            return serde_json::to_value(&*cls.borrow())
                .map_err(|e| format!("failed to serialize Manga: {e}"));
        }
        if let Some(cls) = Class::<JsChapter>::from_object(obj) {
            return serde_json::to_value(&*cls.borrow())
                .map_err(|e| format!("failed to serialize Chapter: {e}"));
        }
        if let Some(cls) = Class::<JsPages>::from_object(obj) {
            return serde_json::to_value(&*cls.borrow())
                .map_err(|e| format!("failed to serialize Pages: {e}"));
        }

        let mut map = serde_json::Map::new();
        for key in obj.keys::<String>() {
            let key = key.map_err(|e| format!("failed to read key: {e}"))?;
            let value: Value = obj
                .get(&key)
                .map_err(|e| format!("failed to read property '{key}': {e}"))?;

            // skip functions during serialization
            if value.is_function() {
                continue;
            }

            map.insert(key, js_value_to_json(&value)?);
        }
        return Ok(serde_json::Value::Object(map));
    }

    Err(format!("unsupported JS value type: {:?}", val.type_name()))
}

fn extract_js_exception(ctx: &Ctx<'_>, err: &rquickjs::Error) -> String {
    // try to get the pending exception from the context
    if let Some(exc) = ctx.catch().as_exception() {
        let msg = exc.message().unwrap_or_default();
        let stack = exc
            .stack()
            .unwrap_or_default();
        if stack.is_empty() {
            return msg;
        }
        return format!("{msg}\n{stack}");
    }
    // fallback Debug of the rquickjs error
    format!("{err:?}")
}

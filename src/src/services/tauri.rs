//! Low-level Tauri global IPC bindings.
//!
//! The frontend is built by `trunk` with no JS bundler, so it cannot `import`
//! the `@tauri-apps/api` npm module.  Instead we rely on Tauri's global API,
//! enabled by `app.withGlobalTauri = true` in `tauri.conf.json`, which exposes
//! `window.__TAURI__` (with `.core.invoke` and `.event.listen`).  These helpers
//! wrap the raw `web-sys` calls so the rest of the app never touches the DOM.

use js_sys::{Function, Reflect};
use serde::de::DeserializeOwned;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::window;

/// Re-exported so callers can build typed payloads without reaching for the
/// raw `web-sys` API.
pub use crate::models::PageProgress;

/// Error returned when the Tauri global API cannot be located.
fn no_tauri() -> JsValue {
  JsValue::from_str("Tauri global API is not available")
}

/// Resolve `window.__TAURI__.core` (the namespace that hosts `invoke`).
fn core_namespace() -> Result<JsValue, JsValue> {
  let win = window().ok_or_else(no_tauri)?;
  let tauri = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
  if tauri.is_undefined() || tauri.is_null() {
    return Err(no_tauri());
  }
  Reflect::get(&tauri, &JsValue::from_str("core"))
}

/// Resolve `window.__TAURI__.event` (the namespace that hosts `listen`).
fn event_namespace() -> Result<JsValue, JsValue> {
  let win = window().ok_or_else(no_tauri)?;
  let tauri = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
  if tauri.is_undefined() || tauri.is_null() {
    return Err(no_tauri());
  }
  Reflect::get(&tauri, &JsValue::from_str("event"))
}

/// Invoke a Tauri command and decode its JSON result.
///
/// `args` is the serialised argument object (`{}` for argument-less commands).
/// On a rejected promise the underlying [`JsValue`] is returned so the caller
/// can translate it into a domain error.
pub async fn invoke<T: DeserializeOwned>(cmd: &str, args: &JsValue) -> Result<T, JsValue> {
  let core = core_namespace()?;
  let invoke_fn = Reflect::get(&core, &JsValue::from_str("invoke"))?
    .dyn_into::<Function>()
    .map_err(|_| JsValue::from_str("invoke is not a function"))?;

  let promise = invoke_fn
    .call2(&core, &JsValue::from_str(cmd), args)
    .map_err(|e| JsValue::from_str(&format!("failed to call {cmd}: {e:?}")))?;

  let result = JsFuture::from(promise.unchecked_into::<js_sys::Promise>())
    .await
    .map_err(|e| JsValue::from_str(&format!("{cmd} rejected: {e:?}")))?;

  serde_wasm_bindgen::from_value(result)
    .map_err(|e| JsValue::from_str(&format!("failed to decode {cmd} result: {e:?}")))
}

/// Build a JSON argument object from `(name, value)` pairs.
#[must_use]
pub fn build_args(pairs: &[(&str, &JsValue)]) -> JsValue {
  let obj = js_sys::Object::new();
  for (name, value) in pairs {
    let _ = Reflect::set(&obj, &JsValue::from_str(name), value);
  }
  obj.into()
}

/// Subscribe to `split://progress` events.
///
/// The supplied handler is invoked with each decoded [`PageProgress`] payload.
/// The subscription lives for the lifetime of the page (the handler closure is
/// intentionally leaked); this matches the original app, which only ever ran a
/// single split at a time.
pub async fn listen_progress<F>(handler: F) -> Result<(), JsValue>
where
  F: Fn(PageProgress) + 'static,
{
  let event = event_namespace()?;
  let listen_fn = Reflect::get(&event, &JsValue::from_str("listen"))?
    .dyn_into::<Function>()
    .map_err(|_| JsValue::from_str("event.listen is not a function"))?;

  let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: JsValue| {
    if let Ok(payload) = Reflect::get(&e, &JsValue::from_str("payload")) {
      if let Ok(progress) = serde_wasm_bindgen::from_value::<PageProgress>(payload) {
        handler(progress);
      }
    }
  }) as Box<dyn Fn(JsValue)>);

  let promise = listen_fn
    .call2(
      &event,
      &JsValue::from_str("split://progress"),
      closure.as_ref(),
    )
    .map_err(|e| JsValue::from_str(&format!("failed to subscribe: {e:?}")))?;

  // Keep the closure alive for the page lifetime.
  closure.forget();

  let _ = JsFuture::from(promise.unchecked_into::<js_sys::Promise>()).await;
  Ok(())
}

/// Schedule `cb` to run on the next animation frame.
///
/// Used to throttle high-frequency progress events so the UI repaints at most
/// once per frame.
pub fn request_animation_frame<F>(cb: F)
where
  F: FnOnce() + 'static,
{
  let Some(win) = window() else {
    cb();
    return;
  };
  let closure = wasm_bindgen::closure::Closure::once(Box::new(cb) as Box<dyn FnOnce()>);
  let _ = win.request_animation_frame(closure.as_ref().unchecked_ref());
  closure.forget();
}

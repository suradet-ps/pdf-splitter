//! Low-level Tauri global IPC bindings.
//!
//! The frontend is built by `trunk` with no JS bundler, so it cannot `import`
//! the `@tauri-apps/api` npm module.  Instead `index.html` loads that module
//! **at runtime** from a CDN (transmuted to a `blob:` URL so the CSP still
//! applies) and exposes it as `window.__TAURI__` (with `.core.invoke` and
//! `.event.listen`).  These helpers wrap the raw `web-sys` calls so the rest
//! of the app never touches the DOM.
//!
//! Because the global is loaded asynchronously, the first IPC call may race the
//! CDN import. [`get_tauri`] awaits that load promise before giving
//! up, so a slow import (or an offline CDN) degrades to a clear error
//! instead of an instant failure.

use js_sys::{Function, Promise, Reflect};
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

/// `index.html` loads the `@tauri-apps/api` module **at runtime** and assigns
/// the resolved module to `window.__TAURI__`.  It also stores the load
/// *promise* on `window.__TAURI_PROMISE__` so the IPC layer can await the
/// global's availability instead of racing it.
///
/// Returns the `window` object once `window.__TAURI__` is present.
async fn get_tauri() -> Result<JsValue, JsValue> {
  let win = window().ok_or_else(no_tauri)?;

  // If the global is already here, return immediately.
  let existing = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
  if !existing.is_undefined() && !existing.is_null() {
    return Ok(JsValue::from(win));
  }

  // Otherwise wait for the runtime load promise (created by index.html).
  let promise = Reflect::get(&win, &JsValue::from_str("__TAURI_PROMISE__"))?;
  if promise.is_undefined() || promise.is_null() {
    return Err(no_tauri());
  }
  let promise = promise.dyn_into::<Promise>().map_err(|_| no_tauri())?;
  let _ = JsFuture::from(promise).await;

  let tauri = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
  if tauri.is_undefined() || tauri.is_null() {
    return Err(no_tauri());
  }
  Ok(JsValue::from(win))
}

/// Resolve `window.__TAURI__.core` (the namespace that hosts `invoke`).
async fn core_namespace() -> Result<JsValue, JsValue> {
  let win = get_tauri().await?;
  let tauri = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
  Reflect::get(&tauri, &JsValue::from_str("core"))
}

/// Resolve `window.__TAURI__.event` (the namespace that hosts `listen`).
async fn event_namespace() -> Result<JsValue, JsValue> {
  let win = get_tauri().await?;
  let tauri = Reflect::get(&win, &JsValue::from_str("__TAURI__"))?;
  Reflect::get(&tauri, &JsValue::from_str("event"))
}

/// Invoke a Tauri command and decode its JSON result.
///
/// `args` is the serialised argument object (`{}` for argument-less commands).
/// On a rejected promise the underlying [`JsValue`] is returned so the caller
/// can translate it into a domain error.
pub async fn invoke<T: DeserializeOwned>(cmd: &str, args: &JsValue) -> Result<T, JsValue> {
  let core = core_namespace().await?;
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
  let event = event_namespace().await?;
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
  // Use a repeatable `Closure::wrap` guarded by an `Option` rather than
  // `Closure::once`: a stray second invocation from the browser (or a call
  // after teardown) would otherwise panic with "closure invoked recursively
  // or after being dropped".  Here the callback runs at most once because it
  // is moved out of the `Option` on the first call; any later call is a
  // no-op.  `forget()` leaks the closure intentionally for the page lifetime.
  let slot =
    std::rc::Rc::new(std::cell::RefCell::new(Some(Box::new(cb) as Box<dyn FnOnce()>)));
  let closure_slot = slot.clone();
  let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_ts: JsValue| {
    if let Some(cb) = closure_slot.borrow_mut().take() {
      cb();
    }
  }) as Box<dyn FnMut(JsValue)>);
  let _ = win.request_animation_frame(closure.as_ref().unchecked_ref());
  closure.forget();
}

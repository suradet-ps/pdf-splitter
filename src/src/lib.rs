//! Leptos CSR frontend for PDF Splitter.
//!
//! This crate replaces the original Vue 3 + TypeScript renderer.  All
//! business logic continues to live in the pure-Rust `pdf-split-core` engine
//! and the thin Tauri command layer in `src-tauri`; this crate is responsible
//! for UI only.  Backend commands are reached through Tauri's global IPC
//! (enabled via `app.withGlobalTauri` in `tauri.conf.json`) wrapped by the
//! helpers in [`services`].
//!
//! # Module layout
//!
//! * [`models`] — data types shared with the backend (mirror its JSON shapes).
//! * [`services`] — Tauri `invoke` / event wrappers (no UI here).
//! * [`contexts`] — application state (the Leptos equivalent of a Pinia store).
//! * [`components`] — presentational Leptos components.
//! * [`pages`] — top-level view composition (`App`).

#![deny(clippy::all, unsafe_code)]
// Under `cfg(test)` only the pure-logic unit tests (e.g. in `models`) are
// compiled and the `#[wasm_bindgen(start)]` entry point is disabled, which
// leaves the UI module tree unreferenced.  Silence the resulting dead-code /
// unused-import noise so `cargo test` stays green without `#[allow]` scattered
// across every module.
#![cfg_attr(test, allow(dead_code, unused_imports))]

mod components;
mod contexts;
mod models;
mod pages;
mod services;

/// Application entry point.  Trunk invokes this via the `#[wasm_bindgen(start)]`
/// attribute on the crate root.
#[cfg(not(test))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
  use leptos::prelude::mount_to_body;

  if cfg!(debug_assertions) {
    console_error_panic_hook::set_once();
  }

  mount_to_body(pages::App);
}

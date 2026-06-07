//! Tauri application entry point.
//!
//! This crate is a **thin wrapper** around the pure-Rust
//! `pdf_split_core` engine.  No business logic lives here — only the
//! Tauri command surface, plugin wiring, and window setup.  Any
//! algorithm, validation rule, or data transformation belongs in
//! `crates/pdf-split-core/`.

// Enforce a strict, idiomatic Rust style throughout the crate.
#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    // Forbid unsafe code — this application has no need for it.
    unsafe_code
)]
// Pedantic lints that produce too many false positives for Tauri glue code.
#![allow(
    // Tauri-generated code sometimes triggers this.
    clippy::used_underscore_binding,
    // Module-level docs are preferred over item-level docs for re-exports.
    clippy::module_name_repetitions,
)]

// module declarations

/// Tauri command handlers (thin wrappers around `pdf_split_core`).
pub mod commands;

// public api

/// Build and run the Tauri application.
///
/// This function **never returns** on a successful run; it hands control to
/// the Tauri event loop.  On failure it panics with a descriptive message.
///
/// # Panics
///
/// Panics if the Tauri runtime cannot be initialised.  This should only
/// happen in abnormal conditions (e.g. missing system `WebView` support, which
/// cannot occur on macOS `>= 12` where `WebKit` is always present).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    // Plugin registration
    // Plugins must be registered before `invoke_handler` so that their
    // commands are available when the renderer calls `invoke()`.
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    // Command registration
    //
    // `generate_handler!` is a Tauri macro that builds the dispatch table
    // from the list of `#[tauri::command]` functions.  Every public command
    // in `commands.rs` must appear here.
    .invoke_handler(tauri::generate_handler![
      commands::get_page_count,
      commands::get_file_info,
      commands::pick_pdf_file,
      commands::pick_output_dir,
      commands::split_pdf,
      commands::reveal_in_finder,
    ])
    // Setup
    //
    // Configure window behaviour that is difficult or impossible to express
    // in the static JSON config.  Notably, on Windows the "maximise" button
    // can still fire even with `resizable: false` in the window config; the
    // code below catches and discards those attempts at runtime.
    .setup(move |_app| {
      // Force non-resizable / non-maximisable on every platform where the
      // static JSON config might not be honoured 100 % of the time (Windows
      // title-bar double-click, Linux WM quirks, …).
      #[cfg(any(target_os = "windows", target_os = "linux"))]
      {
        use tauri::Manager;

        let win = _app
          .get_webview_window("main")
          .expect("main window must exist");

        let win_handle = win.clone();
        win.on_window_event(move |event| {
          if let tauri::WindowEvent::Resized(size) = event {
            // `Resized` fires *after* the WM committed the new size, so we
            // must guard to avoid a set_size loop.  Tauri v2 coalesces
            // repeated resize requests; the condition below only fires when
            // the window is genuinely oversized.
            if size.width > 720_u32 || size.height > 560_u32 {
              let _ = win_handle.set_size(tauri::LogicalSize::new(720.0, 560.0));
            }
          }
        });
      }

      Ok(())
    })
    // Launch
    .run(tauri::generate_context!())
    .expect("fatal: Tauri application failed to start");
}

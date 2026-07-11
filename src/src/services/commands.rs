//! Typed wrappers around the individual Tauri commands.
//!
//! Each function maps the raw `invoke` result/error into a domain type or a
//! [`models::PdfError`].  No UI code calls `invoke` directly.

use leptos::prelude::*;
use serde::Deserialize;
use wasm_bindgen::JsValue;

use super::tauri::{build_args, invoke};
use crate::models::{FileInfoResponse, PageProgress, PdfError, SplitResult};

/// `{ kind, message }` wire shape emitted by the backend.
#[derive(Debug, Deserialize)]
struct RawError {
  kind: String,
  message: String,
}

/// Translate a rejected `invoke` payload into a user-facing [`PdfError`].
fn decode_error(raw: JsValue) -> PdfError {
  if let Ok(parsed) = serde_wasm_bindgen::from_value::<RawError>(raw.clone()) {
    return PdfError::from_raw(&parsed.kind, parsed.message);
  }
  let message = raw.as_string().unwrap_or_else(|| format!("{raw:?}"));
  PdfError::from_raw("Internal", message)
}

/// Open the native PDF file-picker.  Returns `None` if the user cancelled.
pub async fn pick_pdf_file() -> Result<Option<String>, PdfError> {
  invoke::<Option<String>>("pick_pdf_file", &JsValue::NULL)
    .await
    .map_err(decode_error)
}

/// Return page count and file size for `path` in a single round-trip.
pub async fn get_file_info(path: &str) -> Result<FileInfoResponse, PdfError> {
  let args = build_args(&[("path", &JsValue::from_str(path))]);
  invoke::<FileInfoResponse>("get_file_info", &args)
    .await
    .map_err(decode_error)
}

/// Open the native directory-picker.  Returns `None` if the user cancelled.
pub async fn pick_output_dir() -> Result<Option<String>, PdfError> {
  invoke::<Option<String>>("pick_output_dir", &JsValue::NULL)
    .await
    .map_err(decode_error)
}

/// Split `input_path` into per-page PDFs inside `output_dir`.
pub async fn split_pdf(input_path: &str, output_dir: &str) -> Result<SplitResult, PdfError> {
  let args = build_args(&[
    ("inputPath", &JsValue::from_str(input_path)),
    ("outputDir", &JsValue::from_str(output_dir)),
  ]);
  invoke::<SplitResult>("split_pdf", &args)
    .await
    .map_err(decode_error)
}

/// Reveal `path` in the platform file manager.
pub async fn reveal_in_finder(path: &str) -> Result<(), PdfError> {
  let args = build_args(&[("path", &JsValue::from_str(path))]);
  invoke::<()>("reveal_in_finder", &args)
    .await
    .map_err(decode_error)
}

/// Subscribe to per-page progress events.  The handler receives decoded
/// [`PageProgress`] snapshots.
pub async fn subscribe_progress<F>(handler: F) -> Result<(), PdfError>
where
  F: Fn(PageProgress) + 'static,
{
  super::tauri::listen_progress(handler)
    .await
    .map_err(|e| PdfError::from_raw("Internal", format!("{e:?}")))
}

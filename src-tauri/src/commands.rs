//! Tauri command handlers — the thin bridge between the renderer and the
//! PDF processing pipeline.

use std::{fs, path::PathBuf};

use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_dialog::FilePath;

use crate::pdf::{self, PageProgress, PdfError, SplitRequest, SplitResult};

// Additional response types

/// Metadata for a PDF file returned by [`get_file_info`].
///
/// Combines page count (from [`pdf::get_page_count`]) and file size (from the
/// filesystem) into a single round-trip so the frontend avoids two separate
/// Tauri invocations.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
  /// Number of pages in the document.
  pub page_count: u32,
  /// File size in bytes (0 if the metadata read fails for any reason).
  pub size_bytes: u64,
}

// Event identifiers

/// Name of the Tauri event emitted after each page is written to disk.
///
/// The payload is [`PageProgress`] serialised as JSON:
/// ```json
/// { "current": 3, "total": 10, "fileName": "page_0003.pdf" }
/// ```
pub const EVENT_SPLIT_PROGRESS: &str = "split://progress";

/// Name of the Tauri event emitted once the entire split has finished.
///
/// The payload is [`SplitResult`] serialised as JSON.
pub const EVENT_SPLIT_COMPLETE: &str = "split://complete";

// Private helpers

/// Convert a [`FilePath`] (Tauri dialog enum) to an owned `String`.
///
/// On macOS (desktop) `FilePath` is always the `Path` variant.  The `Url`
/// variant only occurs on mobile targets and is therefore handled by a
/// fallback `to_string()` call.
fn file_path_to_string(fp: FilePath) -> String {
  match fp {
    FilePath::Path(p) => p.to_string_lossy().into_owned(),
    // `Url` variant is used on mobile targets.  On macOS desktop it is
    // unreachable in practice, but we handle it to satisfy exhaustiveness.
    FilePath::Url(url) => url.to_string(),
  }
}

// Commands

/// Return the number of pages in the PDF at `path`.
///
/// This is intentionally synchronous and fast — it only parses the document
/// structure without loading any page content streams.  The frontend calls
/// this immediately after the user picks a file so it can display the page
/// count in the UI before the user starts the split.
///
/// # Errors
///
/// Forwards [`PdfError`] for missing files, corrupt PDFs, and empty documents.
#[tauri::command]
pub fn get_page_count(path: String) -> Result<u32, PdfError> {
  pdf::get_page_count(&PathBuf::from(path))
}

/// Return both page count and file size for the PDF at `path` in a single
/// round-trip.
///
/// This avoids the need for the renderer to import `@tauri-apps/plugin-fs`
/// solely to read file metadata: all I/O happens in Rust and the result is
/// serialised as `{ pageCount, sizeBytes }` JSON.
///
/// The `sizeBytes` field falls back to `0` if the filesystem metadata read
/// fails (e.g. a race where the file is removed between selection and the
/// metadata call); the UI handles this gracefully by hiding the size display.
///
/// # Errors
///
/// Forwards [`PdfError`] for missing files, corrupt PDFs, and empty documents.
#[tauri::command]
pub fn get_file_info(path: String) -> Result<FileInfo, PdfError> {
  let pb = PathBuf::from(path);
  let page_count = pdf::get_page_count(&pb)?;
  let size_bytes = fs::metadata(&pb).map_or(0, |m| m.len());
  Ok(FileInfo {
    page_count,
    size_bytes,
  })
}

/// Open a native file-picker dialog pre-filtered to PDF files.
///
/// Returns the absolute path chosen by the user, or `None` if the dialog was
/// cancelled.
///
/// # Errors
///
/// Returns [`PdfError::Internal`] if the dialog plugin itself reports an error
/// (extremely unlikely in normal operation).
#[tauri::command]
pub async fn pick_pdf_file<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, PdfError> {
  use tauri_plugin_dialog::DialogExt;

  let path = app
    .dialog()
    .file()
    .set_title("Select a PDF file to split")
    .add_filter("PDF Document", &["pdf"])
    .blocking_pick_file()
    .map(file_path_to_string);

  Ok(path)
}

/// Open a native directory-picker dialog for the user to choose where split
/// pages will be saved.
///
/// Returns the chosen directory path, or `None` if the dialog was cancelled.
///
/// # Errors
///
/// Returns [`PdfError::Internal`] if the dialog plugin reports an error.
#[tauri::command]
pub async fn pick_output_dir<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, PdfError> {
  use tauri_plugin_dialog::DialogExt;

  let path = app
    .dialog()
    .file()
    .set_title("Choose output folder")
    .blocking_pick_folder()
    .map(file_path_to_string);

  Ok(path)
}

/// Split `input_path` into individual-page PDFs inside `output_dir`, emitting
/// a [`EVENT_SPLIT_PROGRESS`] Tauri event after each page is written.
///
/// The split runs on a dedicated Rayon thread pool via
/// [`tauri::async_runtime::spawn_blocking`] so the Tokio executor is never
/// blocked.  Progress events are emitted to **all** windows so the renderer
/// does not need to pass a window reference.
///
/// # Event contract
///
/// | Event | Payload | When |
/// |-------|---------|------|
/// | `split://progress` | [`PageProgress`] (JSON) | After each page |
/// | `split://complete` | [`SplitResult`] (JSON) | On success |
///
/// The command returns the same [`SplitResult`] value that is emitted via
/// `split://complete` so the caller can also use `await` directly if it
/// prefers not to subscribe to events.
///
/// # Errors
///
/// Forwards [`PdfError`] for all failure modes (missing file, corrupt PDF,
/// I/O failure during write, etc.).
#[tauri::command]
pub async fn split_pdf<R: Runtime>(
  app: AppHandle<R>,
  input_path: String,
  output_dir: String,
) -> Result<SplitResult, PdfError> {
  let request = SplitRequest {
    input_path: PathBuf::from(input_path),
    output_dir: PathBuf::from(output_dir),
  };

  // Clone `app` so the closure (which may be called from multiple rayon
  // workers concurrently) can emit events without moving `app` out of the
  // async stack frame.
  let app_handle = app.clone();

  // `spawn_blocking` moves the CPU-intensive work off the async executor
  // thread onto a dedicated blocking thread.  Rayon then distributes
  // individual page processing across all available CPU cores from within
  // that blocking thread.
  let result = tauri::async_runtime::spawn_blocking(move || {
    pdf::split_pdf(request, move |progress: PageProgress| {
      // Emit progress event — best-effort; ignore failures (e.g. if the
      // window was closed mid-operation).
      let _ = app_handle.emit(EVENT_SPLIT_PROGRESS, &progress);
    })
  })
  .await
  .map_err(|join_err| PdfError::Internal(join_err.to_string()))??;

  // Emit the completion event as well so subscribers don't have to await
  // the command promise.
  let _ = app.emit(EVENT_SPLIT_COMPLETE, &result);

  Ok(result)
}

/// Open `path` (a file or directory) in macOS Finder / the platform's default
/// file manager, selecting / revealing the item.
///
/// On macOS this calls `open -R <path>` so the item is *revealed* (selected)
/// rather than opened/launched.  The frontend calls this when the user clicks
/// "Show in Finder" after a successful split.
///
/// # Errors
///
/// Returns [`PdfError::Internal`] if the opener plugin reports an error.
#[tauri::command]
pub async fn reveal_in_finder<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), PdfError> {
  use tauri_plugin_opener::OpenerExt;

  app
    .opener()
    .reveal_item_in_dir(&path)
    .map_err(|e| PdfError::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// `get_page_count` with a non-existent path must return `FileNotFound`.
  #[test]
  fn get_page_count_missing_file_returns_error() {
    let result = get_page_count("/no/such/file.pdf".to_owned());
    assert!(
      matches!(result, Err(PdfError::FileNotFound { .. })),
      "expected FileNotFound, got: {result:?}"
    );
  }

  /// `get_file_info` with a non-existent path must return `FileNotFound`.
  #[test]
  fn get_file_info_missing_file_returns_error() {
    let result = get_file_info("/no/such/file.pdf".to_owned());
    assert!(
      matches!(result, Err(PdfError::FileNotFound { .. })),
      "expected FileNotFound, got: {result:?}"
    );
  }

  /// Event name constants must stay stable — they are part of the public
  /// contract between backend and frontend.
  #[test]
  fn event_name_constants_are_stable() {
    assert_eq!(EVENT_SPLIT_PROGRESS, "split://progress");
    assert_eq!(EVENT_SPLIT_COMPLETE, "split://complete");
  }
}

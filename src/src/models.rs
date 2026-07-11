//! Data models shared with the Tauri backend.
//!
//! These types mirror the JSON contract serialised by `pdf-split-core` and the
//! command layer in `src-tauri`.  They are intentionally *re-declared* here
//! rather than re-exported from the core crate: the frontend runs on
//! `wasm32` and must stay lean, and the wire format (not the Rust types) is
//! the real contract.  The field names and serde conventions match the
//! backend exactly so (de)serialisation is lossless.

use serde::{Deserialize, Serialize};

/// Discrete states the application can be in.
///
/// The UI renders a different view based on the current state:
///
/// | State        | Visible component        |
/// |--------------|--------------------------|
/// | `idle`       | Drop-zone / file picker  |
/// | `ready`      | File info + split button |
/// | `processing` | Progress bar             |
/// | `complete`    | Result list              |
/// | `error`      | Error card               |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AppState {
  /// No file selected yet.
  #[default]
  Idle,
  /// A PDF is selected and ready to split.
  Ready,
  /// A split operation is running.
  Processing,
  /// The last split finished successfully.
  Complete,
  /// The last operation failed.
  Error,
}

impl AppState {
  /// The `data-state` attribute value used by the root element's CSS.
  #[must_use]
  pub fn as_attr(self) -> &'static str {
    match self {
      Self::Idle => "idle",
      Self::Ready => "ready",
      Self::Processing => "processing",
      Self::Complete => "complete",
      Self::Error => "error",
    }
  }
}

/// Metadata about the PDF file the user has selected.
#[derive(Clone, Debug)]
pub struct PdfFileInfo {
  /// Absolute path to the selected PDF file.
  pub path: String,
  /// Display name (basename), e.g. `"report.pdf"`.
  pub name: String,
  /// File size in bytes.
  pub size_bytes: u64,
  /// Number of pages reported by `get_page_count`.
  pub page_count: u32,
}

/// A split operation that is currently running.
#[derive(Clone, Debug, Default)]
pub struct SplitOperation {
  /// Snapshot of the last progress event received.
  pub progress: Option<PageProgress>,
}

/// Progress snapshot emitted after each page is written.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageProgress {
  /// 1-based count of pages completed so far.
  pub current: u32,
  /// Total number of pages in the source document.
  pub total: u32,
  /// Filename of the most-recently completed output file, e.g. `"page_0042.pdf"`.
  pub file_name: String,
}

/// Outcome of a successful split operation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitResult {
  /// Total number of pages found in the source document.
  pub total_pages: u32,
  /// Absolute paths of every output file, sorted lexicographically.
  pub output_files: Vec<String>,
  /// Wall-clock time taken for the whole operation, in milliseconds.
  pub elapsed_ms: u64,
}

/// Machine-readable discriminant of a backend [`PdfError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfErrorKind {
  FileNotFound,
  InvalidPdf,
  Io,
  NoPages,
  Internal,
}

/// Error returned by a backend command.
///
/// The backend serialises `pdf_split_core::PdfError` as `{ "kind", "message" }`;
/// [`Self::from_raw`] converts that shape into this struct.
#[derive(Clone, Debug)]
pub struct PdfError {
  /// Machine-readable discriminant (matches the Rust enum variant name).
  pub kind: PdfErrorKind,
  /// Human-readable description suitable for display.
  pub message: String,
}

impl PdfError {
  /// Build a [`PdfError`] from the `{ kind, message }` JSON object emitted by
  /// the backend.
  #[must_use]
  pub fn from_raw(kind: &str, message: String) -> Self {
    let kind = match kind {
      "FileNotFound" => PdfErrorKind::FileNotFound,
      "InvalidPdf" => PdfErrorKind::InvalidPdf,
      "Io" => PdfErrorKind::Io,
      "NoPages" => PdfErrorKind::NoPages,
      _ => PdfErrorKind::Internal,
    };
    Self { kind, message }
  }
}

/// Response from the `get_file_info` Tauri command.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfoResponse {
  /// Number of pages in the document.
  pub page_count: u32,
  /// File size in bytes.
  pub size_bytes: u64,
}

/// Format `bytes` as a human-readable string, e.g. `"2.4 MB"`.
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
  if bytes == 0 {
    return "0 B".to_owned();
  }
  let units = ["B", "KB", "MB", "GB"];
  let exp = ((bytes as f64).log2() / 10.0).floor() as usize;
  let exp = exp.min(units.len() - 1);
  let value = bytes as f64 / 1024_f64.powi(exp as i32);
  if exp == 0 {
    format!("{value:.0} {}", units[exp])
  } else {
    format!("{value:.1} {}", units[exp])
  }
}

/// Extract the basename from an absolute file path, handling both UNIX `/`
/// and Windows `\` separators.
#[must_use]
pub fn basename(path: &str) -> String {
  path
    .replace('\\', "/")
    .rsplit('/')
    .next()
    .unwrap_or(path)
    .to_owned()
}

/// Format a duration in milliseconds as a human-readable string.
#[must_use]
pub fn format_duration(ms: u64) -> String {
  if ms < 1000 {
    format!("{ms} ms")
  } else {
    format!("{:.1} s", ms as f64 / 1000.0)
  }
}

/// Display-friendly tail of an output directory path (last two components).
#[must_use]
pub fn shorten_dir(path: &str) -> String {
  let normalised = path.replace('\\', "/");
  let parts: Vec<&str> = normalised.split('/').filter(|p| !p.is_empty()).collect();
  if parts.is_empty() {
    return String::new();
  }
  parts
    .iter()
    .rev()
    .take(2)
    .rev()
    .copied()
    .collect::<Vec<_>>()
    .join("/")
}

/// Derive the default output directory from an input file path.
///
/// Rule: `<parent-of-input>/<stem-of-input>/`
/// Example: `/Users/alice/Docs/report.pdf` → `/Users/alice/Docs/report`
#[must_use]
pub fn default_output_dir(input_path: &str) -> String {
  let normalised = input_path.replace('\\', "/");
  let last_slash = normalised.rfind('/');
  let dir = last_slash.map_or(".", |i| &normalised[..i]);
  let file = last_slash.map_or(normalised.as_str(), |i| &normalised[i + 1..]);
  let stem = if file.to_lowercase().ends_with(".pdf") {
    &file[..file.len() - 4]
  } else {
    file
  };
  format!("{dir}/{stem}")
}

#[cfg(test)]
mod tests {
  use super::*;
  use wasm_bindgen_test::wasm_bindgen_test;

  #[wasm_bindgen_test]
  fn shortens_directory_to_last_two_components() {
    assert_eq!(shorten_dir("/Users/alice/Docs/report"), "Docs/report");
    assert_eq!(shorten_dir("C:\\Users\\alice\\report"), "alice/report");
    assert_eq!(shorten_dir(""), "");
  }

  #[wasm_bindgen_test]
  fn default_output_dir_strips_pdf_extension() {
    assert_eq!(
      default_output_dir("/Users/alice/Docs/report.pdf"),
      "/Users/alice/Docs/report"
    );
    assert_eq!(default_output_dir("report.PDF"), "./report");
  }

  #[wasm_bindgen_test]
  fn format_bytes_produces_expected_units() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(2_400), "2.3 KB");
    assert_eq!(format_bytes(3_000_000), "2.9 MB");
  }

  #[wasm_bindgen_test]
  fn format_duration_switches_at_one_second() {
    assert_eq!(format_duration(450), "450 ms");
    assert_eq!(format_duration(1234), "1.2 s");
  }

  #[wasm_bindgen_test]
  fn basename_handles_unix_and_windows() {
    assert_eq!(basename("/a/b/c.pdf"), "c.pdf");
    assert_eq!(basename("C:\\a\\c.pdf"), "c.pdf");
  }

  #[wasm_bindgen_test]
  fn error_kind_is_parsed_from_backend_shape() {
    let err = PdfError::from_raw("NoPages", "no pages".to_owned());
    assert_eq!(err.kind, PdfErrorKind::NoPages);
    assert_eq!(err.message, "no pages");
  }
}

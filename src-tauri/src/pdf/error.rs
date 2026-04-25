//! Error types for the PDF processing pipeline.

use serde::Serialize;

/// All errors that can arise while splitting a PDF document.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
  /// The supplied path does not point to a regular file.
  #[error("File not found: {path}")]
  FileNotFound { path: String },

  /// The file exists but cannot be parsed as a valid PDF.
  #[error("Invalid or corrupt PDF: {0}")]
  InvalidPdf(#[from] lopdf::Error),

  /// A filesystem operation failed (e.g. creating the output directory).
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),

  /// The PDF contains zero pages (nothing to split).
  #[error("The PDF document contains no pages")]
  NoPages,

  /// An unexpected internal error that should never happen in production.
  #[error("Internal error: {0}")]
  Internal(String),
}

impl Serialize for PdfError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(2))?;
    map.serialize_entry("kind", self.kind_str())?;
    map.serialize_entry("message", &self.to_string())?;
    map.end()
  }
}

impl PdfError {
  /// A stable, machine-readable discriminant string for the frontend.
  const fn kind_str(&self) -> &'static str {
    match self {
      Self::FileNotFound { .. } => "FileNotFound",
      Self::InvalidPdf(_) => "InvalidPdf",
      Self::Io(_) => "Io",
      Self::NoPages => "NoPages",
      Self::Internal(_) => "Internal",
    }
  }
}

impl From<String> for PdfError {
  fn from(msg: String) -> Self {
    Self::Internal(msg)
  }
}

impl From<&str> for PdfError {
  fn from(msg: &str) -> Self {
    Self::Internal(msg.to_owned())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn file_not_found_message() {
    let err = PdfError::FileNotFound {
      path: "/tmp/missing.pdf".to_owned(),
    };
    assert!(err.to_string().contains("/tmp/missing.pdf"));
  }

  #[test]
  fn no_pages_message() {
    let err = PdfError::NoPages;
    assert!(err.to_string().contains("no pages"));
  }

  #[test]
  fn internal_from_string() {
    let err = PdfError::from("boom");
    assert!(err.to_string().contains("boom"));
  }

  #[test]
  fn serialize_contains_kind_and_message() {
    let err = PdfError::FileNotFound {
      path: "x.pdf".to_owned(),
    };
    let json = serde_json::to_string(&err).expect("serialisation failed");
    assert!(json.contains("FileNotFound"));
    assert!(json.contains("x.pdf"));
  }

  #[test]
  fn kind_str_variants() {
    assert_eq!(
      PdfError::FileNotFound {
        path: String::new()
      }
      .kind_str(),
      "FileNotFound"
    );
    assert_eq!(PdfError::NoPages.kind_str(), "NoPages");
    assert_eq!(PdfError::Internal(String::new()).kind_str(), "Internal");
  }
}

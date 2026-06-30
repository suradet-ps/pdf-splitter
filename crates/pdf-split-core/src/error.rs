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
  use rstest::rstest;

  #[rstest]
  #[case::file_not_found(PdfError::FileNotFound { path: "/tmp/missing.pdf".to_owned() }, "/tmp/missing.pdf")]
  #[case::no_pages(PdfError::NoPages, "no pages")]
  #[case::internal(PdfError::from("boom"), "boom")]
  #[trace]
  fn error_message_contains_expected_substring(#[case] err: PdfError, #[case] expected: &str) {
    assert!(err.to_string().contains(expected));
  }

  #[rstest]
  #[case::file_not_found(PdfError::FileNotFound { path: String::new() }, "FileNotFound")]
  #[case::no_pages(PdfError::NoPages, "NoPages")]
  #[case::internal(PdfError::Internal(String::new()), "Internal")]
  #[case::io(PdfError::Io(std::io::Error::other("")), "Io")]
  #[trace]
  fn kind_str_returns_correct_discriminant(#[case] err: PdfError, #[case] expected: &str) {
    assert_eq!(err.kind_str(), expected);
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
  fn should_serialize_io_variant_with_kind_and_message() {
    let err = PdfError::Io(std::io::Error::new(
      std::io::ErrorKind::NotFound,
      "disk not found",
    ));
    let json = serde_json::to_string(&err).expect("serialisation failed");
    assert!(json.contains("\"kind\":\"Io\""), "missing Io kind: {json}");
    assert!(json.contains("\"message\""), "missing message key: {json}");
  }

  #[test]
  fn should_convert_io_error_via_from_trait() {
    let io_err = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "truncated");
    let pdf_err: PdfError = io_err.into();
    assert!(matches!(pdf_err, PdfError::Io(_)));
  }

  #[test]
  fn should_convert_str_ref_to_internal_via_from_trait() {
    let err: PdfError = "something broke".into();
    let PdfError::Internal(msg) = err else {
      panic!("expected Internal variant, got: {err:?}");
    };
    assert_eq!(msg, "something broke");
  }
}

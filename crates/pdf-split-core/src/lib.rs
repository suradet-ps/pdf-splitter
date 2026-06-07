//! Pure-Rust PDF splitting engine.
//!
//! `pdf-split-core` contains all of the framework-agnostic logic used by
//! the Tauri application.  It is intentionally **Tauri-free** — no
//! `tauri`/`tauri-build` dependency, no FFI, no `unsafe` — so that the
//! splitting pipeline can be unit-tested, fuzz-tested, or `miri`-tested
//! in isolation from the desktop shell.
//!
//! # Public surface
//!
//! The crate exposes a deliberately small API:
//!
//! - [`get_page_count`] — quickly reads the number of pages in a PDF.
//! - [`split_pdf`] — splits a PDF into per-page files, reporting progress
//!   through a user-supplied callback.
//! - [`PdfError`] — the only error type callers need to handle.
//! - The accompanying request / result / progress structs ([`SplitRequest`],
//!   [`SplitResult`], [`PageProgress`]) are also part of the public API
//!   because they describe the input/output contract of [`split_pdf`].

#![deny(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    // Workspace forbids unsafe code by default.
    unsafe_code
)]
#![allow(
    // The crate name naturally contains the project name.
    clippy::module_name_repetitions,
)]

mod error;
mod splitter;

pub use error::PdfError;
pub use splitter::{PageProgress, SplitRequest, SplitResult, get_page_count, split_pdf};

/// Test-only helpers for building synthetic PDF documents.
///
/// Exposed (with `#[doc(hidden)]`) so that integration tests in *other*
/// crates — e.g. the Tauri command layer in `src-tauri` — can reuse the
/// same fixtures instead of duplicating the lopdf boilerplate.  These
/// functions are **not** part of the public, stable API and may change at
/// any time.
#[doc(hidden)]
pub mod test_utils {
use std::fs;
use std::path::PathBuf;

/// Build a minimal but structurally valid PDF with `page_count` blank
  /// pages using the lopdf API.  Panics on any internal error — this is
  /// intentional in a test context so failures surface as clear panics.
  #[must_use]
  pub fn make_minimal_pdf(page_count: usize) -> Vec<u8> {
    use lopdf::{Document, Object, dictionary};

    assert!(page_count > 0, "page_count must be at least 1");

    let mut doc = Document::with_version("1.7");

    let pages_id = doc.new_object_id();

    let kid_refs: Vec<Object> = (0..page_count)
      .map(|_| {
        let page = dictionary! {
            "Type"      => "Page",
            "Parent"    => Object::Reference(pages_id),
            "MediaBox"  => Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(612),
                Object::Integer(792),
            ]),
        };
        let pid = doc.add_object(Object::Dictionary(page));
        Object::Reference(pid)
      })
      .collect();

    let pages = dictionary! {
        "Type"  => "Pages",
        "Kids"  => Object::Array(kid_refs),
        "Count" => Object::Integer(i64::try_from(page_count).expect("page_count fits i64")),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog = dictionary! {
        "Type"  => "Catalog",
        "Pages" => Object::Reference(pages_id),
    };
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc
      .save_to(&mut buf)
      .expect("test helper: failed to serialise PDF");
    buf
  }

  /// Write `bytes` to a temporary file inside `dir` and return its path.
  #[must_use]
  pub fn write_pdf(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("test helper: failed to write PDF");
    path
  }

  /// Write arbitrary bytes to a temporary file inside `dir` and return its
  /// path.  Used to forge "corrupt" PDFs in error-path tests.
  #[must_use]
  pub fn write_bytes(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("test helper: failed to write file");
    path
  }

  /// Build a valid PDF with zero pages — used to exercise the `NoPages`
  /// error path.
  #[must_use]
  pub fn make_empty_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, dictionary};

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let pages = dictionary! {
        "Type"  => "Pages",
        "Kids"  => Object::Array(vec![]),
        "Count" => Object::Integer(0),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog = dictionary! {
        "Type"  => "Catalog",
        "Pages" => Object::Reference(pages_id),
    };
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc
      .save_to(&mut buf)
      .expect("test helper: failed to serialise empty PDF");
    buf
  }

  /// Build a PDF whose pages share a single font resource via their
  /// `/Resources` dictionaries.  Used to verify that the deep-copy logic
  /// correctly pulls in shared objects for each output page.
  #[must_use]
  pub fn make_pdf_with_shared_font(page_count: usize) -> Vec<u8> {
    use lopdf::{Document, Object, dictionary};

    assert!(page_count > 0, "page_count must be at least 1");

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let font_dict = dictionary! {
        "Type"    => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    };
    let font_id = doc.add_object(Object::Dictionary(font_dict));

    let kid_refs: Vec<Object> = (0..page_count)
      .map(|_| {
        let font_res = dictionary! {
            "F1" => Object::Reference(font_id),
        };
        let resources = dictionary! {
            "Font" => Object::Dictionary(font_res),
        };
        let page = dictionary! {
            "Type"      => "Page",
            "Parent"    => Object::Reference(pages_id),
            "Resources" => Object::Dictionary(resources),
            "MediaBox"  => Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(612),
                Object::Integer(792),
            ]),
        };
        let pid = doc.add_object(Object::Dictionary(page));
        Object::Reference(pid)
      })
      .collect();

    let pages = dictionary! {
        "Type"  => "Pages",
        "Kids"  => Object::Array(kid_refs),
        "Count" => Object::Integer(i64::try_from(page_count).expect("page_count fits i64")),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog = dictionary! {
        "Type"  => "Catalog",
        "Pages" => Object::Reference(pages_id),
    };
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut buf = Vec::new();
    doc
      .save_to(&mut buf)
      .expect("test helper: failed to serialise PDF with shared font");
    buf
  }
}

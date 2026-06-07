//! Integration tests for `pdf-split-core`.
//!
//! These tests live under `tests/` (rather than inside `src/`) so they
//! exercise the crate exclusively through its *public* API — exactly the
//! same surface a downstream crate like `src-tauri` would see.
//!
//! These tests all go through `lopdf::Document::load` and therefore
//! trigger the same `crossbeam-epoch 0.9.18` Stacked-Borrows false
//! positive under Miri as the unit tests in `src/splitter.rs`.  See the
//! comment in that file for the full explanation.  We disable the
//! whole file when `cfg(miri)` is set so the Miri job runs cleanly.

// Equivalent of `#[cfg(not(miri))]` at file scope for an integration
// test binary.  Wrapping every `#[test]` individually would also work
// but is more verbose and easier to forget when adding new tests.
#![cfg(not(miri))]

use std::path::PathBuf;

use pdf_split_core::{
  PageProgress, SplitRequest,
  test_utils::{make_minimal_pdf, write_pdf},
};

#[test]
fn public_api_can_count_pages() {
  let dir = tempfile::tempdir().expect("tempdir");
  let path = write_pdf(&dir, "x.pdf", &make_minimal_pdf(3));
  assert_eq!(pdf_split_core::get_page_count(&path).expect("count"), 3);
}

#[test]
fn public_api_can_split_pdf() {
  let dir = tempfile::tempdir().expect("tempdir");
  let input = write_pdf(&dir, "in.pdf", &make_minimal_pdf(2));
  let out_dir = dir.path().join("out");

  let result = pdf_split_core::split_pdf(
    SplitRequest {
      input_path: input,
      output_dir: out_dir,
    },
    |_p: PageProgress| {},
  )
  .expect("split");

  assert_eq!(result.total_pages, 2);
  assert_eq!(result.output_files.len(), 2);
  for p in &result.output_files {
    assert!(p.exists(), "{p:?} should exist on disk");
  }
}

#[test]
fn public_api_error_for_missing_file_has_helpful_path() {
  let missing: PathBuf = "/definitely/not/here.pdf".into();
  let err = pdf_split_core::get_page_count(&missing).expect_err("must fail");
  let msg = err.to_string();
  assert!(msg.contains("/definitely/not/here.pdf"), "got: {msg}");
}

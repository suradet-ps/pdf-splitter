//! Core PDF splitting logic.

use std::{
  collections::{BTreeMap, HashSet},
  fs,
  path::{Path, PathBuf},
  time::Instant,
};

use lopdf::{Document, Object, ObjectId, dictionary};

use crate::error::PdfError;

/// Parameters for a single split operation.
#[derive(Debug, Clone)]
pub struct SplitRequest {
  /// Absolute path to the source PDF file.
  pub input_path: PathBuf,
  /// Directory into which individual-page PDFs will be written.  Created
  /// automatically (including intermediate directories) if it does not exist.
  pub output_dir: PathBuf,
}

/// Outcome of a successful split operation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitResult {
  /// Total number of pages found in the source document.
  pub total_pages: u32,
  /// Paths of every output file, sorted lexicographically.
  pub output_files: Vec<PathBuf>,
  /// Wall-clock time taken for the whole operation, in milliseconds.
  pub elapsed_ms: u64,
}

/// Progress snapshot emitted after each page finishes processing.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageProgress {
  /// 1-based count of pages that have been written to disk so far.
  pub current: u32,
  /// Total number of pages to process.
  pub total: u32,
  /// Filename of the most-recently completed output file
  /// (e.g. `"page_0042.pdf"`).
  pub file_name: String,
}

/// Return the number of pages in the PDF at `path` without performing a full split.
///
/// # Errors
///
/// Returns [`PdfError::FileNotFound`] if `path` does not point to a regular file,
/// [`PdfError::InvalidPdf`] if the file cannot be parsed as a valid PDF,
/// [`PdfError::NoPages`] if the document contains zero pages, and
/// [`PdfError::Internal`] if the page count overflows `u32`.
pub fn get_page_count(path: &Path) -> Result<u32, PdfError> {
  if !path.is_file() {
    return Err(PdfError::FileNotFound {
      path: path.display().to_string(),
    });
  }

  let doc = Document::load(path)?;
  let count = doc.get_pages().len();

  if count == 0 {
    return Err(PdfError::NoPages);
  }

  u32::try_from(count)
    .map_err(|_| PdfError::Internal(format!("document has {count} pages, which overflows u32")))
}

/// Split every page of the PDF at `request.input_path` into its own PDF file inside `request.output_dir`.
///
/// # Errors
///
/// Returns [`PdfError::FileNotFound`] if the input path does not point to a regular file,
/// [`PdfError::InvalidPdf`] if the source file cannot be parsed as a valid PDF,
/// [`PdfError::NoPages`] if the document contains zero pages,
/// [`PdfError::Io`] if a filesystem operation fails (e.g. creating the output directory or
/// writing a page file), and [`PdfError::Internal`] for unexpected internal failures.
pub fn split_pdf<F>(request: SplitRequest, on_progress: F) -> Result<SplitResult, PdfError>
where
  F: Fn(PageProgress) + Send + Sync,
{
  let started_at = Instant::now();

  let SplitRequest {
    input_path,
    output_dir,
  } = request;

  if !input_path.is_file() {
    return Err(PdfError::FileNotFound {
      path: input_path.display().to_string(),
    });
  }

  fs::create_dir_all(&output_dir)?;

  let source = Document::load(&input_path)?;

  // Collect page mappings: (1-based page number) → ObjectId, sorted by
  // page number to ensure deterministic output order.
  let page_map: BTreeMap<u32, ObjectId> = source.get_pages();

  if page_map.is_empty() {
    return Err(PdfError::NoPages);
  }

  let sorted_pages: Vec<(u32, ObjectId)> = page_map.into_iter().collect();
  let total = u32::try_from(sorted_pages.len())
    .map_err(|_| PdfError::Internal("page count overflows u32".to_owned()))?;

  let mut output_files: Vec<PathBuf> = Vec::with_capacity(sorted_pages.len());

  for (seq_index, (_page_num, page_object_id)) in sorted_pages.iter().enumerate() {
    let file_name = format!("page_{:04}.pdf", seq_index + 1);
    let output_path = output_dir.join(&file_name);

    // Build a new single-page document by deep-copying the page and all
    // its transitive dependencies from the source.
    let mut single_page_doc = build_single_page_document(&source, *page_object_id)?;
    single_page_doc.save(&output_path)?;

    output_files.push(output_path);

    // Report progress (1-based, strictly increasing)
    let current = u32::try_from(seq_index + 1).unwrap_or(u32::MAX);
    on_progress(PageProgress {
      current,
      total,
      file_name,
    });
  }

  output_files.sort_unstable();

  Ok(SplitResult {
    total_pages: total,
    output_files,
    elapsed_ms: u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
  })
}

/// Build a new `Document` containing exactly one page by deep-copying a page
/// and all its transitive object dependencies from the source document.
fn build_single_page_document(
  source: &Document,
  page_object_id: ObjectId,
) -> Result<Document, PdfError> {
  let mut new_doc = Document::with_version("1.7");

  // Step 1: Gather all objects transitively referenced by this page

  let mut visited: HashSet<ObjectId> = HashSet::new();
  collect_referenced_objects(source, page_object_id, &mut visited);

  // Step 2: Copy all gathered objects into the new document

  // Map from old ObjectId → new ObjectId
  let mut id_map: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();

  // First pass: allocate new IDs for all objects
  for &old_id in &visited {
    if let Ok(obj) = source.get_object(old_id) {
      let new_id = new_doc.add_object(obj.clone());
      id_map.insert(old_id, new_id);
    }
  }

  // Second pass: rewrite all references in the copied objects to use new IDs
  let all_new_ids: Vec<ObjectId> = id_map.values().copied().collect();
  for new_id in &all_new_ids {
    if let Ok(obj) = new_doc.get_object_mut(*new_id) {
      remap_references(obj, &id_map);
    }
  }

  // Step 3: Find the new ID of the copied page object

  let new_page_id = *id_map
    .get(&page_object_id)
    .ok_or_else(|| PdfError::Internal("failed to find copied page object".to_owned()))?;

  // Step 4: Build Pages node pointing to the single page

  let pages_id = new_doc.new_object_id();

  // Update the page's /Parent reference to point to our new Pages node
  if let Ok(Object::Dictionary(page_dict)) = new_doc.get_object_mut(new_page_id) {
    page_dict.set("Parent", Object::Reference(pages_id));
  }

  let pages = dictionary! {
      "Type"  => "Pages",
      "Kids"  => Object::Array(vec![Object::Reference(new_page_id)]),
      "Count" => Object::Integer(1),
  };
  new_doc.objects.insert(pages_id, Object::Dictionary(pages));

  // Step 5: Build Catalog pointing to Pages

  let catalog = dictionary! {
      "Type"  => "Catalog",
      "Pages" => Object::Reference(pages_id),
  };
  let catalog_id = new_doc.add_object(Object::Dictionary(catalog));
  new_doc.trailer.set("Root", Object::Reference(catalog_id));

  // Step 6: Clean up

  // Compact object IDs for smaller file sizes.
  new_doc.renumber_objects();
  // Compress any uncompressed streams.
  new_doc.compress();

  Ok(new_doc)
}

/// Recursively collect all `ObjectId`s that are transitively referenced
/// starting from `root_id`.
fn collect_referenced_objects(
  source: &Document,
  root_id: ObjectId,
  visited: &mut HashSet<ObjectId>,
) {
  if !visited.insert(root_id) {
    return; // Already visited
  }

  let Ok(obj) = source.get_object(root_id) else {
    return; // Dangling reference — skip
  };

  collect_references_from_object(source, obj, visited);
}

/// Walk an `Object` value and recursively collect all referenced object IDs.
fn collect_references_from_object(
  source: &Document,
  obj: &Object,
  visited: &mut HashSet<ObjectId>,
) {
  match obj {
    Object::Reference(id) => {
      collect_referenced_objects(source, *id, visited);
    }
    Object::Array(arr) => {
      for item in arr {
        collect_references_from_object(source, item, visited);
      }
    }
    Object::Dictionary(dict) => {
      for (key, value) in dict {
        // Skip /Parent references — we'll set this ourselves to avoid
        // pulling in the entire page tree from the source document.
        if key == b"Parent" {
          continue;
        }
        collect_references_from_object(source, value, visited);
      }
    }
    Object::Stream(stream) => {
      // The stream's dictionary may contain references too
      for (key, value) in &stream.dict {
        if key == b"Parent" {
          continue;
        }
        collect_references_from_object(source, value, visited);
      }
    }
    // Primitive types (Name, String, Integer, Real, Boolean, Null) have
    // no outgoing references.
    _ => {}
  }
}

/// Rewrite all `Object::Reference` values in `obj` using the provided ID map.
fn remap_references(obj: &mut Object, id_map: &BTreeMap<ObjectId, ObjectId>) {
  match obj {
    Object::Reference(id) => {
      if let Some(&new_id) = id_map.get(id) {
        *id = new_id;
      }
    }
    Object::Array(arr) => {
      for item in arr.iter_mut() {
        remap_references(item, id_map);
      }
    }
    Object::Dictionary(dict) => {
      for (_, value) in dict.iter_mut() {
        remap_references(value, id_map);
      }
    }
    Object::Stream(stream) => {
      for (_, value) in &mut stream.dict {
        remap_references(value, id_map);
      }
    }
    _ => {}
  }
}

// Tests

// Every test in this module exercises the lopdf code path
// (`Document::load` / `Document::save`).  `lopdf` parses PDFs in parallel
// via `rayon`, which in turn spawns a worker pool backed by
// `crossbeam-epoch 0.9.18`.  That version of crossbeam-epoch trips a
// Stacked-Borrows false positive in its lazy thread-local init
// (`internal.rs:549`) when run under Miri — see
// https://github.com/crossbeam-rs/crossbeam/issues for the upstream
// discussion.  No combination of MIRIFLAGS / RAYON_NUM_THREADS silences
// it because the UB fires on the very first interaction between the
// worker thread and the epoch collector.
//
// We therefore exclude this module from the Miri build.  The
// *production* code (the function bodies above) is still compiled
// under Miri, so Miri still type-checks and analyses it; the only
// thing Miri does not run is these end-to-end tests.  The `error`
// module's tests, which never touch lopdf, run under Miri as usual.
#[cfg(all(test, not(miri)))]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};

  use rstest::{fixture, rstest};

  use crate::test_utils::{
    make_empty_pdf, make_minimal_pdf, make_pdf_with_shared_font, write_bytes, write_pdf,
  };

  // ---------------------------------------------------------------------------
  // Fixtures
  // ---------------------------------------------------------------------------

  /// Shared test context: a temp directory (kept alive), the input path, and
  /// the output directory.
  #[derive(Debug)]
  struct Ctx {
    _dir: tempfile::TempDir,
    input: PathBuf,
    out_dir: PathBuf,
  }

  /// Create a temp directory with a source PDF of `page_count` pages.
  #[fixture]
  fn ctx(#[default(1)] page_count: usize) -> Ctx {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(page_count));
    let out_dir = dir.path().join("output");
    Ctx {
      _dir: dir,
      input,
      out_dir,
    }
  }

  /// A [`Ctx`] whose input file does not exist on disk.
  #[fixture]
  fn ctx_missing() -> Ctx {
    let dir = tempfile::tempdir().expect("tempdir");
    let out_dir = dir.path().join("output");
    Ctx {
      _dir: dir,
      input: PathBuf::from("/no/such/file.pdf"),
      out_dir,
    }
  }

  /// A [`Ctx`] whose input is an empty (zero-page) PDF.
  #[fixture]
  fn ctx_empty() -> Ctx {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "empty.pdf", &make_empty_pdf());
    let out_dir = dir.path().join("output");
    Ctx {
      _dir: dir,
      input,
      out_dir,
    }
  }

  /// A [`Ctx`] whose input is corrupt (not a valid PDF).
  #[fixture]
  fn ctx_corrupt() -> Ctx {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_bytes(&dir, "corrupt.pdf", b"not a valid PDF at all");
    let out_dir = dir.path().join("output");
    Ctx {
      _dir: dir,
      input,
      out_dir,
    }
  }

  /// Outcome of a split plus the collected progress events.
  #[derive(Debug)]
  struct SplitWithProgress {
    #[allow(dead_code)]
    result: SplitResult,
    events: Vec<PageProgress>,
  }

  /// Run [`split_pdf`] and collect every [`PageProgress`] event.
  #[fixture]
  fn split_with_progress(#[default(1)] page_count: usize) -> SplitWithProgress {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(page_count));
    let out_dir = dir.path().join("output");

    let events: Arc<Mutex<Vec<PageProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&events);

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      move |p| {
        log.lock().expect("mutex poisoned").push(p);
      },
    )
    .expect("split");

    SplitWithProgress {
      result,
      events: events.lock().expect("mutex").clone(),
    }
  }

  // ---------------------------------------------------------------------------
  // Error paths
  // ---------------------------------------------------------------------------

  #[rstest]
  fn get_page_count_missing_file(ctx_missing: Ctx) {
    let result = get_page_count(&ctx_missing.input);
    assert!(
      matches!(result, Err(PdfError::FileNotFound { .. })),
      "expected FileNotFound, got: {result:?}"
    );
  }

  #[rstest]
  fn split_pdf_missing_input(ctx_missing: Ctx) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx_missing.input,
        output_dir: ctx_missing.out_dir,
      },
      |_| {},
    );
    assert!(
      matches!(result, Err(PdfError::FileNotFound { .. })),
      "expected FileNotFound, got: {result:?}"
    );
  }

  #[rstest]
  fn get_page_count_empty_pdf(ctx_empty: Ctx) {
    let result = get_page_count(&ctx_empty.input);
    assert!(
      matches!(result, Err(PdfError::NoPages)),
      "expected NoPages, got: {result:?}"
    );
  }

  #[rstest]
  fn split_pdf_empty_input(ctx_empty: Ctx) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx_empty.input,
        output_dir: ctx_empty.out_dir,
      },
      |_| {},
    );
    assert!(
      matches!(result, Err(PdfError::NoPages)),
      "expected NoPages, got: {result:?}"
    );
  }

  #[rstest]
  fn get_page_count_corrupt_file(ctx_corrupt: Ctx) {
    let result = get_page_count(&ctx_corrupt.input);
    assert!(
      matches!(result, Err(PdfError::InvalidPdf(_))),
      "expected InvalidPdf, got: {result:?}"
    );
  }

  #[rstest]
  fn split_pdf_corrupt_input(ctx_corrupt: Ctx) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx_corrupt.input,
        output_dir: ctx_corrupt.out_dir,
      },
      |_| {},
    );
    assert!(
      matches!(result, Err(PdfError::InvalidPdf(_))),
      "expected InvalidPdf, got: {result:?}"
    );
  }

  // ---------------------------------------------------------------------------
  // get_page_count — happy path
  // ---------------------------------------------------------------------------

  #[rstest]
  #[case::single(1)]
  #[case::multiple(5)]
  #[trace]
  fn get_page_count_returns_correct_count(#[case] page_count: usize, #[with(page_count)] ctx: Ctx) {
    assert_eq!(
      get_page_count(&ctx.input).expect("count"),
      u32::try_from(page_count).unwrap()
    );
  }

  // ---------------------------------------------------------------------------
  // split_pdf — happy path
  // ---------------------------------------------------------------------------

  #[rstest]
  #[case::one_page(1)]
  #[case::four_pages(4)]
  #[trace]
  fn split_pdf_produces_correct_number_of_files(
    #[case] page_count: usize,
    #[with(page_count)] ctx: Ctx,
  ) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split should succeed");

    assert_eq!(result.total_pages, u32::try_from(page_count).unwrap());
    assert_eq!(result.output_files.len(), page_count);
  }

  #[rstest]
  #[case::three_pages(3)]
  #[case::six_pages(6)]
  #[trace]
  fn split_pdf_all_output_files_exist_on_disk(
    #[case] page_count: usize,
    #[with(page_count)] ctx: Ctx,
  ) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split");

    assert_eq!(result.output_files.len(), page_count);
    for path in &result.output_files {
      assert!(path.exists(), "expected {path:?} to exist on disk");
    }
  }

  #[rstest]
  fn split_pdf_output_files_are_sorted(ctx: Ctx) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split");

    let mut expected = result.output_files.clone();
    expected.sort_unstable();
    assert_eq!(
      result.output_files, expected,
      "output_files should be lexicographically sorted"
    );
  }

  #[rstest]
  fn split_pdf_output_files_have_sequential_names(#[with(3)] ctx: Ctx) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split");

    let names: Vec<String> = result
      .output_files
      .iter()
      .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
      .collect();

    assert_eq!(
      names,
      vec!["page_0001.pdf", "page_0002.pdf", "page_0003.pdf"]
    );
  }

  #[rstest]
  fn split_pdf_creates_output_dir_if_missing(#[with(1)] ctx: Ctx) {
    assert!(
      !ctx.out_dir.exists(),
      "pre-condition: dir should not exist yet"
    );

    split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir.clone(),
      },
      |_| {},
    )
    .expect("split");

    assert!(
      ctx.out_dir.exists(),
      "output directory should have been created"
    );
  }

  #[rstest]
  #[case::single_page(1)]
  #[case::multi_page(3)]
  #[trace]
  fn split_pdf_each_output_is_a_single_page_pdf(
    #[case] page_count: usize,
    #[with(page_count)] ctx: Ctx,
  ) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split");

    assert_eq!(result.output_files.len(), page_count);
    for path in &result.output_files {
      let doc = Document::load(path).expect("output PDF should be loadable");
      assert_eq!(
        doc.get_pages().len(),
        1,
        "{path:?} should contain exactly 1 page"
      );
    }
  }

  #[rstest]
  fn split_result_elapsed_ms_is_accessible(ctx: Ctx) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split");

    // elapsed_ms may be 0 on very fast machines; we just assert the field
    // is present and of the correct type.
    let _: u64 = result.elapsed_ms;
  }

  #[rstest]
  fn should_succeed_when_output_directory_already_exists(#[with(2)] ctx: Ctx) {
    fs::create_dir_all(&ctx.out_dir).expect("pre-create output dir");

    split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split should succeed when output dir already exists");
  }

  #[rstest]
  fn should_serialize_split_result_as_camelcase_json(#[with(1)] ctx: Ctx) {
    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      |_| {},
    )
    .expect("split");

    let json = serde_json::to_string(&result).expect("serialisation failed");
    assert!(
      json.contains("\"totalPages\":"),
      "missing totalPages: {json}"
    );
    assert!(
      json.contains("\"outputFiles\":"),
      "missing outputFiles: {json}"
    );
    assert!(json.contains("\"elapsedMs\":"), "missing elapsedMs: {json}");
  }

  // ---------------------------------------------------------------------------
  // Progress callback
  // ---------------------------------------------------------------------------

  #[rstest]
  #[case::single(1)]
  #[case::five(5)]
  #[trace]
  fn split_pdf_progress_callback_is_called_for_every_page(
    #[case] page_count: usize,
    #[with(page_count)] split_with_progress: SplitWithProgress,
  ) {
    assert_eq!(
      split_with_progress.events.len(),
      page_count,
      "callback should be called once per page"
    );
  }

  #[rstest]
  #[case::two(2)]
  #[case::four(4)]
  #[trace]
  fn split_pdf_progress_total_is_correct(
    #[case] expected_total: u32,
    #[with(expected_total as usize)] split_with_progress: SplitWithProgress,
  ) {
    for event in &split_with_progress.events {
      assert_eq!(
        event.total, expected_total,
        "every progress event should report total={expected_total}"
      );
    }
  }

  #[rstest]
  #[case::three(3, vec![1, 2, 3])]
  #[case::five(5, vec![1, 2, 3, 4, 5])]
  #[trace]
  fn split_pdf_progress_current_values_cover_full_range(
    #[case] page_count: usize,
    #[case] expected_currents: Vec<u32>,
    #[with(page_count)] split_with_progress: SplitWithProgress,
  ) {
    let _ = &page_count;
    let currents: Vec<u32> = split_with_progress
      .events
      .iter()
      .map(|e| e.current)
      .collect();
    assert_eq!(
      currents, expected_currents,
      "current values 1..=total must appear in order"
    );
  }

  // ---------------------------------------------------------------------------
  // Serialisation
  // ---------------------------------------------------------------------------

  #[test]
  fn should_serialize_page_progress_as_camelcase_json() {
    let progress = PageProgress {
      current: 3,
      total: 10,
      file_name: "page_0003.pdf".to_owned(),
    };
    let json = serde_json::to_string(&progress).expect("serialisation failed");
    assert!(json.contains("\"current\":3"), "missing current: {json}");
    assert!(json.contains("\"total\":10"), "missing total: {json}");
    assert!(
      json.contains("\"fileName\":\"page_0003.pdf\""),
      "missing fileName: {json}"
    );
  }

  // ---------------------------------------------------------------------------
  // Output file integrity
  // ---------------------------------------------------------------------------

  #[rstest]
  fn should_emit_progress_with_file_names_matching_output(#[with(3)] ctx: Ctx) {
    let log: Arc<Mutex<Vec<PageProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = Arc::clone(&log);

    let result = split_pdf(
      SplitRequest {
        input_path: ctx.input,
        output_dir: ctx.out_dir,
      },
      move |p| {
        log_clone.lock().expect("mutex").push(p);
      },
    )
    .expect("split");

    let progress_events = log.lock().expect("mutex").clone();

    assert_eq!(progress_events.len(), 3);
    for (i, event) in progress_events.iter().enumerate() {
      let expected_name = format!("page_{:04}.pdf", i + 1);
      assert_eq!(event.file_name, expected_name);
    }

    for event in &progress_events {
      let path_in_result = result.output_files.iter().find(|p| {
        p.file_name()
          .is_some_and(|n| n.to_string_lossy().as_ref() == event.file_name.as_str())
      });
      assert!(
        path_in_result.is_some(),
        "progress file name '{}' not found in output_files",
        event.file_name
      );
    }
  }

  #[rstest]
  #[case::two_pages(2)]
  #[case::four_pages(4)]
  #[trace]
  fn should_preserve_shared_resources_in_output_pages(#[case] page_count: usize) {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_pdf_with_shared_font(page_count));
    let out_dir = dir.path().join("shared");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    )
    .expect("split");

    for path in &result.output_files {
      let doc = Document::load(path).expect("output should be loadable");
      assert_eq!(doc.get_pages().len(), 1);
      assert!(
        doc.objects.len() >= 4,
        "expected >= 4 objects (catalog, pages, page, font) in {path:?}, got {}",
        doc.objects.len()
      );
    }
  }
}

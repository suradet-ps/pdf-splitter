//! Core PDF splitting logic.

use std::{
  collections::{BTreeMap, HashSet},
  fs,
  path::{Path, PathBuf},
  time::Instant,
};

use lopdf::{Document, Object, ObjectId, dictionary};

use super::error::PdfError;

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

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};

  // Test helpers

  /// Build a minimal but structurally valid PDF with `page_count` blank
  /// pages using the lopdf API.  Panics on any internal error — this is
  /// intentional in a test context so failures surface as clear panics.
  fn make_minimal_pdf(page_count: usize) -> Vec<u8> {
    use lopdf::{Document, Object, dictionary};

    assert!(page_count > 0, "page_count must be at least 1");

    let mut doc = Document::with_version("1.7");

    // Reserve the Pages node ID so we can forward-reference it from each
    // Page's /Parent entry.
    let pages_id = doc.new_object_id();

    // Create individual Page objects.
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

    // Insert Pages node at the pre-reserved ID.
    let pages = dictionary! {
        "Type"  => "Pages",
        "Kids"  => Object::Array(kid_refs),
        "Count" => Object::Integer(i64::try_from(page_count).expect("page_count fits i64")),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    // Catalog
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
  fn write_pdf(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("test helper: failed to write PDF");
    path
  }

  #[test]
  fn get_page_count_returns_error_for_missing_file() {
    let result = get_page_count(Path::new("/nonexistent/path/file.pdf"));
    assert!(
      matches!(result, Err(PdfError::FileNotFound { .. })),
      "expected FileNotFound, got: {result:?}"
    );
  }

  #[test]
  fn get_page_count_single_page() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_pdf(&dir, "single.pdf", &make_minimal_pdf(1));
    assert_eq!(get_page_count(&path).expect("count"), 1);
  }

  #[test]
  fn get_page_count_multiple_pages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_pdf(&dir, "multi.pdf", &make_minimal_pdf(5));
    assert_eq!(get_page_count(&path).expect("count"), 5);
  }

  // split_pdf — error paths
  #[test]
  fn split_pdf_returns_error_for_missing_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = split_pdf(
      SplitRequest {
        input_path: PathBuf::from("/no/such/file.pdf"),
        output_dir: dir.path().to_path_buf(),
      },
      |_| {},
    );
    assert!(
      matches!(result, Err(PdfError::FileNotFound { .. })),
      "expected FileNotFound, got: {result:?}"
    );
  }

  // split_pdf — happy path

  #[test]
  fn split_pdf_produces_correct_number_of_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(4));
    let out_dir = dir.path().join("output");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    )
    .expect("split should succeed");

    assert_eq!(result.total_pages, 4);
    assert_eq!(result.output_files.len(), 4);
  }

  #[test]
  fn split_pdf_all_output_files_exist_on_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(3));
    let out_dir = dir.path().join("out");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    )
    .expect("split");

    for path in &result.output_files {
      assert!(path.exists(), "expected {path:?} to exist on disk");
    }
  }

  #[test]
  fn split_pdf_output_files_are_sorted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(6));
    let out_dir = dir.path().join("sorted");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
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

  #[test]
  fn split_pdf_output_files_have_sequential_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(3));
    let out_dir = dir.path().join("named");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
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

  #[test]
  fn split_pdf_creates_output_dir_if_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(1));
    // Deeply nested directory that does not exist yet.
    let out_dir = dir.path().join("a").join("b").join("c");

    assert!(!out_dir.exists(), "pre-condition: dir should not exist yet");

    split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir.clone(),
      },
      |_| {},
    )
    .expect("split");

    assert!(
      out_dir.exists(),
      "output directory should have been created"
    );
  }

  // Progress callback

  #[test]
  fn split_pdf_progress_callback_is_called_for_every_page() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(5));
    let out_dir = dir.path().join("prog");

    let log: Arc<Mutex<Vec<PageProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = Arc::clone(&log);

    split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      move |p| {
        log_clone.lock().expect("mutex poisoned").push(p);
      },
    )
    .expect("split");

    let count = log.lock().expect("mutex").len();
    assert_eq!(count, 5, "callback should be called once per page");
  }

  #[test]
  fn split_pdf_progress_total_is_correct() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(4));
    let out_dir = dir.path().join("total");

    let log: Arc<Mutex<Vec<PageProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = Arc::clone(&log);

    split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      move |p| {
        log_clone.lock().expect("mutex").push(p);
      },
    )
    .expect("split");

    let events: Vec<PageProgress> = log.lock().expect("mutex").clone();
    for event in &events {
      assert_eq!(event.total, 4, "every progress event should report total=4");
    }
  }

  #[test]
  fn split_pdf_progress_current_values_cover_full_range() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(3));
    let out_dir = dir.path().join("range");

    let log: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = Arc::clone(&log);

    split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      move |p| {
        log_clone.lock().expect("mutex").push(p.current);
      },
    )
    .expect("split");

    let currents = log.lock().expect("mutex").clone();

    // Since processing is now sequential, events arrive in order
    assert_eq!(
      currents,
      vec![1, 2, 3],
      "current values 1..=total must appear in order"
    );
  }

  // Output page integrity

  #[test]
  fn split_pdf_each_output_is_a_single_page_pdf() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(3));
    let out_dir = dir.path().join("integrity");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    )
    .expect("split");

    for path in &result.output_files {
      let doc = Document::load(path).expect("output PDF should be loadable");
      assert_eq!(
        doc.get_pages().len(),
        1,
        "{path:?} should contain exactly 1 page"
      );
    }
  }

  #[test]
  fn split_result_elapsed_ms_is_accessible() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(2));
    let out_dir = dir.path().join("elapsed");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    )
    .expect("split");

    // elapsed_ms may be 0 on very fast machines; we just assert the field
    // is present and of the correct type.
    let _: u64 = result.elapsed_ms;
  }

  /// Build a valid PDF with zero pages — used to exercise the `NoPages` error path.
  fn make_empty_pdf() -> Vec<u8> {
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

  /// Build a PDF whose pages share a single font resource via their `/Resources`
  /// dictionaries.  Used to verify that the deep-copy logic correctly pulls in
  /// shared objects for each output page.
  fn make_pdf_with_shared_font(page_count: usize) -> Vec<u8> {
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

  /// Write arbitrary bytes to a temporary file inside `dir` and return its path.
  fn write_file(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("test helper: failed to write file");
    path
  }

  #[test]
  fn should_return_no_pages_when_get_page_count_on_empty_pdf() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_pdf(&dir, "empty.pdf", &make_empty_pdf());
    let result = get_page_count(&path);
    assert!(
      matches!(result, Err(PdfError::NoPages)),
      "expected NoPages, got: {result:?}"
    );
  }

  #[test]
  fn should_return_invalid_pdf_when_get_page_count_on_corrupt_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_file(&dir, "corrupt.pdf", b"not a valid PDF at all");
    let result = get_page_count(&path);
    assert!(
      matches!(result, Err(PdfError::InvalidPdf(_))),
      "expected InvalidPdf, got: {result:?}"
    );
  }

  #[test]
  fn should_return_no_pages_when_splitting_empty_pdf() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_empty_pdf());
    let out_dir = dir.path().join("none");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    );
    assert!(
      matches!(result, Err(PdfError::NoPages)),
      "expected NoPages, got: {result:?}"
    );
  }

  #[test]
  fn should_return_invalid_pdf_when_splitting_corrupt_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_file(&dir, "source.pdf", b"garbage bytes here");
    let out_dir = dir.path().join("bad");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    );
    assert!(
      matches!(result, Err(PdfError::InvalidPdf(_))),
      "expected InvalidPdf, got: {result:?}"
    );
  }

  #[test]
  fn should_succeed_when_output_directory_already_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(2));
    let out_dir = dir.path().join("exists");

    fs::create_dir_all(&out_dir).expect("pre-create output dir");

    split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    )
    .expect("split should succeed when output dir already exists");
  }

  #[test]
  fn should_serialize_split_result_as_camelcase_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(1));
    let out_dir = dir.path().join("serde");

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
      },
      |_| {},
    )
    .expect("split");

    let json = serde_json::to_string(&result).expect("serialisation failed");
    assert!(json.contains("\"totalPages\":"), "missing totalPages: {json}");
    assert!(json.contains("\"outputFiles\":"), "missing outputFiles: {json}");
    assert!(json.contains("\"elapsedMs\":"), "missing elapsedMs: {json}");
  }

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
    assert!(json.contains("\"fileName\":\"page_0003.pdf\""), "missing fileName: {json}");
  }

  #[test]
  fn should_emit_progress_with_file_names_matching_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_minimal_pdf(3));
    let out_dir = dir.path().join("names");

    let log: Arc<Mutex<Vec<PageProgress>>> = Arc::new(Mutex::new(Vec::new()));
    let log_clone = Arc::clone(&log);

    let result = split_pdf(
      SplitRequest {
        input_path: input,
        output_dir: out_dir,
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
      let path_in_result = result
        .output_files
        .iter()
        .find(|p| {
          p.file_name()
            .map(|n| n.to_string_lossy().as_ref() == event.file_name.as_str())
            .unwrap_or(false)
        });
      assert!(
        path_in_result.is_some(),
        "progress file name '{}' not found in output_files",
        event.file_name
      );
    }
  }

  #[test]
  fn should_preserve_shared_resources_in_output_pages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = write_pdf(&dir, "source.pdf", &make_pdf_with_shared_font(2));
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

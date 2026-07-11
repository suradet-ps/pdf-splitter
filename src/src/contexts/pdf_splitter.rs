//! Central application state and the actions that drive it.
//!
//! Mirrors the original `usePdfSplitter` composable: it owns the signals for
//! every view, exposes derived (`Memo`) values, and performs the async
//! transitions (`idle` → `ready` → `processing` → `complete` / `error`).  All
//! backend communication goes through [`services`]; this module never touches
//! `invoke` directly.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use leptos::prelude::*;

use crate::models::{
  basename, default_output_dir, format_bytes, format_duration, shorten_dir, AppState, PdfError,
  PdfFileInfo, SplitOperation, SplitResult,
};
use crate::services::commands::{
  get_file_info, pick_output_dir, pick_pdf_file, reveal_in_finder, split_pdf, subscribe_progress,
};
use crate::services::tauri::request_animation_frame;

/// Shared application state, exposed through Leptos context.
///
/// Every field is a plain `RwSignal`: the struct itself is `Copy`, so it can
/// be passed to child components by value (Leptos clones context cheaply).
#[derive(Clone, Copy, Debug)]
pub struct PdfSplitterContext {
  /// Current step in the application flow.
  pub state: RwSignal<AppState>,
  /// Metadata for the user-selected PDF (populated in `ready`).
  pub file_info: RwSignal<Option<PdfFileInfo>>,
  /// Resolved output directory path.
  pub output_dir: RwSignal<String>,
  /// Current operation snapshot (only meaningful during `processing`).
  pub operation: RwSignal<Option<SplitOperation>>,
  /// Result of the last successful split (only meaningful in `complete`).
  pub result: RwSignal<Option<SplitResult>>,
  /// Last error encountered (only meaningful in `error`).
  pub error: RwSignal<Option<PdfError>>,
  /// Whether an async operation is pending (disables interactive controls).
  pub is_busy: RwSignal<bool>,
}

impl PdfSplitterContext {
  /// Create the signals, register them in context, and return the handle.
  #[must_use]
  pub fn provide() -> Self {
    let ctx = Self {
      state: RwSignal::new(AppState::Idle),
      file_info: RwSignal::new(None),
      output_dir: RwSignal::new(String::new()),
      operation: RwSignal::new(None),
      result: RwSignal::new(None),
      error: RwSignal::new(None),
      is_busy: RwSignal::new(false),
    };
    provide_context(ctx);
    ctx
  }

  /// macOS-style title shown in the title bar.
  #[must_use]
  pub fn window_title(self) -> Memo<String> {
    Memo::new(move |_| match self.state.get() {
      AppState::Idle => "~/pdf-splitter".to_owned(),
      AppState::Ready => {
        format!(
          "~/pdf-splitter — {}",
          self.file_info.get().map_or_else(String::new, |f| f.name)
        )
      }
      AppState::Processing => format!("splitting… {}%", self.progress_percent().get()),
      AppState::Complete => {
        format!(
          "done — {} pages",
          self.result.get().map_or(0, |r| r.total_pages)
        )
      }
      AppState::Error => "~/pdf-splitter — error".to_owned(),
    })
  }

  /// Whether the wordmark / subtitle block should be visible.
  #[must_use]
  pub fn show_subtitle(self) -> Memo<bool> {
    Memo::new(move |_| matches!(self.state.get(), AppState::Idle | AppState::Ready))
  }

  /// Formatted file size, e.g. `"2.4 MB"`.
  #[must_use]
  pub fn file_size_formatted(self) -> Memo<String> {
    Memo::new(move |_| {
      self
        .file_info
        .get()
        .map_or_else(String::new, |f| format_bytes(f.size_bytes))
    })
  }

  /// Progress as a percentage (0–100), rounded to the nearest integer.
  #[must_use]
  pub fn progress_percent(self) -> Memo<i32> {
    Memo::new(move |_| {
      let p = self.operation.get().and_then(|o| o.progress);
      match p {
        Some(p) if p.total > 0 => ((p.current * 100) / p.total) as i32,
        _ => 0,
      }
    })
  }

  /// Formatted elapsed time for the result view.
  #[must_use]
  pub fn elapsed_formatted(self) -> Memo<String> {
    Memo::new(move |_| {
      self
        .result
        .get()
        .map_or_else(String::new, |r| format_duration(r.elapsed_ms))
    })
  }

  /// Short display form of the output directory path.
  #[must_use]
  pub fn output_dir_short(self) -> Memo<String> {
    Memo::new(move |_| shorten_dir(&self.output_dir.get()))
  }

  /// Open the native file-picker and load metadata for the chosen PDF.
  ///
  /// Transitions: `idle` / `ready` / `complete` / `error` → `ready`
  /// (a cancelled dialog leaves state unchanged).
  pub async fn pick_file(self) {
    if self.is_busy.get_untracked() {
      return;
    }
    self.is_busy.set(true);

    match pick_pdf_file().await {
      Ok(Some(path)) => match get_file_info(&path).await {
        Ok(info) => {
          self.file_info.set(Some(PdfFileInfo {
            path: path.clone(),
            name: basename(&path),
            size_bytes: info.size_bytes,
            page_count: info.page_count,
          }));
          self.output_dir.set(default_output_dir(&path));
          self.result.set(None);
          self.error.set(None);
          self.operation.set(None);
          self.state.set(AppState::Ready);
        }
        Err(e) => {
          self.error.set(Some(e));
          self.state.set(AppState::Error);
        }
      },
      // Cancelled — no state change.
      Ok(None) => {}
      Err(e) => {
        self.error.set(Some(e));
        self.state.set(AppState::Error);
      }
    }

    self.is_busy.set(false);
  }

  /// Open the native directory-picker and update `output_dir`.
  ///
  /// Only callable in the `ready` state.  A cancelled dialog keeps the
  /// current value.
  pub async fn pick_output_dir(self) {
    if self.is_busy.get_untracked() || self.state.get_untracked() != AppState::Ready {
      return;
    }
    self.is_busy.set(true);

    if let Ok(Some(dir)) = pick_output_dir().await {
      self.output_dir.set(dir);
    }

    self.is_busy.set(false);
  }

  /// Invoke the `split_pdf` command and stream live progress updates.
  ///
  /// Transitions: `ready` → `processing` → `complete` / `error`.  Progress
  /// events are buffered and flushed at most once per animation frame to
  /// avoid repainting faster than the display refresh rate.
  pub async fn start_split(self) {
    if self.is_busy.get_untracked()
      || self.state.get_untracked() != AppState::Ready
      || self.file_info.get_untracked().is_none()
    {
      return;
    }

    self.is_busy.set(true);
    self.error.set(None);

    // Non-reactive buffer for the latest progress payload, flushed once
    // per frame into the `operation` signal.  An `Rc<RefCell<_>>` is
    // required here because the callback and the rAF closure are both
    // `'static` and must share mutable state outside the reactive system.
    let buffer: Rc<RefCell<Option<crate::models::PageProgress>>> = Rc::new(RefCell::new(None));
    let raf_pending: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    if subscribe_progress({
      let ctx = self;
      let buffer = buffer.clone();
      let raf_pending = raf_pending.clone();
      move |p: crate::models::PageProgress| {
        *buffer.borrow_mut() = Some(p);
        if !raf_pending.get() {
          raf_pending.set(true);
          let ctx = ctx;
          let buffer = buffer.clone();
          let raf_pending = raf_pending.clone();
          request_animation_frame(move || {
            raf_pending.set(false);
            if let Some(p) = buffer.borrow_mut().take() {
              ctx.operation.update(|op| {
                if let Some(o) = op {
                  o.progress = Some(p);
                }
              });
            }
          });
        }
      }
    })
    .await
    .is_err()
    {
      self.error.set(Some(PdfError::from_raw(
        "Internal",
        "failed to listen for progress".to_owned(),
      )));
      self.state.set(AppState::Error);
      self.is_busy.set(false);
      return;
    }

    self.operation.set(Some(SplitOperation { progress: None }));
    self.state.set(AppState::Processing);

    let info = self
      .file_info
      .get_untracked()
      .expect("file_info present (checked above)");
    let split_result = split_pdf(&info.path, &self.output_dir.get()).await;

    match split_result {
      Ok(result) => {
        self.result.set(Some(result));
        self.state.set(AppState::Complete);
      }
      Err(e) => {
        self.error.set(Some(e));
        self.state.set(AppState::Error);
      }
    }

    self.operation.set(None);
    self.is_busy.set(false);
  }

  /// Reveal a path in the platform file manager.  Failures are ignored (e.g.
  /// the folder was deleted after splitting).
  pub async fn reveal_output(self, path: Option<String>) {
    let target = path
      .or_else(|| {
        self
          .result
          .get_untracked()
          .and_then(|r| r.output_files.into_iter().next())
      })
      .or_else(|| Some(self.output_dir.get_untracked()))
      .unwrap_or_default();
    if target.is_empty() {
      return;
    }
    let _ = reveal_in_finder(&target).await;
  }

  /// Reset the application to the initial `idle` state.
  pub async fn reset(self) {
    self.state.set(AppState::Idle);
    self.file_info.set(None);
    self.output_dir.set(String::new());
    self.operation.set(None);
    self.result.set(None);
    self.error.set(None);
    self.is_busy.set(false);
  }
}

/// Provide the [`PdfSplitterContext`] to the component tree.
#[must_use]
pub fn provide_pdf_splitter() -> PdfSplitterContext {
  PdfSplitterContext::provide()
}

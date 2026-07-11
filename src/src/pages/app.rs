//! Root application view.
//!
//! Mirrors `App.vue`: a title bar, an optional wordmark block, and a single
//! content region that swaps between the five state views (`idle`, `ready`,
//! `processing`, `complete`, `error`).  All async actions are wired here as
//! `spawn_local` calls into the [`PdfSplitterContext`]; child components only
//! receive props and callbacks.

use leptos::prelude::AnyView;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::{DropZone, ErrorView, FileCard, ProgressView, ResultView};
use crate::contexts::provide_pdf_splitter;
use crate::models::AppState;

/// Root component.
#[component]
pub fn App() -> impl IntoView {
  let ctx = provide_pdf_splitter();

  // ── Derived values ────────────────────────────────────────────────
  let window_title = ctx.window_title();
  let show_subtitle = ctx.show_subtitle();
  let file_size = ctx.file_size_formatted();
  let output_dir_short = ctx.output_dir_short();
  let output_dir = ctx.output_dir;
  let elapsed = ctx.elapsed_formatted();

  let progress_percent = ctx.progress_percent();
  let current = Memo::new(move |_| {
    ctx
      .operation
      .get()
      .and_then(|o| o.progress.map(|p| p.current))
      .unwrap_or(0)
  });
  let total = Memo::new(move |_| {
    ctx
      .operation
      .get()
      .and_then(|o| o.progress.map(|p| p.total))
      .unwrap_or(0)
  });
  let current_file = Memo::new(move |_| {
    ctx
      .operation
      .get()
      .and_then(|o| o.progress.map(|p| p.file_name))
      .unwrap_or_default()
  });
  let file_name = Memo::new(move |_| ctx.file_info.get().map_or_else(String::new, |f| f.name));

  let total_pages = Memo::new(move |_| ctx.result.get().map_or(0, |r| r.total_pages));
  let output_files = Memo::new(move |_| {
    ctx
      .result
      .get()
      .map_or_else(Vec::new, |r| r.output_files.clone())
  });
  let error_message = Memo::new(move |_| {
    ctx
      .error
      .get()
      .map_or_else(String::new, |e| e.message.clone())
  });
  let error_kind = Memo::new(move |_| ctx.error.get().map(|e| e.kind));

  // ── Action callbacks ──────────────────────────────────────────────
  let on_pick = Callback::new(move |_| {
    let ctx = ctx;
    spawn_local(async move { ctx.pick_file().await });
  });
  let on_drop = Callback::new(move |_: String| {
    let ctx = ctx;
    spawn_local(async move { ctx.pick_file().await });
  });

  let on_split = Callback::new(move |_| {
    let ctx = ctx;
    spawn_local(async move { ctx.start_split().await });
  });
  let on_change_file = on_pick;
  let on_change_output = Callback::new(move |_| {
    let ctx = ctx;
    spawn_local(async move { ctx.pick_output_dir().await });
  });

  let on_reveal = Callback::new(move |path: String| {
    let ctx = ctx;
    spawn_local(async move { ctx.reveal_output(Some(path)).await });
  });
  let on_reset = Callback::new(move |_| {
    let ctx = ctx;
    spawn_local(async move { ctx.reset().await });
  });
  let on_retry = Callback::new(move |_| {
    let ctx = ctx;
    spawn_local(async move {
      ctx.reset().await;
      ctx.pick_file().await;
    });
  });
  let on_dismiss = on_reset;

  // ── State-driven content ──────────────────────────────────────────
  //
  // Each state renders an outer `<div class="view view--X">`.  The branches
  // are unified through `AnyView` (Leptos' type-erased view), so the `match`
  // returns a single concrete type.
  let content = move || -> AnyView {
    match ctx.state.get() {
      AppState::Idle => view! {
          <div class="view view--idle">
              <DropZone busy=ctx.is_busy.into() on_pick=on_pick on_drop=on_drop/>
          </div>
      }
      .into_any(),
      AppState::Ready => {
        let inner: AnyView = match ctx.file_info.get() {
          Some(f) => view! {
              <FileCard
                  file_name=f.name.clone()
                  page_count=f.page_count
                  file_size_formatted=file_size.into()
                  output_dir_short=output_dir_short.into()
                  busy=ctx.is_busy.into()
                  on_split=on_split
                  on_change_file=on_change_file
                  on_change_output=on_change_output
              />
          }
          .into_any(),
          None => view! { <div class="view view--ready"></div> }.into_any(),
        };
        view! { <div class="view view--ready">{inner}</div> }.into_any()
      }
      AppState::Processing => view! {
          <div class="view view--processing">
              <div class="processing-card card">
                  <ProgressView
                      percent=progress_percent.into()
                      current=current.into()
                      total=total.into()
                      current_file=current_file.into()
                      file_name=file_name.into()
                  />
              </div>
          </div>
      }
      .into_any(),
      AppState::Complete => view! {
          <div class="view view--complete">
              <div class="result-wrapper">
                  <ResultView
                      total_pages=total_pages.get()
                      output_files=output_files.into()
                      elapsed_formatted=elapsed.into()
                      output_dir=output_dir.into()
                      on_reveal=on_reveal
                      on_reset=on_reset
                  />
              </div>
          </div>
      }
      .into_any(),
      AppState::Error => view! {
          <div class="view view--error">
              <div class="error-card card">
                  <ErrorView
                      message=error_message.get()
                      kind=error_kind.get()
                      on_retry=on_retry
                      on_dismiss=on_dismiss
                  />
              </div>
          </div>
      }
      .into_any(),
    }
  };

  view! {
      <div class="app" data-state=move || ctx.state.get().as_attr()>
          <div class="app__atmosphere" aria-hidden="true"></div>

          <header class="titlebar" data-tauri-drag-region="" aria-hidden="true">
              <div class="titlebar__traffic-lights" data-no-drag=""></div>
              <span class="titlebar__title" aria-live="polite">
                  {move || window_title.get()}
              </span>
          </header>

          <main class="app__main" role="main">
              <div
                  class="app__wordmark-wrapper"
                  class:app__wordmark-wrapper--hidden=move || !show_subtitle.get()
              >
                  <div class="app__wordmark">
                      <div class="app__heading-group">
                          <h1 class="app__title">"PDF Splitter"</h1>
                          <p class="app__subtitle">"Extract every page into its own file"</p>
                      </div>
                  </div>
              </div>

              <div class="app__content">{content}</div>
          </main>

          <footer class="app__footer" role="contentinfo">
              <span class="app__footer-text">"pdf-splitter · open source · MIT"</span>
          </footer>
      </div>
  }
}

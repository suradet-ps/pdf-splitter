//! File info card — shown in the `ready` state.
//!
//! Mirrors `FileCard.vue`: a document icon + filename + page count, an output
//! folder row with a "Change" button, and the primary "Split PDF" action.

use leptos::prelude::*;

/// Props for [`FileCard`].
#[component]
pub fn FileCard(
  /// Display name (basename) of the selected PDF.
  file_name: String,
  /// Number of pages in the document.
  page_count: u32,
  /// Human-readable file size (e.g. `"2.4 MB"`).
  file_size_formatted: Signal<String>,
  /// Short display form of the output directory path.
  output_dir_short: Signal<String>,
  /// Whether an async operation is in flight (disables interaction).
  busy: Signal<bool>,
  /// Emitted when the user confirms the split.
  on_split: Callback<()>,
  /// Emitted when the user wants to pick a different file.
  on_change_file: Callback<()>,
  /// Emitted when the user wants to change the output folder.
  on_change_output: Callback<()>,
) -> impl IntoView {
  let plural_pages = page_count == 1;
  let file_name = StoredValue::new(file_name);

  view! {
      <div class="file-card">
          <div class="file-info">
              <div class="file-icon" aria-hidden="true">
                  <svg
                      view_box="0 0 36 44"
                      fill="none"
                      xmlns="http://www.w3.org/2000/svg"
                      width="36"
                      height="44"
                  >
                      <rect x="0" y="0" width="28" height="40" rx="3" fill="currentColor" class="doc-body"></rect>
                      <path d="M20 0 L28 8 L20 8 Z" fill="currentColor" class="doc-fold"></path>
                      <line x1="20" y1="0" x2="28" y2="8" stroke="currentColor" stroke-width="0.5" class="doc-fold-line"></line>
                      <rect x="0" y="28" width="28" height="12" rx="2" fill="currentColor" class="doc-band"></rect>
                      <rect x="3" y="32" width="5" height="1.6" rx="0.8" fill="currentColor" class="doc-label"></rect>
                      <rect x="10" y="32" width="4" height="1.6" rx="0.8" fill="currentColor" class="doc-label"></rect>
                      <rect x="16" y="32" width="4" height="1.6" rx="0.8" fill="currentColor" class="doc-label"></rect>
                      <rect x="4" y="9" width="16" height="1.4" rx="0.7" fill="currentColor" class="doc-line"></rect>
                      <rect x="4" y="13" width="13" height="1.4" rx="0.7" fill="currentColor" class="doc-line"></rect>
                      <rect x="4" y="17" width="15" height="1.4" rx="0.7" fill="currentColor" class="doc-line"></rect>
                      <rect x="4" y="21" width="10" height="1.4" rx="0.7" fill="currentColor" class="doc-line"></rect>
                  </svg>
              </div>

              <div class="file-meta">
                  <span class="file-name truncate" title=move || file_name.get_value()>
                      {move || file_name.get_value()}
                  </span>
                  <div class="file-details">
                      <span class="file-detail">
                          <span class="file-detail__val">{page_count}</span>
                          <span class="file-detail__key">
                              {if plural_pages { "page" } else { "pages" }}
                          </span>
                      </span>
                      <Show when=move || !file_size_formatted.get().is_empty()>
                          <span class="file-detail__sep" aria-hidden="true">"·"</span>
                          <span class="file-detail">
                              <span class="file-detail__val">{move || file_size_formatted.get()}</span>
                          </span>
                      </Show>
                  </div>
              </div>

              <button
                  type="button"
                  class="btn-icon dismiss-btn"
                  disabled=move || busy.get()
                  aria-label=move || format!("Remove {}", file_name.get_value())
                  title="Choose a different file"
                  on:click=move |_| on_change_file.run(())
              >
                  <svg
                      view_box="0 0 20 20"
                      fill="none"
                      xmlns="http://www.w3.org/2000/svg"
                      class="icon-md"
                      aria-hidden="true"
                  >
                      <path
                          fill_rule="evenodd"
                          clip_rule="evenodd"
                          d="M4.293 4.293a1 1 0 0 1 1.414 0L10 8.586l4.293-4.293a1 1 0 1 1 1.414 1.414L11.414 10l4.293 4.293a1 1 0 0 1-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 0 1-1.414-1.414L8.586 10 4.293 5.707a1 1 0 0 1 0-1.414Z"
                          fill="currentColor"
                      ></path>
                  </svg>
              </button>
          </div>

          <div class="separator" role="separator"></div>

          <div class="output-row">
              <div class="output-row__info">
                  <span class="output-row__label">"Output folder"</span>
                  <span class="output-row__path truncate" title=move || output_dir_short.get()>
                      {move || {
                          let s = output_dir_short.get();
                          if s.is_empty() {
                              "Same folder as PDF".to_owned()
                          } else {
                              s
                          }
                      }}
                  </span>
              </div>
              <button
                  type="button"
                  class="output-change-btn"
                  disabled=move || busy.get()
                  on:click=move |_| on_change_output.run(())
              >
                  "Change"
              </button>
          </div>

          <div class="separator" role="separator"></div>

          <div class="action-row">
              <span class="page-badge" aria-label=format!("{page_count} pages")>
                  <svg
                      view_box="0 0 12 14"
                      fill="none"
                      xmlns="http://www.w3.org/2000/svg"
                      width="10"
                      height="12"
                      aria-hidden="true"
                  >
                      <path
                          d="M1.5 1A.5.5 0 0 1 2 .5h6.086a.5.5 0 0 1 .353.146l2.414 2.415A.5.5 0 0 1 11 3.414V13a.5.5 0 0 1-.5.5h-8A.5.5 0 0 1 1.5 13V1Z"
                          fill="currentColor"
                          opacity="0.6"
                      ></path>
                  </svg>
                  {page_count}
                  {" "}
                  {if plural_pages { "page" } else { "pages" }}
              </span>

              <button
                  type="button"
                  class="btn-primary btn-glow split-btn"
                  disabled=move || busy.get()
                  on:click=move |_| on_split.run(())
              >
                  <Show
                      when=move || busy.get()
                      fallback=|| {
                          view! {
                              "Split PDF"
                              <svg
                                  view_box="0 0 16 16"
                                  fill="none"
                                  xmlns="http://www.w3.org/2000/svg"
                                  width="14"
                                  height="14"
                                  aria-hidden="true"
                                  class="split-btn__arrow"
                              >
                                  <path
                                      fill_rule="evenodd"
                                      clip_rule="evenodd"
                                      d="M3.75 8a.75.75 0 0 1 .75-.75h5.19L7.22 4.78a.75.75 0 0 1 1.06-1.06l3.5 3.5a.75.75 0 0 1 0 1.06l-3.5 3.5a.75.75 0 0 1-1.06-1.06l2.47-2.47H4.5A.75.75 0 0 1 3.75 8Z"
                                      fill="currentColor"
                                  ></path>
                              </svg>
                          }
                      }
                  >
                      <span class="animate-pulse">"Splitting…"</span>
                  </Show>
              </button>
          </div>
      </div>
  }
}

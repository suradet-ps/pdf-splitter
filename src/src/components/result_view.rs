//! Result view — shown in the `complete` state.
//!
//! Mirrors `ResultView.vue`: a success badge, summary, output folder header,
//! and a scrollable list of the produced files (each with a hover "reveal"
//! button), plus "Split another" / "Open folder" actions.

use leptos::prelude::*;
use web_sys::MouseEvent;

/// Props for [`ResultView`].
#[component]
pub fn ResultView(
  /// Number of pages / output files produced.
  total_pages: u32,
  /// Absolute paths of every output file.
  output_files: Signal<Vec<String>>,
  /// Human-readable elapsed time.
  elapsed_formatted: Signal<String>,
  /// Absolute path of the output directory.
  output_dir: Signal<String>,
  /// Emitted with a file path when the user wants to reveal it in Finder.
  on_reveal: Callback<String>,
  /// Emitted when the user wants to split another file.
  on_reset: Callback<()>,
) -> impl IntoView {
  let hovered = RwSignal::new(None::<usize>);

  let summary_label = Memo::new(move |_| {
    let pages = total_pages;
    let unit = if pages == 1 { "file" } else { "files" };
    match elapsed_formatted.get().is_empty() {
      true => format!("{pages} {unit}"),
      false => format!("{} {} · {}", pages, unit, elapsed_formatted.get()),
    }
  });

  let single_file = total_pages == 1;

  view! {
      <div class="result-view">
          <div class="result-header">
              <div class="success-badge animate-bounce-in" aria-hidden="true">
                  <svg view_box="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg" width="32" height="32">
                      <circle cx="16" cy="16" r="15" fill="currentColor" class="badge-circle"></circle>
                      <path
                          fill_rule="evenodd"
                          clip_rule="evenodd"
                          d="M22.78 10.72a.75.75 0 0 1 0 1.06l-8.5 8.5a.75.75 0 0 1-1.06 0l-3.5-3.5a.75.75 0 1 1 1.06-1.06l2.97 2.97 7.97-7.97a.75.75 0 0 1 1.06 0Z"
                          fill="currentColor"
                          class="badge-check"
                      ></path>
                  </svg>
              </div>

              <div class="result-header__text">
                  <h2 class="result-header__title">"Done"</h2>
                  <p class="result-header__summary">{move || summary_label.get()}</p>
              </div>

              <div class="result-stats" aria-label="Split statistics">
                  <span class="stat-chip stat-chip--files">
                      <svg view_box="0 0 12 14" fill="none" xmlns="http://www.w3.org/2000/svg" width="10" height="12" aria-hidden="true">
                          <path d="M1.5 1A.5.5 0 0 1 2 .5h6.086a.5.5 0 0 1 .353.146l2.414 2.415A.5.5 0 0 1 11 3.414V13a.5.5 0 0 1-.5.5h-8A.5.5 0 0 1 1.5 13V1Z" fill="currentColor" opacity="0.7"></path>
                      </svg>
                      {total_pages}
                      {" "}
                      {if single_file { "file" } else { "files" }}
                  </span>
                  <Show when=move || !elapsed_formatted.get().is_empty()>
                      <span class="stat-chip stat-chip--time">
                          <svg view_box="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg" width="10" height="10" aria-hidden="true">
                              <circle cx="6" cy="6" r="5" stroke="currentColor" stroke-width="1.3" fill="none"></circle>
                              <path d="M6 3.5v2.5l1.5 1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"></path>
                          </svg>
                          {move || elapsed_formatted.get()}
                      </span>
                  </Show>
              </div>
          </div>

          <div class="file-list-container">
              <div class="file-list__header">
                  <svg view_box="0 0 14 12" fill="none" xmlns="http://www.w3.org/2000/svg" width="12" height="10" aria-hidden="true">
                      <path
                          fill_rule="evenodd"
                          clip_rule="evenodd"
                          d="M1 1.5A.5.5 0 0 1 1.5 1h3.379a.5.5 0 0 1 .353.146l.768.768A.5.5 0 0 0 6.353 2.1H12.5a.5.5 0 0 1 .5.5v8a.5.5 0 0 1-.5.5h-11A.5.5 0 0 1 1 10.6V1.5Z"
                          fill="currentColor"
                          opacity="0.6"
                      ></path>
                  </svg>
                  <span class="file-list__header-label">"Output folder"</span>
                  <span class="file-list__header-path truncate">{move || output_dir.get()}</span>
              </div>

              <div class="file-list" role="list" aria-label=format!("{} output files", total_pages)>
                  <For
                      each=move || {
                          output_files
                              .get()
                              .into_iter()
                              .enumerate()
                              .collect::<Vec<(usize, String)>>()
                      }
                      key=|(_, p)| p.clone()
                      children=move |(index, path): (usize, String)| {
                          let name = StoredValue::new(crate::models::basename(&path));
                          let path_sv = StoredValue::new(path);
                          let delay = (index * 14).min(280);
                          let is_hovered = Memo::new(move |_| hovered.get() == Some(index));
                          let reveal = on_reveal;
                          view! {
                              <div
                                  class="file-row"
                                  role="listitem"
                                  style=format!("transition-delay:{delay}ms")
                                  on:mouseenter=move |_| hovered.set(Some(index))
                                  on:mouseleave=move |_| hovered.set(None)
                              >
                                  <span class="file-row__lineno" aria-hidden="true">
                                      {format!("{:>3}", index + 1)}
                                  </span>
                                  <span class="file-row__name">{move || name.get_value()}</span>
                                  <Show when=move || is_hovered.get()>
                                      <button
                                          type="button"
                                          class="file-row__reveal"
                                          aria-label=move || format!("Reveal {} in Finder", name.get_value())
                                          on:click=move |ev: MouseEvent| {
                                              ev.stop_propagation();
                                              reveal.run(path_sv.get_value());
                                          }
                                      >
                                          <svg view_box="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg" width="11" height="11" aria-hidden="true">
                                              <path
                                                  fill_rule="evenodd"
                                                  clip_rule="evenodd"
                                                  d="M2 2.5A.5.5 0 0 1 2.5 2h9a.5.5 0 0 1 .5.5v9a.5.5 0 0 1-.5.5H2.5a.5.5 0 0 1-.5-.5v-9ZM3 3v8h8V3H3ZM7 4.5a.5.5 0 0 1 .5.5v2h2a.5.5 0 0 1 0 1h-2v2a.5.5 0 0 1-1 0v-2h-2a.5.5 0 0 1 0-1h2V5a.5.5 0 0 1 .5-.5Z"
                                                  fill="currentColor"
                                              ></path>
                                          </svg>
                                          "reveal"
                                      </button>
                                  </Show>
                              </div>
                          }
                      }
                  />
              </div>
          </div>

          <div class="result-actions">
              <button
                  type="button"
                  class="btn-ghost result-actions__secondary"
                  on:click=move |_| on_reset.run(())
              >
                  <svg view_box="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true">
                      <path
                          fill_rule="evenodd"
                          clip_rule="evenodd"
                          d="M3.5 8a4.5 4.5 0 0 1 7.854-3H9.25a.75.75 0 0 0 0 1.5h3.25a.75.75 0 0 0 .75-.75V2.5a.75.75 0 0 0-1.5 0v1.386A6 6 0 1 0 14 8a.75.75 0 0 0-1.5 0A4.5 4.5 0 1 1 3.5 8Z"
                          fill="currentColor"
                      ></path>
                  </svg>
                  "Split another"
              </button>

              <button
                  type="button"
                  class="btn-primary btn-glow result-actions__primary"
                  on:click=move |_| {
                      let dir = output_dir.get();
                      on_reveal.run(dir);
                  }
              >
                  <svg view_box="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true">
                      <path
                          fill_rule="evenodd"
                          clip_rule="evenodd"
                          d="M1.5 3.5A1.5 1.5 0 0 1 3 2h2.629A1.5 1.5 0 0 1 6.69 2.44L7.81 3.56A.5.5 0 0 0 8.164 3.7H13a1.5 1.5 0 0 1 1.5 1.5v7A1.5 1.5 0 0 1 13 13.7H3A1.5 1.5 0 0 1 1.5 12.2v-8.7Z"
                          fill="currentColor"
                          opacity="0.85"
                      ></path>
                  </svg>
                  "Open folder"
              </button>
          </div>
      </div>
  }
}

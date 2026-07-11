//! Progress view — shown in the `processing` state.
//!
//! Mirrors `ProgressView.vue`: a status indicator, title, percent badge, a
//! progress track + fill, step dots, and a live output stream line.

use leptos::prelude::*;

/// Maximum number of step dots rendered (large documents are sampled).
const MAX_DOTS: u32 = 20;

/// Props for [`ProgressView`].
#[component]
pub fn ProgressView(
  /// Progress percentage (0–100).
  percent: Signal<i32>,
  /// Pages completed so far (1-based).
  current: Signal<u32>,
  /// Total pages to process.
  total: Signal<u32>,
  /// Filename of the most-recently written output page.
  current_file: Signal<String>,
  /// Basename of the source PDF file.
  file_name: Signal<String>,
) -> impl IntoView {
  let clamped = Memo::new(move |_| percent.get().clamp(0, 100));
  let is_starting = Memo::new(move |_| current.get() == 0);
  let is_finalising = Memo::new(move |_| clamped.get() >= 100 && !is_starting.get());

  let fill_transform = Memo::new(move |_| format!("scaleX({})", clamped.get() as f32 / 100.0));
  let fraction_label = Memo::new(move |_| {
    if total.get() > 0 {
      format!("{} / {}", current.get(), total.get())
    } else {
      "…".to_owned()
    }
  });

  let dot_count = Memo::new(move |_| {
    let t = total.get();
    if t == 0 {
      0
    } else {
      t.min(MAX_DOTS)
    }
  });
  let dots = Memo::new(move |_| (0..dot_count.get()).collect::<Vec<u32>>());

  view! {
      <div class="progress-view" role="status" aria-live="polite" aria-label="Splitting PDF…">
          <div class="progress-view__header">
              <div class="progress-view__indicator" aria-hidden="true">
                  <Show
                      when=move || is_finalising.get()
                      fallback=|| {
                          view! {
                              <svg class="progress-view__spinner animate-spin" view_box="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" width="24" height="24">
                                  <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.5" opacity="0.15"></circle>
                                  <path d="M12 2a10 10 0 0 1 10 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"></path>
                              </svg>
                          }
                      }
                  >
                      <svg view_box="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" width="24" height="24">
                          <circle cx="12" cy="12" r="11" stroke="currentColor" stroke-width="1.5" opacity="0.3"></circle>
                          <path
                              fill_rule="evenodd"
                              clip_rule="evenodd"
                              d="M17.09 8.47a.75.75 0 0 1 0 1.06l-5.5 5.5a.75.75 0 0 1-1.06 0l-2.5-2.5a.75.75 0 1 1 1.06-1.06l1.97 1.97 4.97-4.97a.75.75 0 0 1 1.06 0Z"
                              fill="currentColor"
                          ></path>
                      </svg>
                  </Show>
              </div>

              <div class="progress-view__title-group">
                  <h2 class="progress-view__title">
                      <Show when=move || is_finalising.get() fallback=|| "Splitting PDF">
                          "Finalising…"
                      </Show>
                  </h2>
                  <p class="progress-view__filename truncate" title=move || file_name.get()>
                      {move || file_name.get()}
                  </p>
                  <p class="progress-view__status">
                      <Show
                          when=move || is_starting.get()
                          fallback=move || {
                              view! {
                                  <Show
                                      when=move || is_finalising.get()
                                      fallback=move || {
                                          view! {
                                              <span>
                                                  "Processing page "
                                                  <strong>{move || current.get()}</strong>
                                                  " of "
                                                  <strong>{move || total.get()}</strong>
                                              </span>
                                          }
                                      }
                                  >
                                      <span>"Writing output files…"</span>
                                  </Show>
                              }
                          }
                      >
                          <span class="animate-pulse">"Preparing pages…"</span>
                      </Show>
                  </p>
              </div>

              <div
                  class="progress-view__pct"
                  class:progress-view__pct--finalising=move || is_finalising.get()
                  aria-hidden="true"
              >
                  <span class="progress-view__pct-number">
                      <Show when=move || is_starting.get() fallback=move || clamped.get().to_string()>
                          "0"
                      </Show>
                  </span>
                  <span class="progress-view__pct-unit">"%"</span>
              </div>
          </div>

          <div class="progress-section">
              <div class="progress-section__label" aria-hidden="true">
                  <span class="progress-section__fraction">{move || fraction_label.get()}</span>
                  <span class="progress-section__unit">"pages"</span>
              </div>

              <div
                  class="progress-track"
                  role="progressbar"
                  aria-valuenow=move || clamped.get()
                  aria-valuemin="0"
                  aria-valuemax="100"
                  aria-label=move || format!("{}% complete", clamped.get())
              >
                  <div class="progress-fill" style=move || format!("transform:{}", fill_transform.get())></div>
              </div>

              <Show when=move || { dot_count.get() >= 2 }>
                  <div class="step-dots" aria-hidden="true">
                      <For
                          each=move || dots.get()
                          key=|i| *i
                          children=move |i: u32| {
                              let total = total.get();
                              let count = dot_count.get().max(1);
                              // The i-th dot is "done" once the running page count
                              // reaches the threshold that distributes `total`
                              // pages across `count` dots.
                              let threshold = if total > 0 {
                                  (((i + 1) * total).div_ceil(count)).min(total)
                              } else {
                                  0
                              };
                              let done = current.get() >= threshold;
                              view! { <span class="step-dot" class:step-dot--done=move || done></span> }
                          }
                      />
                  </div>
              </Show>
          </div>

          <div class="output-stream">
              <Show
                  when=move || !current_file.get().is_empty()
                  fallback=move || {
                      view! {
                          <Show when=move || is_starting.get()>
                              <div class="output-line output-line--pending">
                                  <span class="output-line__dot animate-pulse" aria-hidden="true"></span>
                                  <span class="output-line__text">"Waiting for first page…"</span>
                              </div>
                          </Show>
                      }
                  }
              >
                  <div class="output-line output-line--ok">
                      <svg
                          view_box="0 0 14 14"
                          fill="none"
                          xmlns="http://www.w3.org/2000/svg"
                          width="12"
                          height="12"
                          aria-hidden="true"
                          class="output-line__check-icon"
                      >
                          <path
                              fill_rule="evenodd"
                              clip_rule="evenodd"
                              d="M11.78 3.22a.75.75 0 0 1 0 1.06l-5.5 5.5a.75.75 0 0 1-1.06 0L2.72 7.28a.75.75 0 1 1 1.06-1.06l2.47 2.47 4.97-4.97a.75.75 0 0 1 1.06 0Z"
                              fill="currentColor"
                          ></path>
                      </svg>
                      <span class="output-line__text truncate">{move || current_file.get()}</span>
                  </div>
              </Show>
          </div>
      </div>
  }
}

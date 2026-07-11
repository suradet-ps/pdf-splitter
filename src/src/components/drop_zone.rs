//! Drop-zone view — shown in the `idle` state.
//!
//! Mirrors `DropZone.vue`: a dashed drop area that opens the file picker on
//! click / keyboard activation, and on drag-and-drop.  Per-frame repaints are
//! not a concern here (no high-frequency updates), so local `RwSignal` state is
//! sufficient.

use js_sys::Reflect;
use leptos::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::{DragEvent, KeyboardEvent};

/// Read the (Tauri-specific) `path` property off a dropped `File`, if present.
fn file_path(file: &web_sys::File) -> Option<String> {
  let path = Reflect::get(file, &JsValue::from_str("path")).ok()?;
  path.as_string()
}

/// Props for [`DropZone`].
#[component]
pub fn DropZone(
  /// Whether an async operation is in flight (disables interaction).
  busy: Signal<bool>,
  /// Emitted when the user clicks / activates the zone (opens the picker).
  on_pick: Callback<()>,
  /// Emitted when a PDF is dropped onto the zone.
  on_drop: Callback<String>,
) -> impl IntoView {
  let is_drag_over = RwSignal::new(false);
  let drag_counter = StoredValue::new(0i32);

  let on_drag_enter = move |ev: DragEvent| {
    ev.prevent_default();
    drag_counter.set_value(drag_counter.get_value() + 1);
    is_drag_over.set(true);
  };

  let on_drag_leave = move |ev: DragEvent| {
    ev.prevent_default();
    let next = drag_counter.get_value() - 1;
    drag_counter.set_value(next.max(0));
    if drag_counter.get_value() == 0 {
      is_drag_over.set(false);
    }
  };

  let on_drag_over = move |ev: DragEvent| {
    ev.prevent_default();
    if let Some(dt) = ev.data_transfer() {
      dt.set_drop_effect("copy");
    }
  };

  let on_drop = move |ev: DragEvent| {
    ev.prevent_default();
    drag_counter.set_value(0);
    is_drag_over.set(false);

    if busy.get() {
      return;
    }

    let Some(files) = ev.data_transfer().and_then(|dt| dt.files()) else {
      return;
    };
    if files.length() == 0 {
      return;
    }
    let Some(file) = files.get(0) else {
      return;
    };
    let is_pdf = file.type_() == "application/pdf" || file.name().to_lowercase().ends_with(".pdf");
    if !is_pdf {
      return;
    }
    if let Some(path) = file_path(&file) {
      on_drop.run(path);
    }
  };

  let select = move || {
    if !busy.get() {
      on_pick.run(());
    }
  };

  view! {
      <div class="drop-zone-wrapper">
          <div
              class="drop-zone"
              class:drop-zone--active=move || is_drag_over.get() && !busy.get()
              class:drop-zone--busy=move || busy.get()
              role="button"
              tabindex="0"
              aria-label="Drop a PDF file here or press Space to open the file picker"
              on:dragenter=on_drag_enter
              on:dragleave=on_drag_leave
              on:dragover=on_drag_over
              on:drop=on_drop
              on:click=move |_| select()
              on:keydown=move |ev: KeyboardEvent| {
                  if ev.key() == " " || ev.key() == "Enter" {
                      ev.prevent_default();
                      select();
                  }
              }
          >
              <div class="drop-zone__glow" aria-hidden="true"></div>

              <div class="drop-zone__content">
                  <div
                      class="drop-zone__icon"
                      class:animate-float=move || !is_drag_over.get() && !busy.get()
                      aria-hidden="true"
                  >
                      <svg
                          xmlns="http://www.w3.org/2000/svg"
                          width="62"
                          height="62"
                          view_box="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke_width="1.2"
                          stroke_linecap="round"
                          stroke_linejoin="round"
                      >
                          <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"></path>
                          <path d="M14 2v4a2 2 0 0 0 2 2h4"></path>
                          <path d="M12 12v6"></path>
                          <path d="m15 15-3-3-3 3"></path>
                      </svg>
                  </div>

                  <div class="drop-zone__labels">
                      <Show
                          when=move || is_drag_over.get() && !busy.get()
                          fallback=move || {
                              view! {
                                  <Show
                                      when=move || busy.get()
                                      fallback=|| {
                                          view! {
                                              <span class="drop-zone__heading">"Drop your PDF here"</span>
                                              <span class="drop-zone__subtext">
                                                  "or select a file from your computer"
                                              </span>
                                          }
                                      }
                                  >
                                      <span class="drop-zone__heading animate-pulse">"Loading…"</span>
                                  </Show>
                              }
                          }
                      >
                          <span class="drop-zone__heading drop-zone__heading--active">
                              "Release to load"
                          </span>
                      </Show>
                  </div>

                  <Show when=move || !is_drag_over.get()>
                      <div class="drop-zone__actions">
                          <button
                              type="button"
                              class="btn-primary drop-zone__btn"
                              disabled=move || busy.get()
                              tabindex="-1"
                              aria-hidden="true"
                              on:click=move |_| select()
                          >
                              "Select file"
                              <svg
                                  view_box="0 0 16 16"
                                  fill="none"
                                  xmlns="http://www.w3.org/2000/svg"
                                  width="14"
                                  height="14"
                                  aria-hidden="true"
                              >
                                  <path
                                      fill_rule="evenodd"
                                      clip_rule="evenodd"
                                      d="M3.75 8a.75.75 0 0 1 .75-.75h5.19L7.22 4.78a.75.75 0 0 1 1.06-1.06l3.5 3.5a.75.75 0 0 1 0 1.06l-3.5 3.5a.75.75 0 0 1-1.06-1.06l2.47-2.47H4.5A.75.75 0 0 1 3.75 8Z"
                                      fill="currentColor"
                                  ></path>
                              </svg>
                          </button>
                      </div>
                  </Show>
              </div>
          </div>

          <p class="drop-zone__hint" aria-live="polite">
              <Show when=move || is_drag_over.get() && !busy.get()>
                  "PDF files only"
              </Show>
              <Show when=move || !is_drag_over.get() || busy.get()>
                  "Accepts all PDF versions · any number of pages"
              </Show>
          </p>
      </div>
  }
}

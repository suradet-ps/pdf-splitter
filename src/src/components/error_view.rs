//! Error view — shown in the `error` state.
//!
//! Mirrors `ErrorView.vue`: an error icon, a kind-specific title + hint, the
//! raw message in a selectable block, and "Dismiss" / "Try again" actions.

use leptos::prelude::*;

use crate::models::PdfErrorKind;

/// Props for [`ErrorView`].
#[component]
pub fn ErrorView(
  /// Human-readable error message from the backend.
  message: String,
  /// Machine-readable error discriminant (drives title + hint copy).
  kind: Option<PdfErrorKind>,
  /// Emitted when the user wants to retry.
  on_retry: Callback<()>,
  /// Emitted when the user wants to dismiss and return to idle.
  on_dismiss: Callback<()>,
) -> impl IntoView {
  let kind_label = match kind {
    Some(PdfErrorKind::FileNotFound) => "File Not Found",
    Some(PdfErrorKind::InvalidPdf) => "Invalid PDF",
    Some(PdfErrorKind::NoPages) => "No Pages Found",
    Some(PdfErrorKind::Io) => "IO Error",
    Some(PdfErrorKind::Internal) => "Internal Error",
    None => "Something went wrong",
  };

  let hint = match kind {
        Some(PdfErrorKind::FileNotFound) => {
            "The file may have been moved, renamed, or deleted. Please select a valid PDF file."
        }
        Some(PdfErrorKind::InvalidPdf) => {
            "The selected file could not be parsed as a PDF. Make sure the file is not corrupted or password-protected."
        }
        Some(PdfErrorKind::NoPages) => {
            "The PDF document appears to have no pages. Please select a different file."
        }
        Some(PdfErrorKind::Io) => {
            "A filesystem error occurred. Check that the output directory is accessible and that you have write permissions."
        }
        Some(PdfErrorKind::Internal) => {
            "An unexpected internal error occurred. Please try again or report the issue if it persists."
        }
        None => "Please try again with a different file or output directory.",
    };

  view! {
      <div class="error-view" role="alert" aria-live="assertive">
          <div class="error-header">
              <div class="error-icon" aria-hidden="true">
                  <svg view_box="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg" width="32" height="32">
                      <circle cx="16" cy="16" r="15" fill="currentColor" class="error-icon__circle"></circle>
                      <path
                          fill_rule="evenodd"
                          clip_rule="evenodd"
                          d="M11.293 11.293a1 1 0 0 1 1.414 0L16 14.586l3.293-3.293a1 1 0 1 1 1.414 1.414L17.414 16l3.293 3.293a1 1 0 0 1-1.414 1.414L16 17.414l-3.293 3.293a1 1 0 0 1-1.414-1.414L14.586 16l-3.293-3.293a1 1 0 0 1 0-1.414Z"
                          fill="currentColor"
                          class="error-icon__x"
                      ></path>
                  </svg>
              </div>

              <div class="error-header__text">
                  <h2 class="error-header__title">{kind_label}</h2>
                  <p class="error-header__subtitle">"Something went wrong"</p>
              </div>
          </div>

          <div class="error-block">
              <div class="error-block__label" aria-hidden="true">
                  <span class="error-block__label-text">"Error details"</span>
              </div>
              <p class="error-message" data-selectable="">
                  <span class="error-message__prefix" aria-hidden="true">"error:"</span>
                  {message}
              </p>
          </div>

          <div class="error-note">
              <svg view_box="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true" class="error-note__icon">
                  <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.2" fill="none"></circle>
                  <path d="M8 7v4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"></path>
                  <circle cx="8" cy="5" r="0.75" fill="currentColor"></circle>
              </svg>
              <p class="error-note__text">{hint}</p>
          </div>

          <div class="separator" role="separator"></div>

          <div class="error-actions">
              <button
                  type="button"
                  class="btn-ghost error-actions__dismiss"
                  on:click=move |_| on_dismiss.run(())
              >
                  "Dismiss"
              </button>
              <button
                  type="button"
                  class="btn-primary btn-glow error-actions__retry"
                  on:click=move |_| on_retry.run(())
              >
                  <svg view_box="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true">
                      <path
                          fill_rule="evenodd"
                          clip_rule="evenodd"
                          d="M3.5 8a4.5 4.5 0 0 1 7.854-3H9.25a.75.75 0 0 0 0 1.5h3.25a.75.75 0 0 0 .75-.75V2.5a.75.75 0 0 0-1.5 0v1.386A6 6 0 1 0 14 8a.75.75 0 0 0-1.5 0A4.5 4.5 0 1 1 3.5 8Z"
                          fill="currentColor"
                      ></path>
                  </svg>
                  "Try again"
              </button>
          </div>
      </div>
  }
}

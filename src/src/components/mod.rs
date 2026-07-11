//! Presentational Leptos components.
//!
//! Each module maps 1:1 from a former Vue Single File Component.  Components
//! own only local UI state (drag hover, hovered row); all application state
//! and async actions arrive via props / callbacks from [`crate::pages::App`].

mod drop_zone;
mod error_view;
mod file_card;
mod progress_view;
mod result_view;

pub use drop_zone::DropZone;
pub use error_view::ErrorView;
pub use file_card::FileCard;
pub use progress_view::ProgressView;
pub use result_view::ResultView;

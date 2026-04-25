//! PDF processing pipeline.

pub mod error;
pub mod splitter;

pub use error::PdfError;
pub use splitter::{PageProgress, SplitRequest, SplitResult, get_page_count, split_pdf};

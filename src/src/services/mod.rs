//! Tauri IPC helpers — the only place that talks to the backend.
//!
//! Components and contexts must never call `invoke` directly; they go through
//! [`commands`] which returns domain types and converts failures into
//! user-facing [`models::PdfError`] values.

pub mod commands;
pub mod tauri;

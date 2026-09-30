#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Minimal, deterministic, headless HTML/CSS-to-PDF layout engine.
//!
//! Targets report-generation use cases. Parses a restricted HTML/CSS subset
//! and delegates final PDF output to `tpt-doc-pdf`.

/// Inline CSS parsing.
pub mod css;
/// Minimal HTML document model and parsing.
pub mod html;
/// Block layout rendering to page content.
pub mod render;

pub use render::{PageKind, Renderer, approx_text_width};

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Minimal, deterministic, headless HTML/CSS-to-PDF layout engine.
//!
//! Targets report-generation use cases. Parses a restricted HTML/CSS subset
//! and delegates final PDF output to `tpt-doc-pdf`.

pub mod css;
pub mod html;
pub mod render;

pub use render::Renderer;

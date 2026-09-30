#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Pure-Rust PDF 1.7/2.0 generation and parsing.
//!
//! No pdfium, no C-FFI. Documents are built as a declarative model and
//! serialized with precise byte-offset cross-reference tables.

/// The declarative PDF document model.
pub mod document;
/// Font handling and built-in Type1 fonts.
pub mod font;
/// Raster image embedding (JPEG passthrough, PNG transcoding).
pub mod image;
/// Page geometry and content streams.
pub mod page;
/// Cross-reference table construction and serialization.
pub mod xref;

pub use document::Document;
pub use font::Font;
pub use image::Image;
pub use page::Page;

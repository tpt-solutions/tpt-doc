//! # Examples
//!
//! ```
//! use tpt_doc_pdf::{Document, Font, Page};
//!
//! let mut doc = Document::new();
//! let font = doc.embed_builtin_font(Font::Helvetica);
//! let mut page = Page::a4();
//! page.text("Hello, PDF!", font, 12.0, (72.0, 700.0));
//! doc.add_page(page);
//! let bytes = doc.write()?;
//! assert!(bytes.starts_with(b"%PDF-1.7"));
//! # Ok::<(), tpt_doc_core::DocError>(())
//! ```
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

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Pure-Rust PDF 1.7/2.0 generation and parsing.
//!
//! No pdfium, no C-FFI. Documents are built as a declarative model and
//! serialized with precise byte-offset cross-reference tables.

pub mod document;
pub mod font;
pub mod page;
pub mod xref;

pub use document::Document;
pub use font::Font;
pub use page::Page;

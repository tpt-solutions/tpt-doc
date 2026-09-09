#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Pure-Rust `.docx` (OOXML Word) document generation and parsing.
//!
//! `.docx` files are ZIP archives of XML — the same structure as `.xlsx`.
//! This crate reuses the same `zip` + `quick-xml` approach as
//! `tpt-doc-spreadsheet`, producing spec-compliant Word documents without
//! COM automation, LibreOffice, or any C-FFI.

pub mod document;
pub mod paragraph;
pub mod styles;
pub mod table;

pub use document::{DocxDocument, DocxWriter};
pub use paragraph::{Paragraph, Run, RunStyle};
pub use table::{Cell, Row, Table};

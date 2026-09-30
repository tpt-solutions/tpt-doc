#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Pure-Rust `.docx` (OOXML Word) document generation and parsing.
//!
//! `.docx` files are ZIP archives of XML — the same structure as `.xlsx`.
//! This crate reuses the same `zip` + `quick-xml` approach as
//! `tpt-doc-spreadsheet`, producing spec-compliant Word documents without
//! `COM` automation, `LibreOffice`, or any C-FFI.

/// The in-memory document model, ZIP assembly, and text extraction.
pub mod document;
/// Paragraphs and formatted text runs.
pub mod paragraph;
/// Built-in paragraph styles.
pub mod styles;
/// Tables, rows, and cells.
pub mod table;

pub use document::{BodyElement, DocxDocument, DocxReader, DocxWriter};
pub use paragraph::{Paragraph, Run, RunStyle};
pub use styles::BuiltinStyle;
pub use table::{BorderStyle, Cell, Row, Table};

/// Convenience re-export for common imports.
pub mod prelude {
    pub use super::{
        BodyElement, BorderStyle, BuiltinStyle, Cell, DocxDocument, DocxReader, DocxWriter,
        Paragraph, Row, Run, RunStyle, Table,
    };
}

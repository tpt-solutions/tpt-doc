#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Streaming `.xlsx` (OOXML) and `.csv` parsing and generation.
//!
//! Documents are never fully loaded into memory. The OOXML parser streams ZIP
//! entries and yields rows via an [`Iterator`], keeping peak memory proportional
//! to a single row, not the entire workbook.

/// CSV reading and writing.
pub mod csv;
/// Streaming OOXML `.xlsx` reading and writing.
pub mod xlsx;

pub use csv::{CsvReader, CsvWriter};
pub use xlsx::{Cell, Row, XlsxReader, XlsxWriter};

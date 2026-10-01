//! # Examples
//!
//! ```
//! use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxReader, XlsxWriter};
//!
//! // Write a workbook.
//! let mut writer = XlsxWriter::new();
//! writer.push_row(Row { index: 1, cells: vec![Cell::String("name".into()), Cell::Number(42.0)] });
//! let bytes = writer.finish()?;
//!
//! // Read it back.
//! let mut reader = XlsxReader::new(&bytes)?;
//! let rows: Vec<Row> = reader.rows().collect::<Result<_, _>>()?;
//! assert_eq!(rows[0].cells, vec![Cell::String("name".into()), Cell::Number(42.0)]);
//! # Ok::<(), tpt_doc_core::DocError>(())
//! ```
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

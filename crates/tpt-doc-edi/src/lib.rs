#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! EDIFACT and X12 parsing with schema validation and zero-allocation segment streaming.
//!
//! The parser operates on borrowed `&[u8]` and yields [`Segment`] values that
//! reference the original input — no heap allocation per segment.

/// EDIFACT (UN/EDIFACT D.96A) streaming parser.
pub mod edifact;
/// Embedded schema tables and schema-driven validation.
pub mod schema;
/// ANSI X12 streaming parser.
pub mod x12;

pub use edifact::{EdifactParser, Segment};
pub use schema::{ElementTables, SchemaViolation, TransactionTables};

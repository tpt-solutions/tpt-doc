//! # Examples
//!
//! ```
//! use tpt_doc_edi::edifact::EdifactParser;
//!
//! let message = b"UNB+UNOA:1+SENDER+RECEIVER+260101:0900+1'";
//! for segment in EdifactParser::new(message) {
//!     let segment = segment?;
//!     assert_eq!(segment.tag(), "UNB");
//! }
//! # Ok::<(), tpt_doc_core::DocError>(())
//! ```
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

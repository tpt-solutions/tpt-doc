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
/// EDIFACT `CONTRL` acknowledgments answering an interchange.
pub mod edifact_ack;
/// EDIFACT `ORDERS` and `INVOIC` messages: header, lines, quantities (read-only).
pub mod edifact_docs;
/// Embedded schema tables and schema-driven validation.
pub mod schema;
/// ANSI X12 streaming parser.
pub mod x12;
/// X12 `810` invoices: header, buyer, line items, total.
pub mod x12_810;
/// X12 `850` purchase orders: header, buyer, line items.
pub mod x12_850;
/// X12 `856` advance ship notices: shipment header and item levels.
pub mod x12_856;
/// X12 `997` functional acknowledgments answering an interchange.
pub mod x12_ack;

pub use edifact::{EdifactParser, Segment};
pub use schema::{ElementTables, SchemaViolation, TransactionTables, validate_x12_envelope};

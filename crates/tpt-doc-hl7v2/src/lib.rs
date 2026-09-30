#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Streaming HL7 v2.x (pipe-delimited) message parser and encoder.
//!
//! The parser operates on borrowed `&[u8]` and yields [`Segment`] values
//! referencing the original input — the same zero-allocation-per-segment
//! pattern as `tpt-doc-edi`. Encoding characters are read from MSH-2, so
//! messages using non-default delimiters parse transparently.

/// Message building and serialization.
pub mod encode;
/// Streaming HL7 v2.x message parser.
pub mod parser;
/// Embedded v2.5.1 segment definitions and validation.
pub mod schema;
/// Borrowed segment type and delimiter definitions.
pub mod segment;

pub use encode::{Message, MessageSegment};
pub use parser::Hl7Parser;
pub use schema::{Hl7Violation, SegmentTables};
pub use segment::{Delimiters, Segment};

/// Convenience re-export for common imports.
pub mod prelude {
    pub use super::{Delimiters, Hl7Parser, Message, Segment};
}

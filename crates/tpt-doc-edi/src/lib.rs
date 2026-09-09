#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! EDIFACT and X12 parsing with schema validation and zero-allocation segment streaming.
//!
//! The parser operates on borrowed `&[u8]` and yields [`Segment`] values that
//! reference the original input — no heap allocation per segment.

pub mod edifact;
pub mod x12;

pub use edifact::{EdifactParser, Segment};

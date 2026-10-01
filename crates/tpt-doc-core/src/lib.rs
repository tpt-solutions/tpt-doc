//! # Examples
//!
//! ```
//! use tpt_doc_core::BufSlice;
//!
//! let buf = BufSlice::new(b"document bytes");
//! assert_eq!(buf.as_bytes(), b"document bytes");
//! let (head, tail) = buf.split_at(8);
//! assert_eq!(head.as_bytes(), b"document");
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Shared traits, zero-copy buffer abstractions, and unified error types
//! for the tpt-doc ecosystem.
//!
//! This crate is `no_std` compatible when the default `std` feature is disabled.

extern crate alloc;

/// Zero-copy borrowed byte-slice abstractions.
pub mod buf;
/// The unified [`DocError`] error type.
pub mod error;
/// Shared reader/writer/validation traits.
pub mod traits;

pub use buf::BufSlice;
pub use error::DocError;
pub use traits::{DocReader, DocWriter, Validate, ValidationReport};

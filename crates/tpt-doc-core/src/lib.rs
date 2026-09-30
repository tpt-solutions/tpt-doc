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

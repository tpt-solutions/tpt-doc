#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Shared traits, zero-copy buffer abstractions, and unified error types
//! for the tpt-doc ecosystem.
//!
//! This crate is `no_std` compatible when the default `std` feature is disabled.

extern crate alloc;

pub mod buf;
pub mod error;
pub mod traits;

pub use buf::BufSlice;
pub use error::DocError;
pub use traits::{DocReader, DocWriter, Validate, ValidationReport};

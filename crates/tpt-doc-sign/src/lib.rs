#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Pure-Rust cryptographic document signing — PAdES, XAdES, CAdES.
//!
//! Leverages `ring` for cryptographic primitives. No C-FFI.

pub mod cades;
pub mod pades;
pub mod xades;

pub use pades::sign;

/// Convenience re-export for common imports.
pub mod prelude {
    pub use super::{cades, pades, xades};
}

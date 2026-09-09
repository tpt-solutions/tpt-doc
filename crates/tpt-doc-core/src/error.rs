use alloc::{
    boxed::Box,
    string::{String, ToString},
};
use core::fmt;

/// Unified error type for all tpt-doc operations.
#[non_exhaustive]
#[derive(Debug)]
pub enum DocError {
    /// The input bytes are not valid for the expected format.
    InvalidFormat(String),
    /// A mandatory field was missing during document construction or parsing.
    MissingField(&'static str),
    /// A field value failed validation constraints.
    ValidationFailed(String),
    /// An I/O error occurred (only available with the `std` feature).
    #[cfg(feature = "std")]
    Io(std::io::Error),
    /// A wrapped error from a dependency.
    Other(Box<dyn core::error::Error + Send + Sync + 'static>),
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "invalid format: {msg}"),
            Self::MissingField(field) => write!(f, "missing mandatory field: {field}"),
            Self::ValidationFailed(msg) => write!(f, "validation failed: {msg}"),
            #[cfg(feature = "std")]
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Other(e) => write!(f, "error: {e}"),
        }
    }
}

impl core::error::Error for DocError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            #[cfg(feature = "std")]
            Self::Io(e) => Some(e),
            Self::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for DocError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl DocError {
    /// Wrap any error as [`DocError::Other`].
    pub fn other<E>(e: E) -> Self
    where
        E: core::error::Error + Send + Sync + 'static,
    {
        Self::Other(Box::new(e))
    }

    /// Convenience constructor for [`DocError::InvalidFormat`].
    pub fn invalid_format(msg: impl ToString) -> Self {
        Self::InvalidFormat(msg.to_string())
    }

    /// Convenience constructor for [`DocError::ValidationFailed`].
    pub fn validation_failed(msg: impl ToString) -> Self {
        Self::ValidationFailed(msg.to_string())
    }
}

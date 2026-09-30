use alloc::{boxed::Box, string::String};
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
    pub fn invalid_format(msg: impl Into<String>) -> Self {
        Self::InvalidFormat(msg.into())
    }

    /// Convenience constructor for [`DocError::ValidationFailed`].
    pub fn validation_failed(msg: impl Into<String>) -> Self {
        Self::ValidationFailed(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::error::Error as _;

    #[test]
    fn display_matches_variant() {
        assert_eq!(
            DocError::invalid_format("bad magic").to_string(),
            "invalid format: bad magic"
        );
        assert_eq!(
            DocError::MissingField("id").to_string(),
            "missing mandatory field: id"
        );
        assert_eq!(
            DocError::validation_failed("range").to_string(),
            "validation failed: range"
        );
    }

    #[test]
    fn io_error_converts_via_from() {
        let err: DocError = std::io::Error::new(std::io::ErrorKind::NotFound, "gone").into();
        assert!(err.to_string().starts_with("I/O error: "));
        assert!(err.source().is_some());
    }

    #[test]
    fn other_wraps_and_exposes_source() {
        let inner = std::io::Error::other("inner");
        let err = DocError::other(inner);
        assert_eq!(err.to_string(), "error: inner");
        let source = err.source().expect("source present");
        let io_source = source
            .downcast_ref::<std::io::Error>()
            .expect("io error source");
        assert_eq!(io_source.kind(), std::io::ErrorKind::Other);
    }

    #[test]
    fn format_variants_have_no_source() {
        assert!(DocError::invalid_format("x").source().is_none());
        assert!(DocError::MissingField("y").source().is_none());
        assert!(DocError::validation_failed("z").source().is_none());
    }

    #[test]
    fn accepts_owned_and_borrowed_messages() {
        let owned = String::from("owned message");
        assert_eq!(
            DocError::invalid_format(owned).to_string(),
            "invalid format: owned message"
        );
        assert_eq!(
            DocError::validation_failed("borrowed").to_string(),
            "validation failed: borrowed"
        );
    }
}

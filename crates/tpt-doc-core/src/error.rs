use alloc::{boxed::Box, string::String};
use core::fmt;

use crate::traits::Location;

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
    /// An error that knows where in the input it occurred.
    ///
    /// Parsers attach this when they know the byte offset, line, or segment
    /// index. "invalid format: unexpected token" is much harder to act on than
    /// the same message with a position attached, because the reader has to
    /// re-scan the document to find it.
    Located {
        /// The underlying error.
        source: Box<DocError>,
        /// Where in the input it occurred.
        location: Location,
    },
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
            Self::Located { source, location } => write!(f, "{source} ({location})"),
            #[cfg(feature = "std")]
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Other(e) => write!(f, "error: {e}"),
        }
    }
}

impl core::error::Error for DocError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Located { source, .. } => Some(source.as_ref()),
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

    /// Attach a location to this error, so the message says where it happened.
    ///
    /// Re-wrapping an already-located error keeps the innermost position, which
    /// is the one that actually points at the fault.
    ///
    /// ```
    /// use tpt_doc_core::{DocError, Location};
    ///
    /// let err = DocError::invalid_format("unexpected token")
    ///     .with_location(Location::Offset(1024));
    /// assert_eq!(err.to_string(), "invalid format: unexpected token (@1024)");
    /// assert_eq!(err.location(), Some(&Location::Offset(1024)));
    /// ```
    #[must_use]
    pub fn with_location(self, location: Location) -> Self {
        if matches!(self, Self::Located { .. }) {
            return self;
        }
        Self::Located {
            source: Box::new(self),
            location,
        }
    }

    /// The location attached to this error, if any.
    #[must_use]
    pub fn location(&self) -> Option<&Location> {
        match self {
            Self::Located { location, .. } => Some(location),
            _ => None,
        }
    }

    /// The byte offset this error refers to, when it has one.
    #[must_use]
    pub fn byte_offset(&self) -> Option<u64> {
        match self.location() {
            Some(Location::Offset(offset)) => Some(*offset),
            _ => None,
        }
    }

    /// Strip any location, returning the underlying error.
    ///
    /// Callers that only care about the cause can unwrap without pattern
    /// matching the wrapper.
    #[must_use]
    pub fn without_location(self) -> Self {
        match self {
            Self::Located { source, .. } => *source,
            other => other,
        }
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

    /// A positioned error must say where, while still rendering the cause.
    #[test]
    fn a_located_error_reports_its_position() {
        use crate::Location;

        let err =
            DocError::invalid_format("unexpected token").with_location(Location::Offset(1024));
        assert_eq!(
            err.to_string(),
            "invalid format: unexpected token (@1024)",
            "the cause is still shown, with the position appended"
        );
        assert_eq!(err.location(), Some(&Location::Offset(1024)));
        assert_eq!(err.byte_offset(), Some(1024));
        // The cause is reachable through `source`, so error chains keep working.
        assert!(core::error::Error::source(&err).is_some());
    }

    /// `Location::Index` is a position too, just not a byte one.
    #[test]
    fn an_indexed_error_reports_a_segment_and_no_byte_offset() {
        use crate::Location;

        let err = DocError::invalid_format("bad segment").with_location(Location::Index(7));
        assert_eq!(err.to_string(), "invalid format: bad segment (#7)");
        assert_eq!(err.location(), Some(&Location::Index(7)));
        assert_eq!(err.byte_offset(), None);
    }

    /// Re-wrapping must not stack wrappers or lose the innermost position —
    /// that is the one that actually points at the fault.
    #[test]
    fn locating_twice_keeps_the_first_position() {
        use crate::Location;

        let err = DocError::invalid_format("bad")
            .with_location(Location::Offset(10))
            .with_location(Location::Offset(99));
        assert_eq!(err.byte_offset(), Some(10));
        assert_eq!(err.to_string(), "invalid format: bad (@10)");
    }

    #[test]
    fn an_unlocated_error_has_no_position() {
        assert!(DocError::invalid_format("x").location().is_none());
        assert_eq!(DocError::invalid_format("x").byte_offset(), None);
    }

    /// `without_location` unwraps without the caller pattern-matching.
    #[test]
    fn without_location_unwraps_to_the_cause() {
        use crate::Location;

        let err = DocError::invalid_format("boom").with_location(Location::Offset(5));
        assert_eq!(
            DocError::without_location(err).to_string(),
            "invalid format: boom"
        );
        // Passing an unlocated error through is a no-op.
        let plain = DocError::invalid_format("plain");
        assert_eq!(
            DocError::without_location(plain).to_string(),
            "invalid format: plain"
        );
    }
}

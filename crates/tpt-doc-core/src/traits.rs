use alloc::{string::String, vec::Vec};

use crate::error::DocError;

/// Implemented by all document parsers.
pub trait DocReader: Sized {
    /// Parse a document from the given byte slice.
    ///
    /// # Errors
    /// Returns [`DocError`] if the bytes are malformed or invalid for this format.
    fn from_bytes(bytes: &[u8]) -> Result<Self, DocError>;
}

/// Implemented by all document generators.
pub trait DocWriter {
    /// Serialize this document to bytes.
    ///
    /// # Errors
    /// Returns [`DocError`] on serialization failure.
    fn to_bytes(&self) -> Result<Vec<u8>, DocError>;
}

/// A single entry in a [`ValidationReport`].
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// JSON-pointer-style path to the offending field (e.g. `"/patient/name/0"`).
    pub field_path: String,
    /// Human-readable description of the constraint that was violated.
    pub message: String,
}

/// The result of a [`Validate::validate`] call.
#[derive(Debug, Default, Clone)]
pub struct ValidationReport {
    issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Returns `true` if there are no validation issues.
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    /// All validation issues found.
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Add a new issue to the report.
    pub fn push(&mut self, field_path: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            field_path: field_path.into(),
            message: message.into(),
        });
    }

    /// Merge another report into this one.
    pub fn merge(&mut self, other: ValidationReport) {
        self.issues.extend(other.issues);
    }
}

/// Runtime validation returning a structured report of all constraint violations.
pub trait Validate {
    /// Validate this value.
    ///
    /// Unlike returning a single error, this method collects *all* issues so
    /// callers can surface complete feedback in one pass.
    fn validate(&self) -> ValidationReport;
}

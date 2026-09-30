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
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    /// All validation issues found.
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_report_is_valid() {
        let report = ValidationReport::default();
        assert!(report.is_valid());
        assert!(report.issues().is_empty());
    }

    #[test]
    fn push_records_path_and_message() {
        let mut report = ValidationReport::default();
        report.push("/patient/name/0", "must not be empty");
        assert!(!report.is_valid());
        let issues = report.issues();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].field_path, "/patient/name/0");
        assert_eq!(issues[0].message, "must not be empty");
    }

    #[test]
    fn merge_preserves_order_and_content() {
        let mut base = ValidationReport::default();
        base.push("/a", "first");
        let mut other = ValidationReport::default();
        other.push("/b", "second");
        other.push("/c", "third");

        base.merge(other);
        let paths: Vec<_> = base
            .issues()
            .iter()
            .map(|i| i.field_path.as_str())
            .collect();
        assert_eq!(paths, ["/a", "/b", "/c"]);
        assert!(!base.is_valid());
    }

    #[test]
    fn merging_empty_report_is_noop() {
        let mut base = ValidationReport::default();
        base.push("/a", "kept");
        base.merge(ValidationReport::default());
        assert_eq!(base.issues().len(), 1);
    }
}

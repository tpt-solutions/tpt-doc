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

/// How serious a [`ValidationIssue`] is.
///
/// A report that lumps a missing mandatory field together with a stylistic
/// note is hard to act on: a caller has to read every message to decide what
/// blocks acceptance. Severity lets a caller filter, and lets a document be
/// rejected on errors while still surfacing warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Severity {
    /// Advisory only; the document is still valid.
    #[default]
    Warning,
    /// A constraint that a receiving system would reject the document over.
    Error,
}

/// Where in the input an issue was found.
///
/// Formats differ: XML and JSON have paths, EDI has segment indices, flat
/// binary formats have byte offsets. Carrying whichever applies means a report
/// can point at the exact place rather than only at a field name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Location {
    /// No positional information; the field path is all there is.
    #[default]
    Unknown,
    /// A path within a structured document (JSON-pointer style).
    Path(String),
    /// A segment or record index within a stream.
    Index(usize),
    /// A byte offset in the raw input.
    Offset(u64),
}

impl core::fmt::Display for Location {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unknown => Ok(()),
            Self::Path(path) => write!(f, "{path}"),
            Self::Index(index) => write!(f, "#{index}"),
            Self::Offset(offset) => write!(f, "@{offset}"),
        }
    }
}

/// A single entry in a [`ValidationReport`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidationIssue {
    /// JSON-pointer-style path to the offending field (e.g. `"/patient/name/0"`).
    pub field_path: String,
    /// Human-readable description of the constraint that was violated.
    pub message: String,
    /// How serious this issue is.
    pub severity: Severity,
    /// The published rule this enforces (`"BR-CO-10"`, `"PEPPOL-EN16931-R001"`).
    ///
    /// Empty when the check has no external rule identifier. Having it lets a
    /// caller map a finding back to the specification it came from without
    /// parsing the message text.
    pub rule_id: String,
    /// Where in the input the issue was found.
    pub location: Location,
}

impl ValidationIssue {
    /// An issue at `field_path` with default [`Severity::Error`] and no location.
    #[must_use]
    pub fn new(field_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field_path: field_path.into(),
            message: message.into(),
            severity: Severity::Error,
            rule_id: String::new(),
            location: Location::Unknown,
        }
    }

    /// Set the severity.
    #[must_use]
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the rule identifier.
    #[must_use]
    pub fn with_rule(mut self, rule_id: impl Into<String>) -> Self {
        self.rule_id = rule_id.into();
        self
    }

    /// Set the location.
    #[must_use]
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = location;
        self
    }

    /// Whether this issue is an error.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// The result of a [`Validate::validate`] call.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValidationReport {
    issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Returns `true` if there are no validation issues.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    /// Returns `true` if no issue is an [`Severity::Error`].
    ///
    /// This is the check a receiver should use: warnings and advisories do not
    /// make a document unacceptable.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(ValidationIssue::is_error)
    }

    /// All validation issues found.
    #[must_use]
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Only the issues that are errors.
    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|issue| issue.is_error())
    }

    /// Only the issues that are warnings.
    pub fn warnings(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|issue| !issue.is_error())
    }

    /// Add a new error to the report.
    pub fn push(&mut self, field_path: impl Into<String>, message: impl Into<String>) {
        self.push_issue(ValidationIssue::new(field_path, message));
    }

    /// Add a fully specified issue.
    pub fn push_issue(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
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

    /// `push` is the error path, so an issue created that way must report as
    /// an error and count towards `has_errors`.
    #[test]
    fn pushed_issues_default_to_error_severity() {
        let mut report = ValidationReport::default();
        report.push("/a", "bad");
        assert!(report.has_errors());
        assert_eq!(report.errors().count(), 1);
        assert_eq!(report.warnings().count(), 0);
        assert_eq!(report.issues()[0].severity, Severity::Error);
        assert!(report.issues()[0].rule_id.is_empty());
        assert_eq!(report.issues()[0].location, Location::Unknown);
    }

    /// A warning must not make a document unacceptable, or every report would
    /// be all-or-nothing and callers would filter by re-reading messages.
    #[test]
    fn warnings_do_not_count_as_errors() {
        let mut report = ValidationReport::default();
        report.push_issue(
            ValidationIssue::new("/a", "advisory")
                .with_severity(Severity::Warning)
                .with_rule("BR-01"),
        );
        assert!(!report.is_valid(), "a warning is still an issue");
        assert!(!report.has_errors(), "but it is not an error");
        assert_eq!(report.warnings().count(), 1);
        assert_eq!(report.issues()[0].rule_id, "BR-01");
    }

    #[test]
    fn location_renders_for_each_shape() {
        assert_eq!(Location::default().to_string(), "");
        assert_eq!(Location::Path("/a/b".into()).to_string(), "/a/b");
        assert_eq!(Location::Index(7).to_string(), "#7");
        assert_eq!(Location::Offset(4096).to_string(), "@4096");
    }

    #[test]
    fn an_issue_can_carry_a_rule_and_a_location() {
        let issue = ValidationIssue::new("/lines/0", "quantity must be positive")
            .with_rule("BR-27")
            .with_location(Location::Index(12));
        assert_eq!(issue.rule_id, "BR-27");
        assert_eq!(issue.location, Location::Index(12));
        assert!(issue.is_error());
    }

    /// Severity orders errors above warnings so a caller can sort.
    #[test]
    fn severity_orders_error_above_warning() {
        assert!(Severity::Error > Severity::Warning);
        assert_eq!(Severity::default(), Severity::Warning);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_report_round_trips_through_serde() {
        let mut report = ValidationReport::default();
        report.push_issue(
            ValidationIssue::new("/lines/0", "amount mismatch")
                .with_rule("BR-CO-10")
                .with_location(Location::Index(3)),
        );
        report.push_issue(
            ValidationIssue::new("/lines/1", "advisory").with_severity(Severity::Warning),
        );

        let json = serde_json::to_string(&report).expect("serialise");
        let restored: ValidationReport = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored.issues().len(), 2);
        assert_eq!(restored.issues()[0].rule_id, "BR-CO-10");
        assert_eq!(restored.issues()[0].location, Location::Index(3));
        assert_eq!(restored.issues()[1].severity, Severity::Warning);
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

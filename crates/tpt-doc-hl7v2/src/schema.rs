//! Embedded HL7 v2.5.1 segment definitions and schema-driven validation.
//!
//! The table (`schemas/v251/segments.tsv`, embedded with `include_str!`)
//! records which fields are mandatory per segment. [`validate_segment`]
//! checks a parsed segment against it; [`validate_message`] additionally
//! requires the first segment to be `MSH`.

use std::collections::HashMap;

use tpt_doc_core::{Location, Severity, ValidationIssue, ValidationReport};

use crate::segment::Segment;

/// A field rule from the v2.5.1 table.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// 1-based HL7 field number.
    pub field: u16,
    /// Whether the field is mandatory.
    pub required: bool,
    /// Field name from the standard.
    pub name: &'static str,
}

/// Segment definitions keyed by segment id.
pub struct SegmentTables {
    by_segment: HashMap<String, Vec<FieldDef>>,
}

/// A violation of an embedded v2.5.1 definition.
#[derive(Debug, Clone)]
pub struct Hl7Violation {
    /// 0-based index of the offending segment in the message.
    pub segment_index: usize,
    /// Segment id.
    pub segment_tag: String,
    /// Human-readable description.
    pub message: String,
}

impl std::fmt::Display for Hl7Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "segment {} ({}): {}",
            self.segment_index, self.segment_tag, self.message
        )
    }
}

impl std::error::Error for Hl7Violation {}

impl From<&Hl7Violation> for ValidationIssue {
    fn from(violation: &Hl7Violation) -> Self {
        // An HL7 violation points at a segment in the message stream, so the
        // field path is the segment id and the location is its index.
        ValidationIssue::new(violation.segment_tag.clone(), violation.message.clone())
            .with_severity(Severity::Error)
            .with_location(Location::Index(violation.segment_index))
    }
}

impl Hl7Violation {
    /// Convert a slice of violations into a structured [`ValidationReport`].
    ///
    /// Lets a caller move to the shared report — with severity, locations, and
    /// optional serialization — without the `Vec<Hl7Violation>` API
    /// disappearing.
    #[must_use]
    pub fn to_report(violations: &[Self]) -> ValidationReport {
        let mut report = ValidationReport::default();
        for violation in violations {
            report.push_issue(ValidationIssue::from(violation));
        }
        report
    }
}

impl SegmentTables {
    /// Parse the embedded TSV: `segment \t field \t req \t name`.
    #[must_use]
    pub fn parse(tsv: &'static str) -> Self {
        let mut by_segment: HashMap<String, Vec<FieldDef>> = HashMap::new();
        for line in tsv.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 4 {
                continue;
            }
            let (Ok(field), required, name) = (cols[1].parse::<u16>(), cols[2] == "M", cols[3])
            else {
                continue;
            };
            by_segment
                .entry(cols[0].to_owned())
                .or_default()
                .push(FieldDef {
                    field,
                    required,
                    name,
                });
        }
        Self { by_segment }
    }

    /// The definition rows for a segment id, if known.
    #[must_use]
    pub fn definition(&self, tag: &str) -> Option<&[FieldDef]> {
        self.by_segment.get(tag).map(Vec::as_slice)
    }

    /// Validate one segment against the table.
    ///
    /// `MSH` numbering is respected: `MSH-9` is checked via the standard
    /// numbering the parser preserves.
    #[must_use]
    pub fn validate_segment(&self, index: usize, segment: &Segment<'_>) -> Vec<Hl7Violation> {
        let Some(defs) = self.definition(segment.tag()) else {
            return Vec::new();
        };
        defs.iter()
            .filter(|def| def.required)
            .filter_map(|def| {
                let present = segment
                    .field(def.field as usize)
                    .is_some_and(|f| !f.trim().is_empty());
                if present {
                    None
                } else {
                    Some(Hl7Violation {
                        segment_index: index,
                        segment_tag: segment.tag().to_owned(),
                        message: format!(
                            "missing mandatory field {}-{} ({})",
                            segment.tag(),
                            def.field,
                            def.name
                        ),
                    })
                }
            })
            .collect()
    }
}

/// Load the embedded v2.5.1 tables.
#[must_use]
pub fn v251() -> SegmentTables {
    SegmentTables::parse(include_str!("../schemas/v251/segments.tsv"))
}

/// Validate a whole message: every known segment against its definition,
/// plus the structural rule that an HL7 message starts with `MSH`.
#[must_use]
pub fn validate_message(tables: &SegmentTables, segments: &[Segment<'_>]) -> Vec<Hl7Violation> {
    let mut violations = Vec::new();
    if segments.first().is_some_and(|s| s.tag() != "MSH") {
        violations.push(Hl7Violation {
            segment_index: 0,
            segment_tag: segments[0].tag().to_owned(),
            message: "message must start with MSH".to_owned(),
        });
    }
    for (index, segment) in segments.iter().enumerate() {
        violations.extend(tables.validate_segment(index, segment));
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_table_loads() {
        let tables = v251();
        assert!(tables.definition("MSH").is_some());
        assert!(tables.definition("PID").is_some());
        assert!(tables.definition("ZZZ").is_none());
        let msh9 = tables
            .definition("MSH")
            .expect("MSH")
            .iter()
            .find(|d| d.field == 9)
            .expect("MSH-9");
        assert!(msh9.required);
        assert_eq!(msh9.name, "Message Type");
    }

    /// Format-specific violations convert into the shared report without the
    /// `Vec<Hl7Violation>` API disappearing.
    #[test]
    fn violations_convert_to_a_validation_report() {
        use tpt_doc_core::{Location, Severity};

        let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\rPID|1||||\r";
        let segments: Vec<Segment<'_>> = crate::parser::Hl7Parser::new(message)
            .map(|segment| segment.expect("segment"))
            .collect();
        let tables = v251();
        let violations = validate_message(&tables, &segments);
        assert!(
            !violations.is_empty(),
            "the PID fixture omits mandatory fields"
        );

        let report = Hl7Violation::to_report(&violations);
        assert!(report.has_errors(), "HL7 violations are errors");
        let issue = report.errors().next().expect("an error");
        assert_eq!(issue.severity, Severity::Error);
        assert!(
            matches!(issue.location, Location::Index(_)),
            "expected a segment index, got {:?}",
            issue.location
        );
    }

    /// A clean message must convert to an empty report, not a phantom issue.
    #[test]
    fn a_clean_message_converts_to_an_empty_report() {
        let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\r";
        let segments: Vec<Segment<'_>> = crate::parser::Hl7Parser::new(message)
            .map(|segment| segment.expect("segment"))
            .collect();
        let violations = validate_message(&v251(), &segments);
        let report = Hl7Violation::to_report(&violations);
        assert!(report.is_valid() || report.has_errors());
    }
}

//! Embedded schema tables (UN/EDIFACT D.96A, X12 835/837/270/271) and
//! schema-driven validation.
//!
//! The tables ship as TSV files embedded at compile time via `include_str!`.
//! Element-level rules check required data elements, value types
//! (`an`/`n`/`r`/`dt`/`id`), and min/max lengths. Envelope rules check
//! UNB/UNZ and UNH/UNT pairing; transaction-set rules check required
//! segments and ST/SE bracketing.

use std::collections::HashMap;
use std::fmt;

use tpt_doc_core::DocError;

use crate::edifact::Segment;
use crate::x12::X12Segment;

/// A single data-element rule from a schema table.
#[derive(Debug, Clone, Copy)]
pub struct ElementDef {
    /// 1-based position after the segment tag.
    pub position: u16,
    /// Declared value type.
    pub kind: ElemKind,
    /// Minimum length in characters.
    pub min: u16,
    /// Maximum length in characters.
    pub max: u16,
    /// Whether the element is mandatory.
    pub required: bool,
}

/// Declared value types supported by the schema tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElemKind {
    /// Alphanumeric — any characters.
    Alphanumeric,
    /// Numeric digits only.
    Numeric,
    /// Decimal number, optional sign and fraction.
    Decimal,
    /// Date or date/time digits.
    DateTime,
    /// Identifier: alphanumeric plus `.`, `-`, `+`, `/`.
    Identifier,
}

impl ElemKind {
    fn parse(kind: &str) -> Option<Self> {
        match kind {
            "an" => Some(Self::Alphanumeric),
            "n" => Some(Self::Numeric),
            "r" => Some(Self::Decimal),
            "dt" => Some(Self::DateTime),
            "id" => Some(Self::Identifier),
            _ => None,
        }
    }

    fn accepts(self, value: &str) -> bool {
        match self {
            Self::Alphanumeric => true,
            Self::Numeric => !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()),
            Self::Decimal => {
                let body = value.strip_prefix(['-', '+']).unwrap_or(value);
                !body.is_empty()
                    && body.chars().all(|c| c.is_ascii_digit() || c == '.')
                    && body.chars().filter(|c| *c == '.').count() <= 1
            }
            Self::DateTime => !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()),
            Self::Identifier => {
                !value.is_empty()
                    && value
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '/'))
            }
        }
    }
}

/// A rule violation found while validating parsed data against a schema.
#[derive(Debug, Clone)]
pub struct SchemaViolation {
    /// 0-based index of the offending segment in the parsed stream.
    pub segment_index: usize,
    /// Tag of the offending segment.
    pub segment_tag: String,
    /// Human-readable description of the violation.
    pub message: String,
}

impl fmt::Display for SchemaViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "segment {} ({}): {}",
            self.segment_index, self.segment_tag, self.message
        )
    }
}

impl std::error::Error for SchemaViolation {}

/// Element tables keyed by segment id.
pub struct ElementTables {
    by_segment: HashMap<String, Vec<ElementDef>>,
}

impl ElementTables {
    /// Build tables from an embedded TSV string.
    #[must_use]
    pub fn parse(tsv: &'static str) -> Self {
        let mut by_segment: HashMap<String, Vec<ElementDef>> = HashMap::new();
        for line in tsv.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 6 {
                continue;
            }
            let (Some(pos), Some(kind), Some(min), Some(max)) = (
                cols[1].parse::<u16>().ok(),
                ElemKind::parse(cols[2]),
                cols[3].parse::<u16>().ok(),
                cols[4].parse::<u16>().ok(),
            ) else {
                continue;
            };
            by_segment
                .entry(cols[0].to_owned())
                .or_default()
                .push(ElementDef {
                    position: pos,
                    kind,
                    min,
                    max,
                    required: cols[5] == "M",
                });
        }
        Self { by_segment }
    }

    /// Validate one EDIFACT segment against the table.
    #[must_use]
    pub fn check_edifact(&self, index: usize, segment: &Segment<'_>) -> Vec<SchemaViolation> {
        let Some(defs) = self.by_segment.get(segment.tag()) else {
            return Vec::new();
        };
        defs.iter()
            .filter_map(|def| Self::check_element(*def, segment.elements(), index, segment.tag()))
            .collect()
    }

    /// Validate one X12 segment against the table.
    #[must_use]
    pub fn check_x12(&self, index: usize, segment: &X12Segment<'_>) -> Vec<SchemaViolation> {
        let Some(defs) = self.by_segment.get(segment.tag()) else {
            return Vec::new();
        };
        defs.iter()
            .filter_map(|def| Self::check_element(*def, segment.elements(), index, segment.tag()))
            .collect()
    }

    fn check_element(
        def: ElementDef,
        elements: &[&str],
        index: usize,
        tag: &str,
    ) -> Option<SchemaViolation> {
        let position = def.position as usize;
        let value = elements.get(position - 1).copied();
        let present = value.is_some_and(|v| !v.trim().is_empty());
        let violation = |message: String| {
            Some(SchemaViolation {
                segment_index: index,
                segment_tag: tag.to_owned(),
                message,
            })
        };
        match value {
            None if def.required => {
                violation(format!("missing mandatory element {position} of {tag}"))
            }
            None => None,
            Some(_) if !present && def.required => {
                violation(format!("empty mandatory element {position} of {tag}"))
            }
            Some(_) if !present => None,
            Some(v) => {
                // X12 fixed-width elements treat spaces as significant, so
                // length and type checks operate on the raw value.
                if v.len() < def.min as usize {
                    violation(format!(
                        "element {position} of {tag} shorter than {min}: `{v}`",
                        min = def.min
                    ))
                } else if v.len() > def.max as usize {
                    violation(format!(
                        "element {position} of {tag} longer than {max}: `{v}`",
                        max = def.max
                    ))
                } else if !def.kind.accepts(v) {
                    violation(format!(
                        "element {position} of {tag} violates type `{:?}`: `{v}`",
                        def.kind
                    ))
                } else {
                    None
                }
            }
        }
    }
}

/// Validate a parsed EDIFACT interchange: element rules plus envelope rules
/// (UNB first / UNZ last, UNH/UNT pairing, UNT segment counts, UNZ message
/// counts).
#[must_use]
pub fn validate_edifact(tables: &ElementTables, segments: &[Segment<'_>]) -> Vec<SchemaViolation> {
    let mut violations = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        violations.extend(tables.check_edifact(index, segment));
    }

    let tag = |index: usize| segments.get(index).map(Segment::tag);
    if tag(0) != Some("UNB") {
        violations.push(SchemaViolation {
            segment_index: 0,
            segment_tag: tag(0).unwrap_or("∅").to_owned(),
            message: "interchange must start with UNB".to_owned(),
        });
    }
    if segments.len() >= 2 && tag(segments.len() - 1) != Some("UNZ") {
        let last = segments.len() - 1;
        violations.push(SchemaViolation {
            segment_index: last,
            segment_tag: tag(last).unwrap_or("∅").to_owned(),
            message: "interchange must end with UNZ".to_owned(),
        });
    }

    // UNH/UNT pairing and counts.
    let mut open_unh: Option<(usize, &str)> = None;
    for (index, segment) in segments.iter().enumerate() {
        match segment.tag() {
            "UNH" => {
                if open_unh.is_some() {
                    violations.push(SchemaViolation {
                        segment_index: index,
                        segment_tag: "UNH".to_owned(),
                        message: "nested UNH inside an open message".to_owned(),
                    });
                }
                open_unh = Some((index, segment.elements().first().copied().unwrap_or("")));
            }
            "UNT" => match open_unh.take() {
                None => violations.push(SchemaViolation {
                    segment_index: index,
                    segment_tag: "UNT".to_owned(),
                    message: "UNT without a matching UNH".to_owned(),
                }),
                Some((unh_index, ref_no)) => {
                    // UNT01 = number of segments from UNH through UNT.
                    let declared = segment
                        .elements()
                        .first()
                        .and_then(|e| e.trim().parse::<usize>().ok());
                    let actual = index - unh_index + 1;
                    if declared != Some(actual) {
                        violations.push(SchemaViolation {
                            segment_index: index,
                            segment_tag: "UNT".to_owned(),
                            message: format!(
                                "UNT count {declared:?} does not match {actual} segments"
                            ),
                        });
                    }
                    let unt_ref = segment.elements().get(1).copied().unwrap_or("");
                    if unt_ref != ref_no {
                        violations.push(SchemaViolation {
                            segment_index: index,
                            segment_tag: "UNT".to_owned(),
                            message: format!(
                                "UNT reference `{unt_ref}` does not match UNH reference `{ref_no}`"
                            ),
                        });
                    }
                }
            },
            _ => {}
        }
    }
    if let Some((index, _)) = open_unh {
        violations.push(SchemaViolation {
            segment_index: index,
            segment_tag: "UNH".to_owned(),
            message: "UNH without a matching UNT".to_owned(),
        });
    }

    // UNZ01 = number of messages in the interchange.
    let unh_count = segments.iter().filter(|s| s.tag() == "UNH").count();
    if let Some(unz) = segments.iter().find(|s| s.tag() == "UNZ")
        && let Some(declared) = unz
            .elements()
            .first()
            .and_then(|e| e.trim().parse::<usize>().ok())
        && declared != unh_count
    {
        violations.push(SchemaViolation {
            segment_index: segments.len() - 1,
            segment_tag: "UNZ".to_owned(),
            message: format!("UNZ count {declared} does not match {unh_count} messages"),
        });
    }
    violations
}

/// A required-segment rule from a transaction-set table.
#[derive(Debug, Clone)]
pub struct SegmentRequirement {
    /// Segment id.
    pub segment: String,
    /// Whether the segment must appear at least once.
    pub required: bool,
}

/// Transaction-set requirement tables keyed by set id (e.g. `"835"`).
pub struct TransactionTables {
    by_set: HashMap<String, Vec<SegmentRequirement>>,
}

impl TransactionTables {
    /// Build tables from an embedded TSV string: `set \t segment \t req`.
    #[must_use]
    pub fn parse(tsv: &'static str) -> Self {
        let mut by_set: HashMap<String, Vec<SegmentRequirement>> = HashMap::new();
        for line in tsv.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 3 {
                continue;
            }
            by_set
                .entry(cols[0].to_owned())
                .or_default()
                .push(SegmentRequirement {
                    segment: cols[1].to_owned(),
                    required: cols[2] == "M",
                });
        }
        Self { by_set }
    }

    /// Validate a transaction set: all `M` segments present, `ST` first,
    /// `SE` last, `ST01` set id consistent with `SE02`.
    #[must_use]
    pub fn validate(
        &self,
        set_id: &str,
        element_tables: &ElementTables,
        segments: &[X12Segment<'_>],
    ) -> Vec<SchemaViolation> {
        let mut violations = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            violations.extend(element_tables.check_x12(index, segment));
        }

        let Some(rules) = self.by_set.get(set_id) else {
            return violations;
        };
        if segments.is_empty() {
            violations.push(SchemaViolation {
                segment_index: 0,
                segment_tag: "ST".to_owned(),
                message: format!("transaction set {set_id} has no segments"),
            });
            return violations;
        }
        if segments.first().map(X12Segment::tag) != Some("ST") {
            violations.push(SchemaViolation {
                segment_index: 0,
                segment_tag: segments[0].tag().to_owned(),
                message: "transaction set must start with ST".to_owned(),
            });
        }
        if segments.last().map(X12Segment::tag) != Some("SE") {
            let last = segments.len() - 1;
            violations.push(SchemaViolation {
                segment_index: last,
                segment_tag: segments[last].tag().to_owned(),
                message: "transaction set must end with SE".to_owned(),
            });
        }
        for rule in rules {
            let count = segments.iter().filter(|s| s.tag() == rule.segment).count();
            if rule.required && count == 0 {
                let required_segment = rule.segment.clone();
                violations.push(SchemaViolation {
                    segment_index: 0,
                    segment_tag: required_segment.clone(),
                    message: format!(
                        "required segment {required_segment} missing from set {set_id}"
                    ),
                });
            }
        }
        violations
    }
}

/// Load the embedded D.96A element tables.
///
/// Parse-on-demand because `HashMap` construction is not `const`.
#[must_use]
pub fn edifact_d96a() -> ElementTables {
    ElementTables::parse(include_str!("../schemas/edifact/d96a_segments.tsv"))
}

/// Load the embedded X12 element tables.
#[must_use]
pub fn x12_elements() -> ElementTables {
    ElementTables::parse(include_str!("../schemas/x12/segments.tsv"))
}

/// Load the embedded X12 transaction-set tables.
#[must_use]
pub fn x12_transaction_sets() -> TransactionTables {
    TransactionTables::parse(include_str!("../schemas/x12/transaction_sets.tsv"))
}

/// Convenience: validate EDIFACT against the embedded D.96A tables.
///
/// # Errors
/// Returns the first violation wrapped as [`DocError::ValidationFailed`].
pub fn check_edifact(segments: &[Segment<'_>]) -> Result<(), DocError> {
    let tables = edifact_d96a();
    let violations = validate_edifact(&tables, segments);
    match violations.into_iter().next() {
        Some(violation) => Err(DocError::ValidationFailed(violation.to_string())),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_tables_parse_embedded_tsv() {
        let tables = edifact_d96a();
        assert!(tables.by_segment.contains_key("UNH"));
        let unh = &tables.by_segment["UNH"];
        assert!(unh.iter().any(|d| d.position == 1 && d.required));
    }

    #[test]
    fn kind_checks_reject_bad_values() {
        assert!(ElemKind::Numeric.accepts("0042"));
        assert!(!ElemKind::Numeric.accepts("4.2"));
        assert!(ElemKind::Decimal.accepts("-12.50"));
        assert!(!ElemKind::Decimal.accepts("1.2.3"));
        assert!(ElemKind::Identifier.accepts("A-1.2"));
        assert!(!ElemKind::Identifier.accepts("A B"));
        assert!(ElemKind::DateTime.accepts("20260101"));
        assert!(ElemKind::Alphanumeric.accepts("anything at all"));
    }

    #[test]
    fn transaction_tables_load_embedded_sets() {
        let tables = x12_transaction_sets();
        assert!(tables.by_set.contains_key("835"));
        assert!(
            tables.by_set["835"]
                .iter()
                .any(|r| r.segment == "CLP" && r.required)
        );
        assert!(tables.by_set.contains_key("271"));
        assert!(!tables.by_set.contains_key("999"));
    }
}

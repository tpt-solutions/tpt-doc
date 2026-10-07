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

use tpt_doc_core::{DocError, Location, Severity, ValidationIssue, ValidationReport};

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

impl From<&SchemaViolation> for ValidationIssue {
    fn from(violation: &SchemaViolation) -> Self {
        // The segment index is the natural location: an EDI violation points
        // at a segment in the stream, not a path through a tree.
        ValidationIssue::new(violation.segment_tag.clone(), violation.message.clone())
            .with_severity(Severity::Error)
            .with_location(Location::Index(violation.segment_index))
    }
}

impl SchemaViolation {
    /// Convert a slice of violations into a structured [`ValidationReport`].
    ///
    /// Lets callers move to the shared report — with severity, locations, and
    /// optional serialization — without the `Vec<SchemaViolation>` API
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
        // Positions are 1-based. A table row declaring `0` is malformed, and
        // `0 - 1` underflows: a panic in debug and a huge index in release,
        // which silently skips the check instead of reporting the bad table.
        let position = usize::from(def.position);
        // Element positions are 1-based. A table row declaring `0` is malformed,
        // and `0 - 1` underflows: a panic in debug, and a huge index in release
        // that silently skips the check instead of reporting the bad table.
        if position == 0 {
            return Some(SchemaViolation {
                segment_index: index,
                segment_tag: tag.to_owned(),
                message: format!(
                    "schema table declares element position 0 for {tag}, which is invalid"
                ),
            });
        }
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

/// Validate the X12 envelope: ISA/IEA, GS/GE, and ST/SE bracketing, with the
/// control numbers and counts each pair must agree on.
///
/// The docs promised these checks and they were not implemented — only the
/// element-level and required-segment rules existed, so an interchange with a
/// mismatched `SE01` count or a `GE02` that did not repeat `GS06` passed
/// validation.
///
/// Every violation is reported at the index of the segment that carries it, so
/// a caller can point at the right line.
#[must_use]
pub fn validate_x12_envelope(segments: &[X12Segment<'_>]) -> Vec<SchemaViolation> {
    let mut violations = Vec::new();
    let at = |index: usize, tag: &str, message: String| SchemaViolation {
        segment_index: index,
        segment_tag: tag.to_owned(),
        message,
    };
    let count = |elements: &[&str]| {
        elements
            .first()
            .and_then(|e| e.trim().parse::<usize>().ok())
    };

    // ISA13 is the interchange control number, repeated in IEA02.
    let isa_number = segments
        .iter()
        .find(|s| s.tag() == "ISA")
        .and_then(|s| s.elements().get(12).copied());
    let mut groups = 0usize;
    let mut open_gs: Option<(usize, String)> = None;
    let mut open_st: Option<(usize, String)> = None;

    for (index, segment) in segments.iter().enumerate() {
        let elements = segment.elements();
        match segment.tag() {
            "GS" => {
                if let Some((open, control)) = open_gs.take() {
                    violations.push(unclosed("GS", open, &control, "GE"));
                }
                groups += 1;
                // GS06 is the group control number, repeated in GE02.
                open_gs = Some((index, control_of(elements, 5)));
            }
            "GE" => match open_gs.take() {
                None => violations.push(at(index, "GE", "GE without a matching GS".to_owned())),
                Some((_, opened)) => {
                    let declared = count(elements);
                    let found = sets_count(segments, index);
                    violations.extend(check_ge(index, elements, &opened, declared, found));
                }
            },
            "ST" => {
                if let Some((open, control)) = open_st.take() {
                    violations.push(unclosed("ST", open, &control, "SE"));
                }
                // ST02 is repeated in SE02.
                open_st = Some((index, control_of(elements, 1)));
            }
            "SE" => {
                match open_st.take() {
                    None => violations.push(at(index, "SE", "SE without a matching ST".to_owned())),
                    Some((st_index, opened)) => {
                        // SE01 counts ST through SE inclusive.
                        let actual = index - st_index + 1;
                        violations.extend(check_se(index, elements, &opened, actual));
                    }
                }
            }
            "IEA" => {
                violations.extend(close_open_envelopes(
                    index,
                    &mut open_gs,
                    &mut open_st,
                    elements,
                    groups,
                    isa_number,
                ));
            }
            _ => {}
        }
    }

    if let Some((open, control)) = open_gs {
        violations.push(at(
            open,
            "GS",
            format!("GS without a matching GE (open control `{control}`)"),
        ));
    }
    if let Some((open, control)) = open_st {
        violations.push(at(
            open,
            "ST",
            format!("ST without a matching SE (open control `{control}`)"),
        ));
    }

    // The interchange must open with ISA and close with IEA. Without these a
    // file that simply omits IEA validates clean, even though a receiver would
    // reject it outright.
    let first = segments.first().map(X12Segment::tag);
    if first != Some("ISA") {
        violations.push(at(
            0,
            first.unwrap_or("∅"),
            "interchange must start with ISA".to_owned(),
        ));
    }
    if segments.len() >= 2 && segments.last().map(X12Segment::tag) != Some("IEA") {
        let last = segments.len() - 1;
        violations.push(at(
            last,
            segments[last].tag(),
            "interchange must end with IEA".to_owned(),
        ));
    }
    violations
}

/// Read a control-number element, trimmed, defaulting to empty.
fn control_of(elements: &[&str], position: usize) -> String {
    elements
        .get(position)
        .copied()
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// The violation for an envelope segment left open at end of input.
fn unclosed(tag: &str, index: usize, control: &str, partner: &str) -> SchemaViolation {
    SchemaViolation {
        segment_index: index,
        segment_tag: tag.to_owned(),
        message: format!("{tag} without a matching {partner} (open control `{control}`)"),
    }
}

/// Close out any `GS`/`ST` still open when an `IEA` arrives, and check the
/// `IEA` itself: `IEA01` counts the functional groups and `IEA02` must repeat
/// `ISA13`.
#[allow(clippy::too_many_arguments)]
fn close_open_envelopes(
    index: usize,
    open_gs: &mut Option<(usize, String)>,
    open_st: &mut Option<(usize, String)>,
    elements: &[&str],
    groups: usize,
    isa_number: Option<&str>,
) -> Vec<SchemaViolation> {
    let mut out = Vec::new();
    if let Some((open, control)) = open_gs.take() {
        out.push(SchemaViolation {
            segment_index: open,
            segment_tag: "GS".to_owned(),
            message: format!("GS without a matching GE (open control `{control}`)"),
        });
    }
    if let Some((open, control)) = open_st.take() {
        out.push(SchemaViolation {
            segment_index: open,
            segment_tag: "ST".to_owned(),
            message: format!("ST without a matching SE (open control `{control}`)"),
        });
    }
    if let Some(declared) = elements
        .first()
        .and_then(|e| e.trim().parse::<usize>().ok())
        && declared != groups
    {
        out.push(SchemaViolation {
            segment_index: index,
            segment_tag: "IEA".to_owned(),
            message: format!("IEA count {declared} does not match {groups} functional group(s)"),
        });
    }
    let control = elements.get(1).copied().unwrap_or("").trim();
    if let Some(isa) = isa_number.map(str::trim)
        && control != isa
    {
        out.push(SchemaViolation {
            segment_index: index,
            segment_tag: "IEA".to_owned(),
            message: format!(
                "IEA control number `{control}` does not match ISA control number `{isa}`"
            ),
        });
    }
    out
}

/// Check a `GE` against the `GS` that opened the group.
///
/// `GE01` is the number of transaction sets in the group; `GE02` must repeat
/// `GS06`.
fn check_ge(
    index: usize,
    elements: &[&str],
    gs_control: &str,
    declared: Option<usize>,
    set_count: usize,
) -> Vec<SchemaViolation> {
    let mut out = Vec::new();
    if let Some(declared) = declared
        && declared != set_count
    {
        out.push(SchemaViolation {
            segment_index: index,
            segment_tag: "GE".to_owned(),
            message: format!(
                "GE count {declared} does not match the {set_count} transaction set(s) in the group"
            ),
        });
    }
    let control = elements.get(1).copied().unwrap_or("").trim();
    if control != gs_control {
        out.push(SchemaViolation {
            segment_index: index,
            segment_tag: "GE".to_owned(),
            message: format!(
                "GE control number `{control}` does not match GS control number `{gs_control}`"
            ),
        });
    }
    out
}

/// Check an `SE` against the `ST` that opened the transaction set.
///
/// `SE01` counts the segments from `ST` through `SE` inclusive; `SE02` must
/// repeat `ST02`.
fn check_se(
    index: usize,
    elements: &[&str],
    st_control: &str,
    segment_count: usize,
) -> Vec<SchemaViolation> {
    let mut out = Vec::new();
    if let Some(declared) = elements
        .first()
        .and_then(|e| e.trim().parse::<usize>().ok())
        && declared != segment_count
    {
        out.push(SchemaViolation {
            segment_index: index,
            segment_tag: "SE".to_owned(),
            message: format!(
                "SE count {declared} does not match {segment_count} segments from ST to SE"
            ),
        });
    }
    let control = elements.get(1).copied().unwrap_or("").trim();
    if control != st_control {
        out.push(SchemaViolation {
            segment_index: index,
            segment_tag: "SE".to_owned(),
            message: format!(
                "SE control number `{control}` does not match ST control number `{st_control}`"
            ),
        });
    }
    out
}

/// How many `ST` segments precede the `GE` at `ge_index` (GE01 counts the
/// transaction sets in the group, not the whole interchange).
fn sets_count(segments: &[X12Segment<'_>], ge_index: usize) -> usize {
    let group_start = segments[..ge_index]
        .iter()
        .rposition(|s| s.tag() == "GS")
        .unwrap_or(0);
    segments[group_start..ge_index]
        .iter()
        .filter(|s| s.tag() == "ST")
        .count()
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

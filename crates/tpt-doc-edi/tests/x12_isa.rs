//! X12 ISA envelope validation.
//!
//! The delimiters that drive all segment splitting come from fixed positions in
//! the ISA header, so a malformed envelope silently produces garbage segments
//! unless it is rejected up front.

use tpt_doc_core::{Location, Severity};
use tpt_doc_edi::schema::{SchemaViolation, validate_x12_envelope};
use tpt_doc_edi::x12::{X12Parser, X12Segment};

/// A well-formed 106-byte ISA, `*` element / `:` component / `~` terminator.
const VALID_ISA: &[u8] =
    b"ISA*00*          *00*          *ZZ*SENDERID       *ZZ*RECEIVERID     *260101*0900*^*00501*000000001*0*P*:~";

fn with_byte(index: usize, byte: u8) -> Vec<u8> {
    let mut bytes = VALID_ISA.to_vec();
    bytes[index] = byte;
    bytes
}

#[test]
fn valid_isa_is_accepted() {
    assert!(X12Parser::new(VALID_ISA).is_ok());
}

#[test]
fn non_printable_element_separator_is_rejected() {
    // ISA01 = CR would make every segment boundary ambiguous.
    assert!(X12Parser::new(&with_byte(3, b'\r')).is_err());
    assert!(X12Parser::new(&with_byte(3, b'\n')).is_err());
    assert!(X12Parser::new(&with_byte(3, 0x00)).is_err());
}

#[test]
fn element_separator_equal_to_terminator_is_rejected() {
    // ISA01 == ISA terminator means nothing can be split.
    assert!(X12Parser::new(&with_byte(105, b'*')).is_err());
}

#[test]
fn component_separator_equal_to_element_separator_is_rejected() {
    // ISA16 == ISA01 is ambiguous.
    assert!(X12Parser::new(&with_byte(104, b'*')).is_err());
}

#[test]
fn non_printable_component_separator_is_rejected() {
    assert!(X12Parser::new(&with_byte(104, b'\r')).is_err());
}

#[test]
fn short_input_is_rejected() {
    assert!(X12Parser::new(b"ISA*too short").is_err());
    assert!(X12Parser::new(b"").is_err());
}

#[test]
fn wrong_prefix_is_rejected() {
    let mut bytes = VALID_ISA.to_vec();
    bytes[..3].copy_from_slice(b"NSA");
    assert!(X12Parser::new(&bytes).is_err());
}

/// The repetition separator lives at a fixed offset in the ISA envelope.
#[test]
fn repetition_separator_is_read_from_isa11() {
    let parser = X12Parser::new(VALID_ISA).expect("valid ISA");
    assert_eq!(parser.repetition_separator(), b'^', "ISA11 in the fixture");
}

/// A non-printable ISA11 must be rejected.
#[test]
fn non_printable_repetition_separator_is_rejected() {
    assert!(X12Parser::new(&with_byte(82, b'\r')).is_err());
    assert!(X12Parser::new(&with_byte(82, 0x00)).is_err());
}

/// A truncated trailing segment must be reported, not silently dropped.
#[test]
fn unterminated_trailing_segment_is_an_error() {
    let truncated = format!("{}ST*850*0001", String::from_utf8_lossy(VALID_ISA));
    let result: Result<Vec<_>, _> = X12Parser::new(truncated.as_bytes())
        .expect("valid ISA")
        .collect();
    let err = result.expect_err("missing terminator must be reported");
    assert!(
        err.to_string().contains("unterminated"),
        "error should say so: {err}"
    );
}

#[test]
fn fully_terminated_interchange_parses_without_error() {
    let complete = format!(
        "{}GE*1*1~IEA*1*000000001~",
        String::from_utf8_lossy(VALID_ISA)
    );
    let tags: Vec<String> = X12Parser::new(complete.as_bytes())
        .expect("valid ISA")
        .map(|segment| segment.map(|s| s.tag().to_string()))
        .collect::<Result<_, _>>()
        .expect("segments");
    assert_eq!(tags, vec!["ISA", "GE", "IEA"]);
}

/// Build a complete, internally consistent interchange, so a test can mutate
/// one field and know the envelope check has something to catch.
fn interchange(body: &str) -> String {
    format!(
        "{}GS*HP*S*R*20260101*0900*1*X*005010X221~{body}GE*1*1~IEA*1*000000001~",
        String::from_utf8_lossy(VALID_ISA)
    )
}

fn validate(bytes: &str) -> Vec<SchemaViolation> {
    let parser = X12Parser::new(bytes.as_bytes()).expect("valid ISA");
    let segments: Vec<X12Segment<'_>> = parser.map(|segment| segment.expect("segment")).collect();
    validate_x12_envelope(&segments)
}

#[test]
fn a_consistent_interchange_has_no_envelope_violations() {
    let bytes = interchange("ST*835*0001~SE*2*0001~");
    let violations = validate(&bytes);
    assert!(violations.is_empty(), "{violations:?}");
}

/// SE01 counts the segments from ST through SE inclusive.
#[test]
fn se_count_must_match_the_actual_segment_count() {
    let bytes =
        interchange("ST*835*0001~BPR*C*100.00~SE*3*0001~").replace("SE*3*0001", "SE*9*0001");
    let violations = validate(&bytes);
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("SE") && v.message.contains("count")),
        "{violations:?}"
    );
}

/// SE02 must repeat ST02, the transaction-set control number.
#[test]
fn se_control_number_must_match_st() {
    let bytes = interchange("ST*835*0001~SE*1*9999~");
    let violations = validate(&bytes);
    assert!(
        violations.iter().any(|v| v.message.contains("9999")),
        "{violations:?}"
    );
}

/// GE01 counts the transaction sets (ST/SE pairs) in the group.
#[test]
fn ge_count_must_match_the_transaction_set_count() {
    let bytes = interchange("ST*835*0001~SE*2*0001~").replace("GE*1*1", "GE*7*1");
    let violations = validate(&bytes);
    assert!(
        violations.iter().any(|v| v.segment_tag == "GE"),
        "{violations:?}"
    );
}

/// GE02 must repeat GS06, the group control number.
#[test]
fn ge_control_number_must_match_gs() {
    let bytes = interchange("ST*835*0001~SE*2*0001~").replace("GE*1*1", "GE*1*4242");
    let violations = validate(&bytes);
    assert!(
        violations
            .iter()
            .any(|v| v.segment_tag == "GE" && v.message.contains("4242")),
        "{violations:?}"
    );
}

/// IEA01 counts the functional groups; IEA02 must repeat ISA13.
#[test]
fn iea_count_and_control_number_are_checked() {
    let bad_count = interchange("ST*835*0001~SE*2*0001~").replace("IEA*1*", "IEA*4*");
    assert!(
        validate(&bad_count).iter().any(|v| v.segment_tag == "IEA"),
        "{:?}",
        validate(&bad_count)
    );
    let bad_control =
        interchange("ST*835*0001~SE*2*0001~").replace("IEA*1*000000001", "IEA*1*000000009");
    assert!(
        validate(&bad_control)
            .iter()
            .any(|v| v.segment_tag == "IEA"),
        "{:?}",
        validate(&bad_control)
    );
}

/// Every envelope segment must be closed by its partner, and dropping one must
/// be caught rather than silently accepted.
#[test]
fn unpaired_envelope_segments_are_reported() {
    // Dropping GS leaves its GE and IEA unmatched.
    let no_gs = interchange("ST*835*0001~SE*2*0001~")
        .replace("GS*HP*S*R*20260101*0900*1*X*005010X221~", "");
    let violations = validate(&no_gs);
    assert!(
        violations
            .iter()
            .any(|v| v.segment_tag == "GE" && v.message.contains("without a matching GS")),
        "dropping GS should leave GE unmatched: {violations:?}"
    );

    // Dropping GE leaves GS open.
    let no_ge = interchange("ST*835*0001~SE*2*0001~").replace("GE*1*1~", "");
    let violations = validate(&no_ge);
    assert!(
        violations
            .iter()
            .any(|v| v.segment_tag == "GS" && v.message.contains("without a matching GE")),
        "dropping GE should leave GS open: {violations:?}"
    );

    // Dropping IEA leaves any open GS and the counts unreconciled.
    let no_iea = interchange("ST*835*0001~SE*2*0001~").replace("IEA*1*000000001~", "");
    assert!(
        !validate(&no_iea).is_empty(),
        "dropping IEA must be reported"
    );
}

/// An ST with no SE leaves the transaction set open, and the violation must
/// name the ST's own index rather than segment 0.
#[test]
fn an_unclosed_transaction_set_reports_its_own_index() {
    let bytes = interchange("ST*835*0001~BPR*C*100.00~");
    let violations = validate(&bytes);
    let unclosed = violations
        .iter()
        .find(|v| v.message.contains("ST without a matching SE"))
        .expect("unclosed ST reported");
    assert_eq!(unclosed.segment_tag, "ST");
    assert_eq!(
        unclosed.segment_index, 2,
        "should name the ST segment's own index"
    );
}

/// A missing trailing segment must be reported at a real index, not at 0.
#[test]
fn a_missing_trailing_segment_names_a_real_index() {
    let bytes = interchange("ST*835*0001~BPR*C*100.00~");
    let violations = validate(&bytes);
    let bad = violations
        .iter()
        .find(|v| v.message.contains("ST without a matching SE"))
        .expect("unclosed ST reported");
    assert!(
        bad.segment_index > 0,
        "a missing trailing segment must not be reported at index 0: {bad:?}"
    );
    assert_eq!(bad.segment_index, 2, "the ST's own index");
}

/// Format-specific violations convert into the shared report without the
/// `Vec<SchemaViolation>` API disappearing.
#[test]
fn schema_violations_convert_to_a_validation_report() {
    let bytes = interchange("ST*835*0001~SE*2*0001~").replace("SE*2*0001", "SE*2*9999");
    let violations = validate(&bytes);
    assert!(!violations.is_empty());

    let report = SchemaViolation::to_report(&violations);
    assert!(report.has_errors(), "EDI violations are errors");
    let first = report.errors().next().expect("an error");
    assert_eq!(first.severity, Severity::Error);
    // The segment index becomes the location, so a caller can find the line.
    assert!(
        matches!(first.location, Location::Index(_)),
        "expected a segment index, got {:?}",
        first.location
    );
}

//! X12 ISA envelope validation.
//!
//! The delimiters that drive all segment splitting come from fixed positions in
//! the ISA header, so a malformed envelope silently produces garbage segments
//! unless it is rejected up front.

use tpt_doc_edi::x12::X12Parser;

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
    let result: Result<Vec<_>, _> =
        X12Parser::new(truncated.as_bytes()).expect("valid ISA").collect();
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

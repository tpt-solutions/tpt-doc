//! Integration tests: parse the sample ADT and ORU fixtures, validate them
//! against the embedded v2.5.1 definitions, and fuzz the parser.

use tpt_doc_hl7v2::Hl7Parser;
use tpt_doc_hl7v2::schema;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).expect("fixture file")
}

fn parse(bytes: &[u8]) -> Vec<tpt_doc_hl7v2::Segment<'_>> {
    Hl7Parser::new(bytes)
        .collect::<Result<Vec<_>, _>>()
        .expect("parse fixture")
}
/// The five MSH encoding characters must be printable and mutually distinct;
/// otherwise the message cannot be tokenised unambiguously.
#[test]
fn degenerate_msh_delimiters_are_rejected() {
    // MSH-3 equal to MSH-4 (component separator == repetition separator).
    let duplicated = b"MSH|^~~\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\r";
    let result: Result<Vec<_>, _> = Hl7Parser::new(duplicated).collect();
    let err = result.expect_err("repeated encoding character must be rejected");
    assert!(err.to_string().contains("distinct"), "{err}");

    // MSH-2 reusing the field separator.
    let clash = b"MSH||~\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\r";
    assert!(
        Hl7Parser::new(clash)
            .collect::<Result<Vec<_>, _>>()
            .is_err(),
        "MSH-2 must not reuse MSH-1"
    );
}

/// A non-printable encoding character cannot be tokenised. `\x01` is used
/// rather than CR/LF so the segment is not split before MSH-2 is read.
#[test]
fn non_printable_msh_delimiter_is_rejected() {
    let bad = b"MSH|^~\x01\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\r";
    let result: Result<Vec<_>, _> = Hl7Parser::new(bad).collect();
    let err = result.expect_err("non-printable delimiter must be rejected");
    assert!(err.to_string().contains("printable"), "{err}");
}

/// The ordinary default delimiters must keep working.
#[test]
fn standard_delimiters_are_accepted() {
    let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\rPID|1";
    let segments: Vec<_> = Hl7Parser::new(message)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(segments[0].component(9, 1), Some("ADT"));
}

/// HL7 v2.x specifies CR (`\r`) as the segment terminator, but real-world
/// senders emit CR-LF or bare LF. All three must parse identically.
#[test]
fn crlf_and_lf_line_endings_parse_like_cr() {
    let cr = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\rPID|1||12345";

    let from_cr: Vec<_> = Hl7Parser::new(cr).collect::<Result<Vec<_>, _>>().unwrap();
    let tags: Vec<&str> = from_cr.iter().map(|s| s.tag()).collect();
    assert_eq!(tags, vec!["MSH", "PID"]);

    // CR-LF: the LF must be skipped, not left at the start of the next segment.
    let crlf = cr.to_vec();
    let cr_count = crlf.iter().filter(|b| **b == b'\r').count();
    let mut rebuilt = Vec::with_capacity(crlf.len() + cr_count);
    for byte in &crlf {
        rebuilt.push(*byte);
        if *byte == b'\r' {
            rebuilt.push(b'\n');
        }
    }
    let from_crlf: Vec<_> = Hl7Parser::new(&rebuilt)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let crlf_tags: Vec<&str> = from_crlf.iter().map(|s| s.tag()).collect();
    assert_eq!(crlf_tags, vec!["MSH", "PID"]);
    assert_eq!(
        from_crlf[1].field(3),
        Some("12345"),
        "field content must be clean"
    );

    // Bare LF: replace each CR with LF, nothing else.
    let lf: Vec<u8> = cr
        .iter()
        .map(|b| if *b == b'\r' { b'\n' } else { *b })
        .collect();
    let from_lf: Vec<_> = Hl7Parser::new(&lf).collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        from_lf.iter().map(|s| s.tag()).collect::<Vec<_>>(),
        vec!["MSH", "PID"]
    );
    assert_eq!(from_lf[1].field(3), Some("12345"));
}

/// A bare LF after CR (CR-LF) must not leave an empty leading segment.
#[test]
fn crlf_does_not_produce_empty_segments() {
    let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|M1|P|2.5.1\r\nPID|1||12345\r\n";
    let segments: Vec<_> = Hl7Parser::new(message)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(segments.len(), 2, "no empty segments for {segments:?}");
    assert_eq!(segments[1].tag(), "PID");
    assert_eq!(segments[1].field(1), Some("1"));
}

/// A long run of empty segments must not recurse per segment — that overflows
/// the stack on untrusted input.
#[test]
fn long_run_of_empty_segments_does_not_overflow() {
    // A valid MSH header sets the delimiters, then every segment is empty.
    let mut message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|MSG1|P|2.5.1\r".to_vec();
    message.extend(std::iter::repeat_n(b'\r', 200_000));

    let tags: Vec<String> = Hl7Parser::new(&message)
        .map(|segment| segment.map(|s| s.tag().to_string()))
        .collect::<Result<_, _>>()
        .expect("segments");
    assert_eq!(tags, vec!["MSH"], "only the MSH should survive");
}

#[test]
fn parses_adt_a01_fixture() {
    let bytes = fixture("adt_a01.hl7");
    let segments = parse(&bytes);
    assert_eq!(segments.len(), 4);
    let msh = &segments[0];
    assert_eq!(msh.component(9, 1), Some("ADT"));
    assert_eq!(msh.component(9, 2), Some("A01"));
    assert_eq!(msh.field(10), Some("MSG00001"));
    let pid = &segments[2];
    assert_eq!(pid.component(3, 1), Some("12345"));
    assert_eq!(pid.component(3, 4), Some("NZHPI"));
}

#[test]
fn parses_oru_r01_fixture() {
    let bytes = fixture("oru_r01.hl7");
    let segments = parse(&bytes);
    assert_eq!(segments.len(), 6);
    assert_eq!(segments[0].component(9, 1), Some("ORU"));
    let obx = &segments[3];
    assert_eq!(obx.field(2), Some("NM"));
    assert_eq!(obx.field(5), Some("5.2"));
}

#[test]
fn fixtures_pass_v251_schema_validation() {
    for name in ["adt_a01.hl7", "oru_r01.hl7"] {
        let bytes = fixture(name);
        let segments = parse(&bytes);
        let violations = schema::validate_message(&schema::v251(), &segments);
        assert!(
            violations.is_empty(),
            "{name}: unexpected violations {violations:?}"
        );
    }
}

#[test]
fn missing_mandatory_fields_are_reported() {
    // PID without PID-3 (patient identifier list) or PID-5 (name).
    let message = b"MSH|^~\\&|A|B|C|D|20261001||ADT^A01|X1|P|2.5.1\rPID|1\r";
    let bytes: &[u8] = message;
    let segments = parse(bytes);
    let violations = schema::validate_message(&schema::v251(), &segments);
    let missing: Vec<_> = violations.iter().map(|v| v.message.as_str()).collect();
    assert!(missing.iter().any(|m| m.contains("PID-3")), "{missing:?}");
    assert!(missing.iter().any(|m| m.contains("PID-5")), "{missing:?}");
}

#[test]
fn message_not_starting_with_msh_is_reported() {
    let bytes: &[u8] = b"PID|1||X^^^NZHPI^MR||Smith^John\r";
    let segments = parse(bytes);
    let violations = schema::validate_message(&schema::v251(), &segments);
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("must start with MSH")),
        "{violations:?}"
    );
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn hl7_parser_never_panics_on_arbitrary_bytes(input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let segments: Vec<_> = Hl7Parser::new(&input).filter_map(Result::ok).collect();
            let _ = segments.len();
        }

        #[test]
        fn encoded_message_round_trips(
            app in r"[A-Z0-9]{1,8}",
            control_id in r"[A-Z0-9]{1,10}",
            patient_id in r"[0-9]{1,8}",
        ) {
            let mut message = tpt_doc_hl7v2::Message::msh(
                &app, "FAC", "RIS", "FAC", "20261001120000", "ADT^A01", &control_id,
            );
            message.push_segment("PID", &["1", "", &format!("{patient_id}^^^NZHPI^MR"), "", "Smith^John"]);
            let bytes = message.to_bytes();
            let segments: Vec<_> = Hl7Parser::new(&bytes)
                .collect::<Result<Vec<_>, _>>()
                .expect("parse own output");
            assert_eq!(segments[0].field(3), Some(app.as_str()));
            assert_eq!(segments[0].field(10), Some(control_id.as_str()));
            assert_eq!(segments[1].component(3, 4), Some("NZHPI"));
        }
    }
}

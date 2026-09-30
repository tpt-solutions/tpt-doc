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

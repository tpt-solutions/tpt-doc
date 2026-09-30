//! Integration tests: parse the sample EDIFACT and X12 fixtures, validate
//! them against the embedded schema tables, and fuzz the parsers.

use tpt_doc_edi::edifact::EdifactParser;
use tpt_doc_edi::schema;
use tpt_doc_edi::x12::X12Parser;

fn edifact_fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sample.edifact"
    ))
    .expect("fixture file")
}

fn x12_fixture() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sample_835.x12"
    ))
    .expect("fixture file")
}

#[test]
fn parses_edifact_fixture_and_asserts_values() {
    let bytes = edifact_fixture();
    let segments: Vec<_> = EdifactParser::new(&bytes)
        .collect::<Result<Vec<_>, _>>()
        .expect("parse fixture");
    assert_eq!(segments.len(), 9);
    assert_eq!(segments[0].tag(), "UNB");
    assert_eq!(segments[2].tag(), "BGM");
    assert_eq!(segments[4].elements().first().copied(), Some("BY"));
    let qty = &segments[6];
    assert_eq!(qty.tag(), "QTY");
    assert_eq!(qty.elements().first().copied(), Some("21:250"));
}

#[test]
fn edifact_fixture_passes_d96a_schema_validation() {
    let bytes = edifact_fixture();
    let segments: Vec<_> = EdifactParser::new(&bytes)
        .collect::<Result<Vec<_>, _>>()
        .expect("parse fixture");
    let violations = schema::validate_edifact(&schema::edifact_d96a(), &segments);
    assert!(
        violations.is_empty(),
        "unexpected violations: {violations:?}"
    );
}

#[test]
fn edifact_envelope_violations_are_detected() {
    // UNT count disagrees with the actual number of segments.
    let broken: &[u8] =
        b"UNB+UNOA:1+A+B+260101:0900+1'UNH+M1+ORDERS:D:96A:UN'BGM+220+PO1+9'UNT+99+M1'UNZ+1+1'";
    let segments: Vec<_> = EdifactParser::new(broken)
        .collect::<Result<Vec<_>, _>>()
        .expect("parse");
    let violations = schema::validate_edifact(&schema::edifact_d96a(), &segments);
    assert!(
        violations
            .iter()
            .any(|v| v.segment_tag == "UNT" && v.message.contains("UNT count")),
        "expected UNT count violation, got {violations:?}"
    );
}

#[test]
fn parses_x12_fixture_and_asserts_values() {
    let bytes = x12_fixture();
    let segments: Vec<_> = X12Parser::new(&bytes)
        .expect("valid ISA")
        .collect::<Result<Vec<_>, _>>()
        .expect("parse fixture");
    assert_eq!(segments[0].tag(), "ISA");
    assert_eq!(segments[3].tag(), "BPR");
    let clp = segments
        .iter()
        .find(|s| s.tag() == "CLP")
        .expect("CLP present");
    assert_eq!(clp.elements().first().copied(), Some("CLP0001"));
    assert_eq!(clp.elements().get(2).copied(), Some("500"));
}

#[test]
fn x12_fixture_passes_835_transaction_validation() {
    let bytes = x12_fixture();
    let segments: Vec<_> = X12Parser::new(&bytes)
        .expect("valid ISA")
        .collect::<Result<Vec<_>, _>>()
        .expect("parse fixture");

    // Element-level checks over the whole interchange.
    let tables = schema::x12_elements();
    for (index, segment) in segments.iter().enumerate() {
        let violations = tables.check_x12(index, segment);
        assert!(
            violations.is_empty(),
            "violations in {segment:?}: {violations:?}"
        );
    }

    // Transaction-set checks over the ST..SE range.
    let start = segments.iter().position(|s| s.tag() == "ST").expect("ST");
    let end = segments.iter().position(|s| s.tag() == "SE").expect("SE");
    let violations =
        schema::x12_transaction_sets().validate("835", &tables, &segments[start..=end]);
    assert!(
        violations.is_empty(),
        "unexpected violations: {violations:?}"
    );
}

#[test]
fn x12_missing_required_segment_is_reported() {
    let bytes = x12_fixture();
    let segments: Vec<_> = X12Parser::new(&bytes)
        .expect("valid ISA")
        .collect::<Result<Vec<_>, _>>()
        .expect("parse fixture");
    let tables = schema::x12_elements();
    let start = segments.iter().position(|s| s.tag() == "ST").expect("ST");
    let end = segments.iter().position(|s| s.tag() == "SE").expect("SE");
    // Drop the mandatory CLP segment.
    let without_clp: Vec<_> = segments[start..=end]
        .iter()
        .filter(|s| s.tag() != "CLP")
        .cloned()
        .collect();
    let violations = schema::x12_transaction_sets().validate("835", &tables, &without_clp);
    assert!(
        violations
            .iter()
            .any(|v| v.segment_tag == "CLP" && v.message.contains("required segment")),
        "expected missing-CLP violation, got {violations:?}"
    );
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn edifact_parser_never_panics_on_arbitrary_bytes(input in proptest::collection::vec(any::<u8>(), 0..512)) {
            let segments: Vec<_> = EdifactParser::new(&input).filter_map(Result::ok).collect();
            let _ = segments.len();
        }

        #[test]
        fn x12_parser_never_panics_on_arbitrary_bytes(input in proptest::collection::vec(any::<u8>(), 0..512)) {
            if let Ok(parser) = X12Parser::new(&input) {
                let segments: Vec<_> = parser.filter_map(Result::ok).collect();
                let _ = segments.len();
            }
        }

        #[test]
        fn edifact_segment_round_trip(
            reference in r"[A-Z0-9]{1,14}",
            code in r"[A-Z]{1,6}",
        ) {
            let msg = format!("UNH+{reference}+{code}:D:96A:UN'BGM+220+PO1+9'");
            let segments: Vec<_> = EdifactParser::new(msg.as_bytes())
                .collect::<Result<Vec<_>, _>>()
                .expect("parse");
            assert_eq!(segments[0].elements().first().copied(), Some(reference.as_str()));
            assert_eq!(segments[0].elements().get(1).copied(), Some(&*format!("{code}:D:96A:UN")));
        }

        #[test]
        fn x12_element_round_trip(
            claim_id in r"[A-Z0-9]{1,20}",
            charge in r"[0-9]{1,8}",
        ) {
            // A minimal, well-formed 835 interchange carrying arbitrary
            // CLP data: the parser must reproduce it element by element.
            let isa = format!(
                "ISA*00*          *00*          *ZZ*{:<15}*ZZ*{:<15}*260101*0900*^*00501*000000001*0*P*:~",
                "SENDER", "RECEIVER"
            );
            let interchange = format!(
                "{isa}\nGS*HP*SENDER*RECEIVER*20260101*0900*1*X*005010X221A1~\n\
                 ST*835*0001~\nCLP*{claim_id}*1*{charge}*0*CO~\nSE*3*0001~\nGE*1*1~\nIEA*1*000000001~\n"
            );
            let segments: Vec<_> = X12Parser::new(interchange.as_bytes())
                .expect("valid ISA")
                .collect::<Result<Vec<_>, _>>()
                .expect("segments");
            let clp = segments
                .iter()
                .find(|s| s.tag() == "CLP")
                .expect("CLP present");
            assert_eq!(clp.elements().first().copied(), Some(claim_id.as_str()));
            assert_eq!(clp.elements().get(2).copied(), Some(charge.as_str()));
        }
    }
}

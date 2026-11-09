//! X12 999 implementation acknowledgments: built, read back, and refused when a
//! set has no response.

use tpt_doc_edi::validate_x12_envelope;
use tpt_doc_edi::x12::X12Parser;
use tpt_doc_edi::x12_ack::{build_997, build_999, parse_999};

const REMITTANCE: &[u8] = include_bytes!("fixtures/sample_835.x12");

#[test]
fn an_accepted_999_reads_back_with_its_set_response() {
    let ack = build_999(REMITTANCE, true, "000000042", "260101", "0930").expect("build");
    let groups = parse_999(&ack).expect("parse");
    assert_eq!(groups.len(), 1);
    assert!(groups[0].is_accepted());
    assert_eq!(groups[0].sets[0].set_id, "835");
    assert_eq!(groups[0].sets[0].code, "A", "the IK5 response");
}

#[test]
fn a_999_declares_itself_and_passes_the_envelope_validator() {
    let ack = build_999(REMITTANCE, false, "000000043", "260101", "0931").expect("build");
    let text = String::from_utf8(ack.clone()).expect("ascii");
    assert!(text.contains("ST*999*0001~"), "{text}");
    assert!(text.contains("IK5*R~"), "{text}");
    let parsed: Vec<_> = X12Parser::new(&ack)
        .expect("envelope")
        .collect::<Result<_, _>>()
        .expect("segments");
    assert!(validate_x12_envelope(&parsed).is_empty());
}

#[test]
fn a_rejected_999_is_not_accepted() {
    let ack = build_999(REMITTANCE, false, "000000043", "260101", "0931").expect("build");
    let groups = parse_999(&ack).expect("parse");
    assert!(!groups[0].is_accepted());
    assert_eq!(groups[0].sets[0].code, "R");
}

#[test]
fn a_997_is_not_read_as_a_999() {
    // A 997 carries AK5, not IK5, so a 999 reader finds no set response.
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("build");
    let err = parse_999(&ack).expect_err("no IK5 responses");
    assert!(err.to_string().contains("no response code"), "{err}");
}

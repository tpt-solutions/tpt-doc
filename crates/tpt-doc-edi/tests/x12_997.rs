//! X12 997 functional acknowledgments: the envelope is swapped, the codes
//! follow the decision, and the result passes the envelope validator.

use tpt_doc_edi::validate_x12_envelope;
use tpt_doc_edi::x12::X12Parser;
use tpt_doc_edi::x12_ack::build_997;

const REMITTANCE: &[u8] = include_bytes!("fixtures/sample_835.x12");

fn segments(bytes: &[u8]) -> Vec<(String, Vec<String>)> {
    X12Parser::new(bytes)
        .expect("envelope")
        .map(|segment| {
            let segment = segment.expect("segment");
            (
                segment.tag().to_owned(),
                segment.elements().iter().map(|e| (*e).to_owned()).collect(),
            )
        })
        .collect()
}

fn element(segments: &[(String, Vec<String>)], tag: &str, index: usize) -> String {
    segments
        .iter()
        .find(|(t, _)| t == tag)
        .and_then(|(_, e)| e.get(index).cloned())
        .unwrap_or_else(|| panic!("no {tag} element {index}"))
}

#[test]
fn an_accepted_interchange_is_acknowledged_with_swapped_parties() {
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("997");
    let parsed = segments(&ack);
    // The original receiver answers, so ISA06 is RECEIVERID and ISA08 SENDERID.
    assert_eq!(element(&parsed, "ISA", 5).trim_end(), "RECEIVERID");
    assert_eq!(element(&parsed, "ISA", 7).trim_end(), "SENDERID");
    assert_eq!(element(&parsed, "ISA", 12), "000000042");
    assert_eq!(element(&parsed, "GS", 0), "FA");
}

#[test]
fn ak1_names_the_group_and_ak2_the_transaction_set() {
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("997");
    let parsed = segments(&ack);
    assert_eq!(element(&parsed, "AK1", 0), "HP", "GS01 of the original");
    assert_eq!(element(&parsed, "AK1", 1), "1", "GS06 of the original");
    assert_eq!(element(&parsed, "AK2", 0), "835");
    assert_eq!(element(&parsed, "AK2", 1), "0001");
}

#[test]
fn accepted_codes_are_a_and_the_counts_match() {
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("997");
    let parsed = segments(&ack);
    assert_eq!(element(&parsed, "AK5", 0), "A");
    assert_eq!(element(&parsed, "AK9", 0), "A");
    assert_eq!(element(&parsed, "AK9", 1), "1", "sets included");
    assert_eq!(element(&parsed, "AK9", 2), "1", "sets received");
    assert_eq!(element(&parsed, "AK9", 3), "1", "sets accepted");
}

#[test]
fn a_rejection_writes_r_and_accepts_none() {
    let ack = build_997(REMITTANCE, false, "000000043", "260101", "0931").expect("997");
    let parsed = segments(&ack);
    assert_eq!(element(&parsed, "AK5", 0), "R");
    assert_eq!(element(&parsed, "AK9", 0), "R");
    assert_eq!(element(&parsed, "AK9", 3), "0");
}

#[test]
fn the_acknowledgment_passes_the_envelope_validator() {
    let ack = build_997(REMITTANCE, true, "000000042", "260101", "0930").expect("997");
    let parsed: Vec<_> = X12Parser::new(&ack)
        .expect("envelope")
        .collect::<Result<_, _>>()
        .expect("segments");
    let violations = validate_x12_envelope(&parsed);
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn a_control_number_that_is_not_nine_digits_or_fewer_is_refused() {
    assert!(build_997(REMITTANCE, true, "1234567890", "260101", "0930").is_err());
    assert!(build_997(REMITTANCE, true, "12A", "260101", "0930").is_err());
    assert!(build_997(REMITTANCE, true, "", "260101", "0930").is_err());
}

#[test]
fn input_without_a_transaction_set_is_refused() {
    let no_set = b"ISA*00*          *00*          *ZZ*A              *ZZ*B              *260101*0900*^*00501*000000001*0*P*:~GS*HP*A*B*20260101*0900*1*X*005010X221A1~GE*0*1~IEA*0*000000001~";
    assert!(build_997(no_set, true, "000000042", "260101", "0930").is_err());
}

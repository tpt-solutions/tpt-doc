//! EDIFACT CONTRL acknowledgments: built, read back, and refused when wrong.

use tpt_doc_edi::edifact::EdifactParser;
use tpt_doc_edi::edifact_ack::{build_contrl, read_contrl};

const INTERCHANGE: &[u8] = include_bytes!("fixtures/sample.edifact");

fn elements_of(bytes: &[u8], tag: &str) -> Vec<String> {
    EdifactParser::new(bytes)
        .map(|s| s.expect("segment"))
        .find(|s| s.tag() == tag)
        .unwrap_or_else(|| panic!("no {tag}"))
        .elements()
        .iter()
        .map(|e| (*e).to_owned())
        .collect()
}

#[test]
fn an_accepted_interchange_is_acknowledged_with_swapped_parties() {
    let ack = build_contrl(INTERCHANGE, true, "0042", "260101", "0930").expect("build");
    let unb = elements_of(&ack, "UNB");
    assert_eq!(unb[1], "RECEIVER", "the original recipient answers");
    assert_eq!(unb[2], "SENDER");
    assert_eq!(unb[4], "0042");
}

#[test]
fn the_uci_names_the_original_reference_and_the_action() {
    let ack = build_contrl(INTERCHANGE, true, "0042", "260101", "0930").expect("build");
    let uci = elements_of(&ack, "UCI");
    assert_eq!(
        uci[0], "0001",
        "the acknowledged interchange control reference"
    );
    assert_eq!(uci[3], "7", "accepted");
}

#[test]
fn a_rejection_writes_the_rejected_action_code() {
    let ack = build_contrl(INTERCHANGE, false, "0043", "260101", "0931").expect("build");
    assert_eq!(elements_of(&ack, "UCI")[3], "4");
}

#[test]
fn the_message_envelope_counts_its_segments() {
    let ack = build_contrl(INTERCHANGE, true, "0042", "260101", "0930").expect("build");
    assert_eq!(elements_of(&ack, "UNT")[0], "3", "UNH, UCI and UNT");
    assert_eq!(elements_of(&ack, "UNZ")[0], "1");
}

#[test]
fn read_contrl_returns_the_reference_and_decision() {
    let accepted = build_contrl(INTERCHANGE, true, "0042", "260101", "0930").expect("build");
    assert_eq!(
        read_contrl(&accepted).expect("read"),
        ("0001".to_owned(), true)
    );
    let rejected = build_contrl(INTERCHANGE, false, "0043", "260101", "0931").expect("build");
    assert_eq!(
        read_contrl(&rejected).expect("read"),
        ("0001".to_owned(), false)
    );
}

#[test]
fn an_unknown_action_code_is_refused() {
    let ack = build_contrl(INTERCHANGE, true, "0042", "260101", "0930").expect("build");
    let text = String::from_utf8(ack)
        .expect("ascii")
        .replace("UCI+0001+SENDER+RECEIVER+7'", "UCI+0001+SENDER+RECEIVER+9'");
    assert!(read_contrl(text.as_bytes()).is_err());
}

#[test]
fn an_interchange_without_unb_is_refused() {
    assert!(build_contrl(b"UNH+1+ORDERS:D:96A:UN'", true, "1", "260101", "0930").is_err());
}

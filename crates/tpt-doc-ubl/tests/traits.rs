//! The shared `tpt-doc-core` traits must agree with the inherent XML API.

use tpt_doc_core::{DocReader, DocWriter, Validate};
use tpt_doc_ubl::Invoice;

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/peppol_invoice.xml"
    ))
    .expect("fixture file")
}

#[test]
fn from_bytes_matches_from_xml() {
    let bytes = fixture_bytes();
    let via_trait = <Invoice as DocReader>::from_bytes(&bytes).expect("trait parse");
    let inherent = Invoice::from_xml(&bytes).expect("inherent parse");
    assert_eq!(via_trait, inherent);
}

#[test]
fn to_bytes_round_trips_through_from_bytes() {
    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse");
    let bytes = DocWriter::to_bytes(&invoice).expect("write");
    let back = <Invoice as DocReader>::from_bytes(&bytes).expect("read back");
    assert_eq!(back, invoice);
}

#[test]
fn validate_trait_reports_the_same_rules_as_validate_fn() {
    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse");
    let report = Validate::validate(&invoice);
    let direct = tpt_doc_ubl::validate(&invoice);
    assert_eq!(report.errors().count(), direct.len());
    assert!(!report.has_errors(), "the fixture is a valid invoice");
}

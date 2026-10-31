//! Golden output for the parsers, from the fixtures committed in each crate.
//! Regenerate with `TPT_UPDATE_GOLDEN=1 cargo test`.

use std::fmt::Write as _;

use tpt_testkit::tpt_assert_golden;

#[test]
fn hl7v2_adt_a01() {
    let bytes = include_bytes!("../../crates/tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7");
    let mut out = String::new();
    for segment in tpt_doc_hl7v2::Hl7Parser::new(bytes) {
        writeln!(out, "{segment:?}").unwrap();
    }
    tpt_assert_golden!(out, "hl7v2_adt_a01.expected.txt");
}

#[test]
fn hl7v2_oru_r01() {
    let bytes = include_bytes!("../../crates/tpt-doc-hl7v2/tests/fixtures/oru_r01.hl7");
    let mut out = String::new();
    for segment in tpt_doc_hl7v2::Hl7Parser::new(bytes) {
        writeln!(out, "{segment:?}").unwrap();
    }
    tpt_assert_golden!(out, "hl7v2_oru_r01.expected.txt");
}

#[test]
fn edifact_sample() {
    let bytes = include_bytes!("../../crates/tpt-doc-edi/tests/fixtures/sample.edifact");
    let mut out = String::new();
    for segment in tpt_doc_edi::edifact::EdifactParser::new(bytes) {
        writeln!(out, "{segment:?}").unwrap();
    }
    tpt_assert_golden!(out, "edifact_sample.expected.txt");
}

#[test]
fn x12_835() {
    let bytes = include_bytes!("../../crates/tpt-doc-edi/tests/fixtures/sample_835.x12");
    let mut out = String::new();
    for segment in tpt_doc_edi::x12::X12Parser::new(bytes).expect("fixture has an ISA header") {
        writeln!(out, "{segment:?}").unwrap();
    }
    tpt_assert_golden!(out, "x12_835.expected.txt");
}

#[test]
fn fhir_patient_bundle() {
    let bytes = include_bytes!("../../crates/tpt-doc-fhir/tests/fixtures/patient_bundle.json");
    let bundle = tpt_doc_fhir::prelude::Bundle::from_json(bytes).expect("fixture parses");
    let mut out = format!("type: {}\n", bundle.bundle_type);
    for (kind, count) in bundle.counts() {
        writeln!(out, "{kind}: {count}").unwrap();
    }
    tpt_assert_golden!(out, "fhir_patient_bundle.expected.txt");
}

#[test]
fn xlsx_simple() {
    let bytes = include_bytes!("../../crates/tpt-doc-spreadsheet/tests/fixtures/simple.xlsx");
    let mut reader = tpt_doc_spreadsheet::xlsx::XlsxReader::new(bytes).expect("fixture opens");
    let mut out = String::new();
    for row in reader.rows() {
        writeln!(out, "{:?}", row.expect("row parses")).unwrap();
    }
    tpt_assert_golden!(out, "xlsx_simple.expected.txt");
}

#[test]
fn docx_simple() {
    let bytes = include_bytes!("../../crates/tpt-doc-word/tests/fixtures/simple.docx");
    let text = tpt_doc_word::prelude::DocxReader::extract_text(bytes).expect("fixture parses");
    tpt_assert_golden!(text, "docx_simple.expected.txt");
}

#[test]
fn ubl_peppol_invoice() {
    let bytes = include_bytes!("../../crates/tpt-doc-ubl/tests/fixtures/peppol_invoice.xml");
    let invoice = tpt_doc_ubl::prelude::Invoice::from_xml(bytes).expect("fixture parses");
    tpt_assert_golden!(format!("{invoice:#?}\n"), "ubl_peppol_invoice.expected.txt");
}

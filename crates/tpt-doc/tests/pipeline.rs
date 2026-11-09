//! The e-invoice pipeline: a valid invoice comes out as a signed PDF that
//! carries its text; an invalid one is refused before anything is rendered.
#![cfg(feature = "pipeline")]

use ring::rand::SystemRandom;
use tpt_doc::pipeline::invoice_json_to_signed_pdf;
use tpt_doc::sign::PrivateKey;
use tpt_doc::ubl::Invoice;

const KEY: &[u8] = include_bytes!("../../tpt-doc-sign/tests/fixtures/test_key.pem");
const CERT: &[u8] = include_bytes!("../../tpt-doc-sign/tests/fixtures/test_cert.pem");
const SIGNING_TIME: i64 = 1_794_182_400;

fn invoice_json() -> Vec<u8> {
    let xml = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tpt-doc-ubl/tests/fixtures/peppol_invoice.xml"
    ))
    .expect("fixture");
    let invoice = Invoice::from_xml(&xml).expect("parse fixture");
    serde_json::to_vec(&invoice).expect("json")
}

fn key_and_cert() -> (PrivateKey, Vec<u8>) {
    (
        PrivateKey::from_pem(KEY).expect("key"),
        tpt_doc::sign::pem::pem_to_der(CERT).expect("cert"),
    )
}

#[test]
fn a_valid_invoice_becomes_a_signed_factur_x_pdf() {
    let (key, cert) = key_and_cert();
    let pdf = invoice_json_to_signed_pdf(
        &invoice_json(),
        &key,
        &cert,
        &SystemRandom::new(),
        SIGNING_TIME,
    )
    .expect("pipeline");
    assert!(pdf.starts_with(b"%PDF"));
    assert!(
        pdf.windows(10).any(|w| w == b"/ByteRange"),
        "the PDF is signed"
    );
    let text = tpt_doc::pdf::extract::extract_text(&pdf).expect("extract");
    assert!(text.contains("Invoice"), "{text}");
}

#[test]
fn an_invoice_that_breaks_a_rule_is_refused() {
    let (key, cert) = key_and_cert();
    // An empty invoice id breaks a mandatory rule, so nothing is rendered.
    let mut value: serde_json::Value = serde_json::from_slice(&invoice_json()).expect("json");
    value["id"] = serde_json::Value::String(String::new());
    let broken = serde_json::to_vec(&value).expect("json");
    let err = invoice_json_to_signed_pdf(&broken, &key, &cert, &SystemRandom::new(), SIGNING_TIME)
        .expect_err("broken invoice");
    assert!(err.to_string().contains("rule"), "{err}");
}

#[test]
fn text_that_is_not_an_invoice_is_refused() {
    let (key, cert) = key_and_cert();
    let err = invoice_json_to_signed_pdf(
        b"{\"hello\":1}",
        &key,
        &cert,
        &SystemRandom::new(),
        SIGNING_TIME,
    )
    .expect_err("not an invoice");
    assert!(err.to_string().contains("not an invoice"), "{err}");
}

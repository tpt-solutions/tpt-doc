//! X12 810 invoices: header, lines, total in cents, and the CTT count check.

use tpt_doc_edi::validate_x12_envelope;
use tpt_doc_edi::x12::X12Parser;
use tpt_doc_edi::x12_810::parse_810;

const INVOICE: &[u8] = include_bytes!("fixtures/sample_810.x12");

#[test]
fn the_fixture_passes_the_envelope_validator() {
    let segments: Vec<_> = X12Parser::new(INVOICE)
        .expect("envelope")
        .collect::<Result<_, _>>()
        .expect("segments");
    let violations = validate_x12_envelope(&segments);
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn the_header_is_read() {
    let invoice = parse_810(INVOICE).expect("parse");
    assert_eq!(invoice.number, "INV5001");
    assert_eq!(invoice.date, "20260102");
    assert_eq!(invoice.purchase_order.as_deref(), Some("PO12345"));
    assert_eq!(invoice.buyer.as_deref(), Some("ACME BUYER"));
}

#[test]
fn the_lines_and_total_are_read() {
    let invoice = parse_810(INVOICE).expect("parse");
    assert_eq!(invoice.lines.len(), 2);
    assert_eq!(invoice.lines[0].quantity, "10");
    assert_eq!(invoice.lines[0].unit_price, "2.50");
    assert_eq!(invoice.lines[1].products[0].1, "GADGET-9");
    assert_eq!(invoice.total_cents, Some(8500));
}

#[test]
fn a_total_that_is_not_cents_is_refused() {
    let text = String::from_utf8(INVOICE.to_vec())
        .expect("ascii")
        .replace("TDS*8500~", "TDS*85.00~");
    assert!(parse_810(text.as_bytes()).is_err());
}

#[test]
fn a_count_that_disagrees_with_the_lines_is_refused() {
    let text = String::from_utf8(INVOICE.to_vec())
        .expect("ascii")
        .replace("CTT*2~", "CTT*5~");
    let err = parse_810(text.as_bytes()).expect_err("count mismatch");
    assert!(err.to_string().contains("counts 5 lines"), "{err}");
}

#[test]
fn an_850_is_not_read_as_an_invoice() {
    let text = String::from_utf8(INVOICE.to_vec())
        .expect("ascii")
        .replace("ST*810*0001~", "ST*850*0001~");
    assert!(parse_810(text.as_bytes()).is_err());
}

//! X12 850 purchase orders: header, buyer, lines, and the CTT count check.

use tpt_doc_edi::validate_x12_envelope;
use tpt_doc_edi::x12::X12Parser;
use tpt_doc_edi::x12_850::parse_850;

const ORDER: &[u8] = include_bytes!("fixtures/sample_850.x12");

#[test]
fn the_fixture_passes_the_envelope_validator() {
    let segments: Vec<_> = X12Parser::new(ORDER)
        .expect("envelope")
        .collect::<Result<_, _>>()
        .expect("segments");
    let violations = validate_x12_envelope(&segments);
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn the_header_and_buyer_are_read() {
    let order = parse_850(ORDER).expect("parse");
    assert_eq!(order.number, "PO12345");
    assert_eq!(order.date, "20260101");
    assert_eq!(order.buyer.as_deref(), Some("ACME BUYER"));
}

#[test]
fn each_line_keeps_its_quantity_price_and_product() {
    let order = parse_850(ORDER).expect("parse");
    assert_eq!(order.lines.len(), 2);
    let first = &order.lines[0];
    assert_eq!(first.line_id, "1");
    assert_eq!(first.quantity, "10");
    assert_eq!(first.unit, "EA");
    assert_eq!(first.unit_price, "2.50", "the price is kept as sent");
    assert_eq!(first.products, [("VP".to_owned(), "WIDGET-1".to_owned())]);
    assert_eq!(order.lines[1].products[0].1, "GADGET-9");
}

#[test]
fn a_count_that_disagrees_with_the_lines_is_refused() {
    let text = String::from_utf8(ORDER.to_vec())
        .expect("ascii")
        .replace("CTT*2~", "CTT*3~");
    let err = parse_850(text.as_bytes()).expect_err("count mismatch");
    assert!(err.to_string().contains("counts 3 lines"), "{err}");
}

#[test]
fn an_order_without_a_beg_is_refused() {
    let text = String::from_utf8(ORDER.to_vec())
        .expect("ascii")
        .replace("BEG*00*SA*PO12345**20260101~", "");
    let err = parse_850(text.as_bytes()).expect_err("no BEG");
    assert!(err.to_string().contains("BEG"), "{err}");
}

#[test]
fn a_transaction_set_that_is_not_an_850_is_refused() {
    let text = String::from_utf8(ORDER.to_vec())
        .expect("ascii")
        .replace("ST*850*0001~", "ST*855*0001~");
    assert!(parse_850(text.as_bytes()).is_err());
}

#[test]
fn a_line_without_a_quantity_is_refused() {
    let text = String::from_utf8(ORDER.to_vec())
        .expect("ascii")
        .replace("PO1*2*5*CS*12.00**VP*GADGET-9~", "PO1*2~");
    assert!(parse_850(text.as_bytes()).is_err());
}

//! EDIFACT ORDERS, INVOIC and DESADV: header, lines, quantities, and the UNT check.

use tpt_doc_edi::edifact_docs::{MessageKind, parse_document};

const ORDERS: &[u8] = include_bytes!("fixtures/sample_orders.edifact");
const INVOIC: &[u8] = include_bytes!("fixtures/sample_invoic.edifact");
const DESADV: &[u8] = include_bytes!("fixtures/sample_desadv.edifact");

fn replaced(source: &[u8], from: &str, to: &str) -> Vec<u8> {
    String::from_utf8(source.to_vec())
        .expect("ascii")
        .replace(from, to)
        .into_bytes()
}

#[test]
fn an_order_reads_its_header_and_lines() {
    let order = parse_document(ORDERS).expect("parse");
    assert_eq!(order.kind, MessageKind::Orders);
    assert_eq!(order.number, "PO12345");
    assert_eq!(order.date, "20260101");
    assert_eq!(order.buyer.as_deref(), Some("BUYER-LTD"));
    assert_eq!(order.amount_due, None);
    assert_eq!(order.lines.len(), 2);
    assert_eq!(order.lines[0].item, "WIDGET-1");
    assert_eq!(order.lines[0].quantity.as_deref(), Some("10"));
    assert_eq!(order.lines[1].line_number, "2");
    assert_eq!(order.lines[1].quantity.as_deref(), Some("5"));
}

#[test]
fn an_invoice_reads_its_amount_due() {
    let invoice = parse_document(INVOIC).expect("parse");
    assert_eq!(invoice.kind, MessageKind::Invoic);
    assert_eq!(invoice.number, "INV5001");
    assert_eq!(invoice.amount_due.as_deref(), Some("85.00"));
    assert_eq!(invoice.lines[0].quantity.as_deref(), Some("10"));
}

#[test]
fn a_unt_count_that_disagrees_is_refused() {
    let err = parse_document(&replaced(ORDERS, "UNT+9+", "UNT+8+")).expect_err("count");
    assert!(err.to_string().contains("UNT counts 8"), "{err}");
}

#[test]
fn a_message_of_another_kind_is_refused() {
    let err = parse_document(&replaced(ORDERS, "ORDERS:D:96A", "APERAK:D:96A"))
        .expect_err("APERAK is not supported");
    assert!(err.to_string().contains("APERAK"), "{err}");
}

#[test]
fn a_line_without_an_item_number_is_refused() {
    assert!(parse_document(&replaced(ORDERS, "LIN+2++GADGET-9:VP'", "LIN+2'")).is_err());
}

#[test]
fn a_despatch_advice_reads_its_note_and_despatched_quantities() {
    let note = parse_document(DESADV).expect("parse");
    assert_eq!(note.kind, MessageKind::Desadv);
    assert_eq!(note.number, "DN7001");
    assert_eq!(note.lines.len(), 2);
    assert_eq!(note.lines[0].quantity.as_deref(), Some("10"));
}

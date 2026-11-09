//! X12 856 advance ship notices: header, item levels, and the CTT count.

use tpt_doc_edi::validate_x12_envelope;
use tpt_doc_edi::x12::X12Parser;
use tpt_doc_edi::x12_856::parse_856;

const NOTICE: &[u8] = include_bytes!("fixtures/sample_856.x12");

fn replaced(from: &str, to: &str) -> Vec<u8> {
    String::from_utf8(NOTICE.to_vec())
        .expect("ascii")
        .replace(from, to)
        .into_bytes()
}

#[test]
fn the_fixture_passes_the_envelope_validator() {
    let segments: Vec<_> = X12Parser::new(NOTICE)
        .expect("envelope")
        .collect::<Result<_, _>>()
        .expect("segments");
    assert!(validate_x12_envelope(&segments).is_empty());
}

#[test]
fn the_shipment_header_is_read() {
    let notice = parse_856(NOTICE).expect("parse");
    assert_eq!(notice.shipment_id, "SHIP100");
    assert_eq!(notice.date, "20260105");
    assert_eq!(notice.time, "1200");
}

#[test]
fn only_item_levels_become_items() {
    let notice = parse_856(NOTICE).expect("parse");
    assert_eq!(
        notice.items.len(),
        2,
        "the shipment and order levels are not items"
    );
    assert_eq!(notice.items[0].level_id, "3");
    assert_eq!(
        notice.items[0].products,
        [("VP".to_owned(), "WIDGET-1".to_owned())]
    );
    assert_eq!(notice.items[0].quantity, "10");
    assert_eq!(notice.items[0].unit, "EA");
    assert_eq!(notice.items[1].products[0].1, "GADGET-9");
    assert_eq!(notice.items[1].quantity, "5");
}

#[test]
fn a_ctt_that_disagrees_with_the_hl_count_is_refused() {
    let err = parse_856(&replaced("CTT*4~", "CTT*3~")).expect_err("count mismatch");
    assert!(err.to_string().contains("counts 3 HL"), "{err}");
}

#[test]
fn an_item_without_a_quantity_is_refused() {
    assert!(parse_856(&replaced("SN1**5*EA~", "")).is_err());
}

#[test]
fn an_810_is_not_read_as_a_ship_notice() {
    assert!(parse_856(&replaced("ST*856*0001~", "ST*810*0001~")).is_err());
}

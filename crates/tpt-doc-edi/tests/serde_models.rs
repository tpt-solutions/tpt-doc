//! With the `serde` feature the EDIFACT segment model serializes to JSON.
//! Segments borrow from the input, so only serialization is provided.
#![cfg(feature = "serde")]

use tpt_doc_edi::edifact::EdifactParser;

#[test]
fn parsed_segments_serialize_to_json() {
    let input = b"UNH+1+INVOIC:D:96A:UN'";
    let segment = EdifactParser::new(input)
        .next()
        .expect("one segment")
        .expect("valid segment");
    let json = serde_json::to_string(&segment).expect("serialize");
    assert!(json.contains("UNH"), "{json}");
}

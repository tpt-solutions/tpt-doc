//! With the `serde` feature the HL7 message model round-trips through JSON
//! and encodes to the same bytes afterwards.
#![cfg(feature = "serde")]

use tpt_doc_hl7v2::encode::Message;

#[test]
fn message_round_trips_through_json() {
    let mut message = Message::msh(
        "APP",
        "FAC",
        "RCV",
        "RFAC",
        "20260101120000",
        "ADT^A01",
        "42",
    );
    message.push_segment("PID", &["1", "", "12345^^^HOSP^MR"]);
    let json = serde_json::to_string(&message).expect("serialize");
    let back: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.to_bytes(), message.to_bytes());
}

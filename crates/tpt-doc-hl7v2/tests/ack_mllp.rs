//! Acknowledgments answer a message; MLLP frames messages in a byte stream.

use tpt_doc_core::Location;
use tpt_doc_hl7v2::Hl7Parser;
use tpt_doc_hl7v2::ack::{AckCode, ack_for};
use tpt_doc_hl7v2::mllp::{frame, unframe};

const ADT: &[u8] = b"MSH|^~\\&|HIS|ACME|RIS|CITY|20260101120000||ADT^A01|MSG1|P|2.5.1\rPID|1||12345^^^ACME||Doe^Jane\r";

fn parse(bytes: &[u8]) -> Vec<Vec<String>> {
    Hl7Parser::new(bytes)
        .map(|segment| {
            let segment = segment.expect("segment");
            let mut fields = vec![segment.tag().to_owned()];
            fields.extend(segment.raw_fields().iter().map(|f| (*f).to_owned()));
            fields
        })
        .collect()
}

#[test]
fn accept_swaps_the_parties_and_names_the_original() {
    let ack = ack_for(ADT, AckCode::Accept, None, "ACK1", "20260101120001").expect("ack");
    let segments = parse(&ack);
    assert_eq!(segments[0][0], "MSH");
    // Sender and receiver are swapped: the original receiver answers.
    assert_eq!(segments[0].get(3).map(String::as_str), Some("RIS"));
    assert_eq!(segments[0].get(5).map(String::as_str), Some("HIS"));
    // MSH-9 is the trigger of the original, MSH-10 is the new control ID.
    assert!(segments[0].iter().any(|f| f == "ACK^A01"), "{segments:?}");
    assert!(segments[0].iter().any(|f| f == "ACK1"), "{segments:?}");
    let msa = segments.iter().find(|s| s[0] == "MSA").expect("MSA");
    assert_eq!(msa[1], "AA");
    assert_eq!(msa[2], "MSG1", "MSA-2 names the original control ID");
}

#[test]
fn error_carries_the_reason_in_msa_3() {
    let ack = ack_for(
        ADT,
        AckCode::Error,
        Some("PID-3 missing"),
        "ACK2",
        "20260101120002",
    )
    .expect("ack");
    let segments = parse(&ack);
    let msa = segments.iter().find(|s| s[0] == "MSA").expect("MSA");
    assert_eq!(msa[1], "AE");
    assert_eq!(msa[3], "PID-3 missing");
}

#[test]
fn reject_code_is_ar() {
    assert_eq!(AckCode::Reject.code(), "AR");
}

#[test]
fn caret_in_the_reason_is_escaped_not_a_component_separator() {
    let ack = ack_for(
        ADT,
        AckCode::Reject,
        Some("bad^value"),
        "ACK3",
        "20260101120003",
    )
    .expect("ack");
    let segments: Vec<_> = Hl7Parser::new(&ack)
        .collect::<Result<_, _>>()
        .expect("parse");
    let msa = segments.iter().find(|s| s.tag() == "MSA").expect("MSA");
    assert_eq!(msa.component(3, 2), None, "the caret must not split MSA-3");
    assert_eq!(msa.field(3), Some("bad\\S\\value"), "escaped, not raw");
}

#[test]
fn a_document_without_an_msh_is_refused() {
    assert!(ack_for(b"PID|1||12345\r", AckCode::Accept, None, "A", "20260101").is_err());
}

#[test]
fn an_acknowledgment_is_itself_parseable_by_the_schema_check() {
    let ack = ack_for(ADT, AckCode::Accept, None, "ACK4", "20260101120004").expect("ack");
    let segments: Vec<_> = Hl7Parser::new(&ack)
        .collect::<Result<_, _>>()
        .expect("parse");
    assert_eq!(segments.len(), 2);
}

#[test]
fn mllp_frame_round_trips() {
    let framed = frame(ADT);
    assert_eq!(framed.first(), Some(&0x0B));
    assert_eq!(&framed[framed.len() - 2..], &[0x1C, 0x0D]);
    let messages = unframe(&framed).expect("unframe");
    assert_eq!(messages, [ADT]);
}

#[test]
fn a_stream_of_frames_splits_into_messages() {
    let mut stream = frame(b"MSH|one");
    stream.extend(frame(b"MSH|two"));
    let messages = unframe(&stream).expect("unframe");
    assert_eq!(messages, [b"MSH|one".as_slice(), b"MSH|two".as_slice()]);
}

#[test]
fn a_truncated_frame_is_an_error_not_a_short_list() {
    let mut stream = frame(b"MSH|one");
    stream.extend_from_slice(b"\x0bMSH|two");
    let err = unframe(&stream).expect_err("second frame is not terminated");
    assert!(err.to_string().contains("not terminated"), "{err}");
    assert!(matches!(err.location(), Some(Location::Offset(_))));
}

#[test]
fn bytes_outside_a_frame_are_refused_at_their_offset() {
    let mut stream = frame(b"MSH|one");
    stream.push(b'X');
    let err = unframe(&stream).expect_err("stray byte");
    assert_eq!(
        err.location(),
        Some(&Location::Offset(stream.len() as u64 - 1))
    );
}

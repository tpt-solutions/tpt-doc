//! Batches: build, read back, and refuse malformed or truncated streams.

use tpt_doc_core::Location;
use tpt_doc_hl7v2::batch::{BatchHeader, build_batch, read_batch};

fn header() -> BatchHeader {
    BatchHeader {
        sending_app: "HIS".into(),
        sending_facility: "ACME".into(),
        receiving_app: "RIS".into(),
        receiving_facility: "CITY".into(),
        datetime: "20260110120000".into(),
        batch_id: "B-001".into(),
    }
}

const ADT1: &[u8] = b"MSH|^~\\&|HIS|ACME|RIS|CITY|20260110||ADT^A01|M1|P|2.5.1\rPID|1||111\r";
const ADT2: &[u8] = b"MSH|^~\\&|HIS|ACME|RIS|CITY|20260110||ADT^A01|M2|P|2.5.1\rPID|1||222\r";

#[test]
fn a_built_batch_reads_back_to_the_same_messages() {
    let batch = build_batch(&header(), &[ADT1, ADT2]).expect("build");
    let messages = read_batch(&batch).expect("read");
    assert_eq!(messages, [ADT1, ADT2]);
}

#[test]
fn the_batch_starts_with_bhs_and_ends_with_the_count() {
    let batch = build_batch(&header(), &[ADT1, ADT2]).expect("build");
    assert!(
        batch.starts_with(b"BHS|^~\\&|HIS|ACME|RIS|CITY|"),
        "{:?}",
        String::from_utf8_lossy(&batch)
    );
    assert!(
        batch.ends_with(b"BTS|2\r"),
        "{:?}",
        String::from_utf8_lossy(&batch)
    );
}

#[test]
fn a_message_without_msh_cannot_be_batched() {
    assert!(build_batch(&header(), &[b"PID|1||111\r"]).is_err());
}

#[test]
fn a_missing_bts_is_refused() {
    let batch = build_batch(&header(), &[ADT1]).expect("build");
    let without_trailer = &batch[..batch.len() - "BTS|1\r".len()];
    let err = read_batch(without_trailer).expect_err("no trailer");
    assert!(err.to_string().contains("BTS"), "{err}");
}

#[test]
fn a_count_that_disagrees_with_the_messages_is_refused() {
    let batch = build_batch(&header(), &[ADT1, ADT2]).expect("build");
    let text = String::from_utf8(batch)
        .expect("ascii")
        .replace("BTS|2", "BTS|3");
    let err = read_batch(text.as_bytes()).expect_err("count mismatch");
    assert!(err.to_string().contains("counts 3"), "{err}");
}

#[test]
fn a_stream_that_does_not_start_with_a_header_is_refused_at_zero() {
    let err = read_batch(ADT1).expect_err("no batch header");
    assert_eq!(err.location(), Some(&Location::Offset(0)));
}

#[test]
fn a_file_wrapper_around_a_batch_is_accepted() {
    let batch = build_batch(&header(), &[ADT1]).expect("build");
    let mut wrapped = b"FHS|^~\\&|HIS|ACME\r".to_vec();
    wrapped.extend_from_slice(&batch);
    wrapped.extend_from_slice(b"FTS|1\r");
    assert_eq!(read_batch(&wrapped).expect("read"), [ADT1]);
}

#[test]
fn a_caret_in_the_batch_id_is_escaped() {
    let mut h = header();
    h.batch_id = "B^1".into();
    let batch = build_batch(&h, &[ADT1]).expect("build");
    let first_line = batch.split(|b| *b == b'\r').next().expect("line");
    let text = String::from_utf8_lossy(first_line);
    assert!(text.ends_with("|B\\S\\1"), "{text}");
}

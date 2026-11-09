//! The shared package helpers: deterministic output, bounded reads, and
//! clear errors for input that is not a package.

use tpt_doc_core::Limits;
use tpt_doc_ooxml::{PackageWriter, open_archive, read_part};

fn build(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut writer = PackageWriter::new();
    for (name, content) in parts {
        writer.add_part(name, content).expect("add part");
    }
    writer.finish().expect("finish")
}

#[test]
fn identical_parts_give_identical_bytes() {
    let parts = [("a.xml", "<a/>"), ("dir/b.xml", "<b>text</b>")];
    assert_eq!(build(&parts), build(&parts));
}

#[test]
fn a_written_part_reads_back() {
    let bytes = build(&[("word/document.xml", "<w:document/>")]);
    let mut archive = open_archive(&bytes).expect("open");
    let content = read_part(&mut archive, "word/document.xml", &Limits::default()).expect("read");
    assert_eq!(content, b"<w:document/>");
}

#[test]
fn a_missing_part_is_a_named_error() {
    let bytes = build(&[("a.xml", "<a/>")]);
    let mut archive = open_archive(&bytes).expect("open");
    let err = read_part(&mut archive, "b.xml", &Limits::default()).expect_err("missing");
    assert!(err.to_string().contains("b.xml"), "{err}");
}

#[test]
fn bytes_that_are_not_a_zip_are_rejected() {
    let err = open_archive(b"not a zip at all").expect_err("not a zip");
    assert!(err.to_string().contains("ZIP"), "{err}");
}

#[test]
fn a_part_larger_than_the_limit_is_refused() {
    let bytes = build(&[("big.xml", "0123456789")]);
    let mut archive = open_archive(&bytes).expect("open");
    let limits = Limits {
        max_entry_bytes: 4,
        ..Limits::default()
    };
    assert!(read_part(&mut archive, "big.xml", &limits).is_err());
}

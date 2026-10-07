//! Integration tests for `.docx` reading and writing.
//!
//! `tests/fixtures/simple.docx` is built with Python's `zipfile` from
//! hand-written OOXML parts — independent of `DocxWriter` — so the fixture
//! test exercises real ZIP/XML interop against a known-good package.

use tpt_doc_core::Limits;
use tpt_doc_word::prelude::*;

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/simple.docx"
    ))
    .expect("fixture file")
}

#[test]
fn extracts_text_from_independent_fixture() {
    let text = DocxReader::extract_text(&fixture_bytes()).expect("parse fixture");
    let lines: Vec<&str> = text.split('\n').collect();
    assert_eq!(
        lines,
        [
            "Meeting Minutes",
            "Attendees: Aitken & Brown, 50% of the team <present>",
            "Item one\tItem two",
            "Item three",
            "Decision",
            "Owner",
            "Budget approval",
            "Félix",
        ]
    );
}

#[test]
fn writer_output_is_readable_and_complete() {
    let mut doc = DocxDocument::new();
    doc.push_paragraph(Paragraph::styled("Title", "Status Report"));
    doc.push_paragraph(Paragraph::new("First paragraph with <angle> & ampersand."));
    let bytes = DocxWriter::write(&doc).expect("write");

    let text = DocxReader::extract_text(&bytes).expect("extract");
    let lines: Vec<&str> = text.split('\n').collect();
    assert_eq!(
        lines,
        ["Status Report", "First paragraph with <angle> & ampersand."]
    );
}

#[test]
fn non_docx_input_is_rejected_not_panicked_on() {
    assert!(DocxReader::extract_text(b"not a zip file").is_err());
    assert!(DocxReader::extract_text(&[]).is_err());
}

/// A document body is only a few elements deep, so a `document.xml` nested far
/// deeper is refused instead of walked.
#[test]
fn deeply_nested_document_xml_is_rejected() {
    let bytes = docx_with_nesting(64);
    let limits = Limits {
        max_xml_depth: 8,
        ..Limits::default()
    };
    let err = DocxReader::extract_text_with_limits(&bytes, limits)
        .expect_err("deep nesting must be rejected");
    assert!(err.to_string().contains("8 level limit"), "{err}");
}

/// The default limits leave a realistic document untouched: the fixture nests
/// well under `max_xml_depth`.
#[test]
fn default_limits_allow_the_fixture() {
    let limits = Limits::default();
    let text =
        DocxReader::extract_text_with_limits(&fixture_bytes(), limits).expect("fixture parses");
    assert!(text.starts_with("Meeting Minutes"), "{text}");
}

/// Wrap `word/document.xml` nested `depth` levels deep inside a `.docx`.
fn docx_with_nesting(depth: usize) -> Vec<u8> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let body = format!(
        "<w:document><w:body>{}<w:p><w:r><w:t>x</w:t></w:r></w:p>{}</w:body></w:document>",
        "<w:p>".repeat(depth),
        "</w:p>".repeat(depth)
    );
    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("word/document.xml", options)
        .expect("start entry");
    zip.write_all(body.as_bytes()).expect("write entry");
    zip.finish().expect("finish zip").into_inner()
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn xml_safe(s: &str) -> bool {
        s.chars().all(|c| {
            matches!(
                c,
                '\t' | '\n'
                    | '\r'
                    | '\u{20}'..='\u{D7FF}'
                    | '\u{E000}'..='\u{FFFD}'
                    | '\u{10000}'..='\u{10FFFF}'
            )
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn round_trip_paragraph_text(text in any::<String>().prop_filter(
            "xml-safe text",
            |s| xml_safe(s) && s.len() < 128,
        )) {
            let mut doc = DocxDocument::new();
            doc.push_paragraph(Paragraph::new(text.clone()));
            let bytes = DocxWriter::write(&doc).expect("write");
            let extracted = DocxReader::extract_text(&bytes).expect("extract");
            prop_assert_eq!(extracted, text);
        }
    }
}

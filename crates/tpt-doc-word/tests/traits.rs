//! The `DocWriter` implementation must produce the same bytes as the writer.

use tpt_doc_core::DocWriter;
use tpt_doc_word::{DocxDocument, DocxWriter, Paragraph};

#[test]
fn to_bytes_matches_docx_writer() {
    let mut doc = DocxDocument::new();
    doc.push_paragraph(Paragraph::new("hello"));
    let via_trait = DocWriter::to_bytes(&doc).expect("trait write");
    let direct = DocxWriter::write(&doc).expect("direct write");
    assert_eq!(via_trait, direct);
}

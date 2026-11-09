//! The `DocWriter` implementation writes the same bytes as `Document::write`,
//! and the document can still be written more than once.

use tpt_doc_core::DocWriter;
use tpt_doc_pdf::{Document, Font, Page};

fn sample() -> Document {
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text("trait writer", font, 12.0, (72.0, 700.0));
    doc.add_page(page);
    doc
}

#[test]
fn to_bytes_matches_write() {
    let doc = sample();
    assert_eq!(
        DocWriter::to_bytes(&doc).expect("trait"),
        doc.write().expect("write")
    );
}

#[test]
fn a_document_can_be_written_twice() {
    let doc = sample();
    let first = doc.write().expect("first");
    let second = doc.write().expect("second");
    assert_eq!(first, second);
}

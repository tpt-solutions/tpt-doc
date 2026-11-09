//! With the `serde` feature a PDF document model round-trips through JSON
//! and writes the same bytes afterwards.
#![cfg(feature = "serde")]

use tpt_doc_pdf::{Document, Font, Page};

#[test]
fn document_round_trips_through_json() {
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text("Hello, serde", font, 14.0, (72.0, 720.0));
    page.rect(72.0, 700.0, 200.0, 2.0);
    doc.add_page(page);

    let json = serde_json::to_string(&doc).expect("serialize");
    let back: Document = serde_json::from_str(&json).expect("deserialize");
    let original = doc.write().expect("write original");
    let restored = back.write().expect("write restored");
    assert_eq!(restored, original);
}

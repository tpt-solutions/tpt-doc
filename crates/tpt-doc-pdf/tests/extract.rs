//! Text extraction: what the writer put on the page comes back, page by page,
//! including through attachment-bearing files.

use tpt_doc_pdf::extract::{extract_pages, extract_text};
use tpt_doc_pdf::{Document, Font, Page};

fn two_page_document() -> Vec<u8> {
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut first = Page::a4();
    first.text("Hello, world", font, 12.0, (72.0, 760.0));
    first.text("Second line", font, 12.0, (72.0, 740.0));
    doc.add_page(first);
    let mut second = Page::letter();
    second.text(
        "Café (ünïcode) and a back\\slash",
        font,
        12.0,
        (72.0, 700.0),
    );
    doc.add_page(second);
    doc.write().expect("write")
}

#[test]
fn each_text_run_comes_back_in_page_order() {
    let pages = extract_pages(&two_page_document()).expect("extract");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0], "Hello, world\nSecond line");
    assert_eq!(pages[1], "Café (ünïcode) and a back\\slash");
}

#[test]
fn parentheses_backslashes_and_accents_survive() {
    let text = extract_text(&two_page_document()).expect("extract");
    assert!(text.contains("Café (ünïcode) and a back\\slash"), "{text}");
}

#[test]
fn runs_on_the_same_line_are_joined_with_a_space() {
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text("one", font, 12.0, (72.0, 700.0));
    page.text("two", font, 12.0, (120.0, 700.0));
    doc.add_page(page);
    let pages = extract_pages(&doc.write().expect("write")).expect("extract");
    assert_eq!(pages[0], "one two");
}

#[test]
fn a_rendered_invoice_pdf_yields_its_text() {
    // The Factur-X template embeds an XML attachment; the page text is still
    // readable through the attachment-bearing catalog.
    let pdf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../templates/03-ubl-invoice-facturx/expected.pdf"
    ))
    .expect("template pdf");
    let text = extract_text(&pdf).expect("extract");
    assert!(text.contains("Invoice"), "{text}");
}

#[test]
fn a_rendered_contract_yields_its_words() {
    let pdf = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../templates/05-signed-contract/expected.pdf"
    ))
    .expect("template pdf");
    let text = extract_text(&pdf).expect("extract");
    assert!(text.contains("Services Agreement"), "{text}");
    assert!(text.contains("Consulting"), "{text}");
}

#[test]
fn bytes_that_are_not_a_pdf_are_refused() {
    let err = extract_pages(b"just some text").expect_err("not a pdf");
    assert!(err.to_string().contains("PDF"), "{err}");
}

#[test]
fn a_truncated_pdf_is_refused() {
    let pdf = two_page_document();
    let cut = &pdf[..pdf.len() / 2];
    assert!(extract_pages(cut).is_err());
}

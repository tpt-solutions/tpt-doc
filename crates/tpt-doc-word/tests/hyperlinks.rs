//! Hyperlinks: a linked run keeps its address through a write and a read, one
//! relationship is written per distinct address, and a document without links
//! is written as it always was.

use tpt_doc_core::Limits;
use tpt_doc_ooxml::{open_archive, read_part};
use tpt_doc_word::{BodyElement, Cell, DocxDocument, DocxWriter, Paragraph, Row, Run, Table};

const URL: &str = "https://example.com/a?x=1&y=<2>";

fn linked_document() -> DocxDocument {
    let mut doc = DocxDocument::new();
    let mut p = Paragraph::new("See ");
    p.push_run(Run::linked("the site", URL));
    p.push_run(Run::new(" now"));
    doc.push_paragraph(p);

    let mut table = Table::new();
    table.push_row(Row::new([Cell::new("plain"), Cell::new("also plain")]));
    let mut cell = Paragraph::new("");
    cell.push_run(Run::linked("again", URL));
    let mut linked_cell = Cell::new("");
    linked_cell.push_paragraph(cell);
    let mut row = Row::new([Cell::new("x")]);
    row.push_cell(linked_cell);
    table.push_row(row);
    doc.push_table(table);
    doc
}

fn rels_of(bytes: &[u8]) -> String {
    let mut archive = open_archive(bytes).expect("zip");
    let part = read_part(
        &mut archive,
        "word/_rels/document.xml.rels",
        &Limits::default(),
    )
    .expect("rels part");
    String::from_utf8(part).expect("utf-8")
}

#[test]
fn a_linked_run_keeps_its_address_through_a_read() {
    let bytes = DocxWriter::write(&linked_document()).expect("write");
    let doc = DocxDocument::read(&bytes).expect("read");
    let BodyElement::Paragraph(p) = &doc.body()[0] else {
        panic!("first element is a paragraph");
    };
    assert_eq!(p.runs()[1].link.as_deref(), Some(URL));
    assert_eq!(p.runs()[1].text, "the site");
    assert_eq!(p.runs()[2].link, None, "the run after the link is plain");
}

#[test]
fn a_linked_run_inside_a_table_cell_is_read_back() {
    let bytes = DocxWriter::write(&linked_document()).expect("write");
    let doc = DocxDocument::read(&bytes).expect("read");
    let BodyElement::Table(table) = &doc.body()[1] else {
        panic!("second element is a table");
    };
    // The cell's second paragraph is an empty run followed by the linked run.
    let cell_run = &table.rows()[1].cells()[1].paragraphs()[1].runs()[1];
    assert_eq!(cell_run.link.as_deref(), Some(URL));
}

#[test]
fn the_same_address_is_one_relationship() {
    let rels = rels_of(&DocxWriter::write(&linked_document()).expect("write"));
    assert_eq!(rels.matches("relationships/hyperlink").count(), 1, "{rels}");
    assert!(rels.contains("TargetMode=\"External\""));
    assert!(
        rels.contains("https://example.com/a?x=1&amp;y=&lt;2&gt;"),
        "{rels}"
    );
}

#[test]
fn a_document_with_links_survives_write_read_write() {
    let original = DocxWriter::write(&linked_document()).expect("write");
    let read_back = DocxDocument::read(&original).expect("read");
    assert_eq!(
        DocxWriter::write(&read_back).expect("write again"),
        original
    );
}

#[test]
fn a_document_without_links_has_no_hyperlink_relationships() {
    let mut doc = DocxDocument::new();
    doc.push_paragraph(Paragraph::new("plain text"));
    let rels = rels_of(&DocxWriter::write(&doc).expect("write"));
    assert!(!rels.contains("hyperlink"), "{rels}");
}

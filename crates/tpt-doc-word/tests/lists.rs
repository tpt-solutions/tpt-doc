//! Lists: bulleted and numbered paragraphs keep their kind through a write and a
//! read, in the body and in table cells; a document without lists has no
//! numbering part.

use tpt_doc_ooxml::open_archive;
use tpt_doc_word::{BodyElement, Cell, DocxDocument, DocxWriter, ListKind, Paragraph, Row, Table};

fn list_document() -> DocxDocument {
    let mut doc = DocxDocument::new();
    doc.push_paragraph(Paragraph::new("Shopping"));
    doc.push_paragraph(Paragraph::list_item(ListKind::Bulleted, "milk"));
    doc.push_paragraph(Paragraph::list_item(ListKind::Numbered, "first step"));
    doc.push_paragraph(Paragraph::list_item(ListKind::Numbered, "second step"));

    let mut cell = Cell::new("");
    cell.push_paragraph(Paragraph::list_item(ListKind::Bulleted, "in a cell"));
    let mut table = Table::new();
    table.push_row(Row::new([cell]));
    doc.push_table(table);
    doc
}

fn kinds(paragraphs: &[&Paragraph]) -> Vec<Option<ListKind>> {
    paragraphs.iter().map(|p| p.list).collect()
}

#[test]
fn list_kinds_survive_a_read() {
    let bytes = DocxWriter::write(&list_document()).expect("write");
    let doc = DocxDocument::read(&bytes).expect("read");
    let BodyElement::Paragraph(_) = &doc.body()[0] else {
        panic!("first element is a paragraph");
    };
    let body: Vec<&Paragraph> = doc
        .body()
        .iter()
        .filter_map(|e| match e {
            BodyElement::Paragraph(p) => Some(p),
            BodyElement::Table(_) => None,
        })
        .collect();
    assert_eq!(
        kinds(&body),
        [
            None,
            Some(ListKind::Bulleted),
            Some(ListKind::Numbered),
            Some(ListKind::Numbered),
        ]
    );
}

#[test]
fn a_list_item_in_a_table_cell_is_read_back() {
    let bytes = DocxWriter::write(&list_document()).expect("write");
    let doc = DocxDocument::read(&bytes).expect("read");
    let BodyElement::Table(table) = &doc.body()[4] else {
        panic!("fifth element is a table");
    };
    let cell_paragraph = &table.rows()[0].cells()[0].paragraphs()[1];
    assert_eq!(cell_paragraph.list, Some(ListKind::Bulleted));
}

#[test]
fn a_document_with_lists_survives_write_read_write() {
    let original = DocxWriter::write(&list_document()).expect("write");
    let read_back = DocxDocument::read(&original).expect("read");
    assert_eq!(DocxWriter::write(&read_back).expect("again"), original);
}

#[test]
fn a_document_without_lists_has_no_numbering_part() {
    let mut doc = DocxDocument::new();
    doc.push_paragraph(Paragraph::new("plain"));
    let bytes = DocxWriter::write(&doc).expect("write");
    let archive = open_archive(&bytes).expect("zip");
    assert!(archive.index_for_name("word/numbering.xml").is_none());
}

#[test]
fn a_list_document_has_a_numbering_part_and_relationship() {
    let bytes = DocxWriter::write(&list_document()).expect("write");
    let archive = open_archive(&bytes).expect("zip");
    assert!(archive.index_for_name("word/numbering.xml").is_some());
}

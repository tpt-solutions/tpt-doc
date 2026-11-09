//! The structural reader: a document written and read back writes the same
//! bytes, and the features the model carries survive the trip.

use tpt_doc_core::{DocError, DocReader};
use tpt_doc_word::{
    BorderStyle, Cell, DocxDocument, DocxWriter, Paragraph, Row, Run, RunStyle, Table,
};

fn sample() -> DocxDocument {
    let mut doc = DocxDocument::new();
    doc.push_paragraph(Paragraph::styled("Heading1", "Quarterly report"));
    let mut body = Paragraph::new("Plain ");
    body.push_run(Run::styled(
        "bold and italic",
        RunStyle {
            bold: true,
            italic: true,
            underline: false,
            font_size_half_points: Some(28),
        },
    ));
    body.push_run(Run::styled(
        " underlined",
        RunStyle {
            underline: true,
            ..RunStyle::default()
        },
    ));
    doc.push_paragraph(body);

    let mut table = Table::new();
    table.borders = BorderStyle::Single;
    table.column_widths_twips = vec![3000, 4000];
    table.push_row(Row::new([Cell::new("Item"), Cell::new("Amount")]));
    table.push_row(Row::new([Cell::new("Widget"), Cell::new("10.00")]));
    doc.push_table(table);

    doc.set_header("Confidential");
    doc.set_footer("Page footer");
    doc
}

#[test]
fn a_document_survives_a_write_read_write_trip() {
    let original = DocxWriter::write(&sample()).expect("write");
    let read_back = DocxDocument::read(&original).expect("read");
    assert_eq!(DocxWriter::write(&read_back).expect("write"), original);
}

#[test]
fn the_reader_recovers_header_and_footer() {
    let doc = DocxDocument::read(&DocxWriter::write(&sample()).expect("write")).expect("read");
    assert_eq!(doc.header(), Some("Confidential"));
    assert_eq!(doc.footer(), Some("Page footer"));
}

#[test]
fn the_reader_is_available_through_the_shared_trait() {
    let bytes = DocxWriter::write(&sample()).expect("write");
    let doc = <DocxDocument as DocReader>::from_bytes(&bytes).expect("trait read");
    assert_eq!(doc.body().len(), 3, "two paragraphs and a table");
}

#[test]
fn bytes_that_are_not_a_package_are_refused() {
    let err: DocError = DocxDocument::read(b"plain text").expect_err("not a zip");
    assert!(err.to_string().contains("ZIP"), "{err}");
}

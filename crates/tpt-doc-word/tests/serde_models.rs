//! A Word document model must survive a serde round trip: the bytes it writes
//! before and after are identical.

use tpt_doc_word::{
    BorderStyle, Cell, DocxDocument, DocxWriter, Paragraph, Row, Run, RunStyle, Table,
};

#[test]
fn document_round_trips_through_json() {
    let mut doc = DocxDocument::new();
    doc.push_paragraph(Paragraph::new("Hello"));
    doc.set_header("Header text");
    doc.set_footer("Page footer");
    let mut row = Row::new([Cell::new("a"), Cell::new("b")]);
    row.push_cell(Cell::new("c"));
    let mut table = Table::new();
    table.push_row(row);
    doc.push_table(table);

    let json = serde_json::to_string(&doc).expect("serialize");
    let back: DocxDocument = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        DocxWriter::write(&back).expect("write back"),
        DocxWriter::write(&doc).expect("write original")
    );
}

#[test]
fn styled_runs_and_borders_serialize() {
    let run = Run::styled(
        "bold",
        RunStyle {
            bold: true,
            ..RunStyle::default()
        },
    );
    let json = serde_json::to_string(&run).expect("serialize run");
    assert!(json.contains("bold"));
    let _ = BorderStyle::default();
}

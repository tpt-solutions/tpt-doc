//! Write a small workbook, then stream it back.
//!
//! Run with `cargo run -p tpt-doc-spreadsheet --example xlsx_roundtrip`.

use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxReader, XlsxWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = XlsxWriter::new();
    writer.push_row(Row {
        index: 1,
        cells: vec![Cell::String("item".into()), Cell::String("qty".into())],
    });
    writer.push_row(Row {
        index: 2,
        cells: vec![Cell::String("widget".into()), Cell::Number(12.0)],
    });
    let bytes = writer.finish()?;
    println!("wrote {} bytes", bytes.len());

    let mut reader = XlsxReader::new(&bytes)?;
    for row in reader.rows() {
        let row = row?;
        println!("row {}: {:?}", row.index, row.cells());
    }
    Ok(())
}

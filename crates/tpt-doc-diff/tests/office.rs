//! XLSX and DOCX diffs over documents built with the workspace's own writers.

use tpt_doc_diff::{ChangeKind, Options, diff_docx, diff_xlsx};
use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxWriter};
use tpt_doc_word::{DocxDocument, DocxWriter, Paragraph};

fn workbook(rows: &[&[Cell]]) -> Vec<u8> {
    let mut w = XlsxWriter::new();
    for (i, cells) in rows.iter().enumerate() {
        w.push_row(Row {
            index: u32::try_from(i + 1).expect("row"),
            cells: cells.to_vec(),
        });
    }
    w.finish().expect("xlsx")
}

#[test]
fn xlsx_changes_are_reported_by_cell_reference() {
    let s = |t: &str| Cell::String(t.to_owned());
    let a = workbook(&[&[s("item"), s("qty")], &[s("widget"), Cell::Number(12.0)]]);
    let b = workbook(&[
        &[s("item"), s("qty")],
        &[s("widget"), Cell::Number(15.0)],
        &[s("gadget"), Cell::Number(1.0)],
    ]);
    let d = diff_xlsx(&a, &b, &Options::default()).expect("diff");
    let found: Vec<_> = d
        .changes
        .iter()
        .map(|c| (c.path.as_str(), c.kind))
        .collect();
    assert!(found.contains(&("B2", ChangeKind::Changed)), "{found:?}");
    assert!(found.contains(&("A3", ChangeKind::Added)), "{found:?}");
    assert!(found.contains(&("B3", ChangeKind::Added)), "{found:?}");
    assert_eq!(d.changes.len(), 3);
    assert!(
        diff_xlsx(&a, &a, &Options::default())
            .expect("diff")
            .is_empty()
    );
}

#[test]
fn docx_changes_are_reported_by_paragraph() {
    let doc = |lines: &[&str]| {
        let mut d = DocxDocument::new();
        for line in lines {
            d.push_paragraph(Paragraph::new(*line));
        }
        DocxWriter::write(&d).expect("docx")
    };
    let a = doc(&["Title", "Dear Sir", "Regards"]);
    let b = doc(&["Title", "Dear Madam", "Regards", "PS"]);
    let d = diff_docx(&a, &b, &Options::default()).expect("diff");
    let found: Vec<_> = d
        .changes
        .iter()
        .map(|c| (c.path.as_str(), c.kind))
        .collect();
    assert_eq!(
        found,
        [("/p/1", ChangeKind::Changed), ("/p/3", ChangeKind::Added)]
    );
}

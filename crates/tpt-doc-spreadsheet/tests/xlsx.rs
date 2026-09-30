//! Integration tests for `.xlsx` reading and writing.
//!
//! `tests/fixtures/simple.xlsx` is built with Python's `zipfile` from
//! hand-written OOXML parts — deliberately independent of `XlsxWriter` — so
//! the fixture tests exercise real ZIP/XML interop.

use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxReader, XlsxWriter};

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/simple.xlsx"
    ))
    .expect("fixture file")
}

// The fixture deliberately contains a well-known decimal literal (3.14).
#[allow(clippy::approx_constant)]
#[test]
fn parses_independent_fixture() {
    let mut reader = XlsxReader::new(&fixture_bytes()).expect("valid xlsx");
    let rows: Vec<Row> = reader
        .rows()
        .collect::<Result<Vec<_>, _>>()
        .expect("parse fixture");

    assert_eq!(rows.len(), 4);

    assert_eq!(rows[0].index, 1);
    assert_eq!(
        rows[0].cells,
        vec![
            Cell::String("Alpha".to_owned()),
            Cell::Number(1.5),
            Cell::Boolean(true),
        ]
    );

    assert_eq!(
        rows[1].cells,
        vec![
            Cell::String("Beta <&> test".to_owned()),
            Cell::String("inline text".to_owned()),
            Cell::Error("#REF!".to_owned()),
        ]
    );

    assert_eq!(
        rows[2].cells,
        vec![Cell::String("Γεωγραφία".to_owned()), Cell::Number(-42.0),]
    );

    assert_eq!(rows[3].cells, vec![Cell::Blank, Cell::Number(3.14)]);
}

#[test]
fn writer_output_round_trips_through_reader() {
    let rows = vec![
        Row {
            index: 1,
            cells: vec![
                Cell::String("name".to_owned()),
                Cell::String("amount".to_owned()),
            ],
        },
        Row {
            index: 2,
            cells: vec![
                Cell::String("Widget & Co <Ltd>".to_owned()),
                Cell::Number(19.99),
                Cell::Boolean(false),
                Cell::Blank,
                Cell::Error("#N/A".to_owned()),
            ],
        },
        Row {
            index: 5,
            cells: vec![Cell::Number(0.1), Cell::String("padded".to_owned())],
        },
    ];

    let mut writer = XlsxWriter::new();
    for row in &rows {
        writer.push_row(row.clone());
    }
    let bytes = writer.finish().expect("write");

    let mut reader = XlsxReader::new(&bytes).expect("valid xlsx");
    let parsed: Vec<Row> = reader
        .rows()
        .collect::<Result<Vec<_>, _>>()
        .expect("parse own output");
    assert_eq!(parsed, rows);
}

#[test]
fn non_xlsx_input_is_rejected_not_panicked_on() {
    assert!(XlsxReader::new(b"not a zip file").is_err());
    assert!(XlsxReader::new(&[]).is_err());
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn xml_safe(s: &str) -> bool {
        s.chars()
            .all(|c| matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'))
    }

    fn arb_cell() -> impl Strategy<Value = Cell> {
        prop_oneof![
            3 => any::<String>()
                .prop_filter("xml-safe text", |s| xml_safe(s) && s.len() < 64)
                .prop_map(Cell::String),
            3 => any::<f64>().prop_filter("finite", |f: &f64| f.is_finite()).prop_map(Cell::Number),
            1 => any::<bool>().prop_map(Cell::Boolean),
            1 => Just(Cell::Blank),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn round_trip_arbitrary_rows(rows in proptest::collection::vec(
            (1u32..50, proptest::collection::vec(arb_cell(), 0..5)),
            0..8
        ).prop_map(|rows| {
            rows.into_iter().map(|(index, cells)| Row { index, cells }).collect::<Vec<_>>()
        })) {
            let mut writer = XlsxWriter::new();
            for row in &rows {
                writer.push_row(row.clone());
            }
            let bytes = writer.finish().expect("write");

            let mut reader = XlsxReader::new(&bytes).expect("valid xlsx");
            let parsed: Vec<Row> = reader.rows().collect::<Result<Vec<_>, _>>().expect("parse");
            prop_assert_eq!(parsed, rows);
        }
    }
}

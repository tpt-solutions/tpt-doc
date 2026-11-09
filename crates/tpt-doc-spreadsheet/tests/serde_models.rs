//! The spreadsheet row model must survive a serde round trip unchanged.

use tpt_doc_spreadsheet::xlsx::{Cell, Row};

#[test]
fn row_round_trips_through_json() {
    let row = Row {
        index: 3,
        cells: vec![
            Cell::String("name".into()),
            Cell::Number(42.5),
            Cell::Boolean(true),
            Cell::Error("#DIV/0!".into()),
            Cell::Blank,
        ],
    };
    let json = serde_json::to_string(&row).expect("serialize");
    let back: Row = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, row);
}

//! Integration tests for `.xlsx` reading and writing.
//!
//! `tests/fixtures/simple.xlsx` is built with Python's `zipfile` from
//! hand-written OOXML parts — deliberately independent of `XlsxWriter` — so
//! the fixture tests exercise real ZIP/XML interop.

use tpt_doc_core::Limits;
use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxReader, XlsxWriter};

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/simple.xlsx"
    ))
    .expect("fixture file")
}

/// A minimal single-row `.xlsx` with `rows` rows, built here rather than in a
/// fixture file so the row count can be tuned per test.
fn workbook_with_rows(rows: u32) -> Vec<u8> {
    let mut writer = XlsxWriter::new();
    for index in 1..=rows {
        writer.push_row(Row {
            index,
            cells: vec![Cell::Number(f64::from(index))],
        });
    }
    writer.finish().expect("write workbook")
}

/// A `.xlsx` whose worksheet part is `nested` elements deep inside `sheetData`.
fn workbook_with_nesting(nested: u32) -> Vec<u8> {
    let open = "<a>".repeat(nested as usize);
    let close = "</a>".repeat(nested as usize);
    let sheet = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{open}<row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c></row>{close}</sheetData></worksheet>"#
    );
    zipped_package(&sheet)
}

/// Wrap worksheet XML in the OOXML package parts a reader needs.
fn zipped_package(sheet: &str) -> Vec<u8> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, body) in [
        (
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet),
    ] {
        zip.start_file(name, options).expect("start entry");
        zip.write_all(body.as_bytes()).expect("write entry");
    }
    zip.finish().expect("finish zip").into_inner()
}

/// A sheet count at the limit parses; past it, the reader must report an error
/// rather than silently truncating the data.
#[test]
fn max_rows_is_enforced() {
    let bytes = workbook_with_rows(5);
    let limits = Limits {
        max_rows: 5,
        ..Limits::default()
    };
    let mut reader = XlsxReader::with_limits(&bytes, limits).expect("reader");
    let rows: Vec<Row> = reader
        .rows()
        .collect::<Result<Vec<_>, _>>()
        .expect("at the limit");
    assert_eq!(rows.len(), 5);

    let limits = Limits {
        max_rows: 3,
        ..Limits::default()
    };
    let mut reader = XlsxReader::with_limits(&bytes, limits).expect("reader");
    let mut rows = reader.rows();
    let mut seen = 0;
    let error = loop {
        match rows.next() {
            Some(Ok(_)) => seen += 1,
            Some(Err(e)) => break e,
            None => panic!("expected a row-limit error, got {seen} rows"),
        }
    };
    assert_eq!(seen, 3, "rows up to the limit are still yielded");
    assert!(error.to_string().contains("3 row limit"), "{error}");
}

/// Nesting deeper than `max_xml_depth` is refused rather than walked, which is
/// what stops a nesting bomb from exhausting the parser's own stack.
#[test]
fn max_xml_depth_is_enforced() {
    let bytes = workbook_with_nesting(6);
    let limits = Limits {
        max_xml_depth: 4,
        ..Limits::default()
    };
    let mut reader = XlsxReader::with_limits(&bytes, limits).expect("reader");
    let error = reader
        .rows()
        .collect::<Result<Vec<_>, _>>()
        .expect_err("nesting bomb must be rejected");
    assert!(error.to_string().contains("4 level limit"), "{error}");
}

/// The depth bound applies to the shared strings table too, since it is parsed
/// by the same reader before the worksheet.
#[test]
fn max_xml_depth_is_enforced_in_shared_strings() {
    let sheet = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>"#;
    let bytes = zipped_package_with_extra(sheet, "<sst><r><rPh>".to_owned());
    let limits = Limits {
        max_xml_depth: 2,
        ..Limits::default()
    };
    // `XlsxReader` holds the whole worksheet, so it is not `Debug` and
    // `expect_err` is unavailable; match instead.
    let error = match XlsxReader::with_limits(&bytes, limits) {
        Ok(_) => panic!("a shared strings table nested past the limit must be rejected"),
        Err(e) => e,
    };
    assert!(error.to_string().contains("level limit"), "{error}");
}

/// Build a package whose shared strings table contains `prefix` (unclosed).
fn zipped_package_with_extra(sheet: &str, prefix: String) -> Vec<u8> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, body) in [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet),
    ] {
        zip.start_file(name, options).expect("start entry");
        zip.write_all(body.as_bytes()).expect("write entry");
    }
    zip.start_file("xl/sharedStrings.xml", options)
        .expect("start sst");
    zip.write_all(prefix.as_bytes()).expect("write sst");
    zip.finish().expect("finish zip").into_inner()
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

/// Build a package from arbitrary `(name, body)` parts.
fn package_of(parts: &[(&str, &str)]) -> Vec<u8> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, body) in parts {
        zip.start_file(*name, options).expect("start entry");
        zip.write_all(body.as_bytes()).expect("write entry");
    }
    zip.finish().expect("finish zip").into_inner()
}

const ONE_CELL_SHEET: &str = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row></sheetData></worksheet>"#;

#[test]
fn first_sheet_is_resolved_through_workbook_rels() {
    // The first listed sheet lives in `sheet7.xml`, and a decoy sheet1 exists.
    let bytes = package_of(&[
        (
            "xl/workbook.xml",
            r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="A" sheetId="1" r:id="rId9"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId9" Target="worksheets/sheet7.xml"/></Relationships>"#,
        ),
        ("xl/sharedStrings.xml", "<sst><si><t>real</t></si></sst>"),
        ("xl/worksheets/sheet7.xml", ONE_CELL_SHEET),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>decoy</t></is></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let mut reader = XlsxReader::new(&bytes).expect("open");
    let rows: Vec<Row> = reader.rows().collect::<Result<_, _>>().expect("rows");
    assert_eq!(rows[0].cells, vec![Cell::String("real".into())]);
}

#[test]
fn phonetic_runs_are_not_part_of_the_cell_text() {
    let bytes = package_of(&[
        (
            "xl/sharedStrings.xml",
            "<sst><si><t>漢字</t><rPh sb=\"0\" eb=\"2\"><t>かんじ</t></rPh></si></sst>",
        ),
        ("xl/worksheets/sheet1.xml", ONE_CELL_SHEET),
    ]);
    let mut reader = XlsxReader::new(&bytes).expect("open");
    let rows: Vec<Row> = reader.rows().collect::<Result<_, _>>().expect("rows");
    assert_eq!(rows[0].cells, vec![Cell::String("漢字".into())]);
}

#[test]
fn out_of_range_shared_string_index_is_an_error() {
    let bytes = package_of(&[
        ("xl/sharedStrings.xml", "<sst></sst>"),
        ("xl/worksheets/sheet1.xml", ONE_CELL_SHEET),
    ]);
    let mut reader = XlsxReader::new(&bytes).expect("open");
    let result: Result<Vec<Row>, _> = reader.rows().collect();
    assert!(result.is_err());
}

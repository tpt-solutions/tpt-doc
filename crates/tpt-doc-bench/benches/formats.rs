//! Throughput of the parsers and writers on the shipped fixtures and on
//! generated data. Run with `cargo bench -p tpt-doc-bench`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use tpt_doc_fhir::Patient;
use tpt_doc_hl7v2::Hl7Parser;
use tpt_doc_pdf::{Document, Font, Page};
use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxReader, XlsxWriter};

const ADT: &[u8] = include_bytes!("../../tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7");
const EDIFACT: &[u8] = include_bytes!("../../tpt-doc-edi/tests/fixtures/sample.edifact");
const PATIENT: &[u8] = br#"{"resourceType":"Patient","id":"p1","name":[{"family":"Smith","given":["Ann"]}],"birthDate":"1970-01-01","gender":"female"}"#;

fn hl7_parse(c: &mut Criterion) {
    c.bench_function("hl7v2 parse ADT^A01", |b| {
        b.iter(|| Hl7Parser::new(black_box(ADT)).count());
    });
}

fn edifact_parse(c: &mut Criterion) {
    c.bench_function("edifact parse sample", |b| {
        b.iter(|| tpt_doc_edi::EdifactParser::new(black_box(EDIFACT)).count());
    });
}

fn xlsx_round_trip(c: &mut Criterion) {
    let rows: Vec<Row> = (1..=1000u32)
        .map(|index| Row {
            index,
            cells: (0..10)
                .map(|col| Cell::Number(f64::from(index * 10 + col)))
                .collect(),
        })
        .collect();
    c.bench_function("xlsx write+read 1000x10", |b| {
        b.iter(|| {
            let mut writer = XlsxWriter::new();
            for row in &rows {
                writer.push_row(row.clone());
            }
            let bytes = writer.finish().expect("write");
            XlsxReader::new(&bytes).expect("read").rows().count()
        });
    });
}

fn pdf_write(c: &mut Criterion) {
    c.bench_function("pdf write 50 lines", |b| {
        b.iter(|| {
            let mut doc = Document::new();
            let font = doc.embed_builtin_font(Font::Helvetica);
            let mut page = Page::a4();
            for line in 0..50u16 {
                page.text(
                    "The quick brown fox",
                    font,
                    10.0,
                    (72.0, 800.0 - f32::from(line) * 14.0),
                );
            }
            doc.add_page(page);
            doc.write().expect("write")
        });
    });
}

fn fhir_patient(c: &mut Criterion) {
    c.bench_function("fhir patient json round trip", |b| {
        b.iter(|| {
            let patient = Patient::from_json(black_box(PATIENT)).expect("parse");
            patient.to_json().expect("write")
        });
    });
}

criterion_group!(
    benches,
    hl7_parse,
    edifact_parse,
    xlsx_round_trip,
    pdf_write,
    fhir_patient
);
criterion_main!(benches);

//! The template gallery. Each `templates/NN-*` folder holds an input and the
//! output the library should produce from it. These tests regenerate every
//! output and compare it byte for byte with the expected file.
//!
//! After an intended change, run `TPT_UPDATE_GOLDEN=1 cargo test -p
//! tpt-doc-templates`, then review the diff of `templates/` before committing.

use std::fmt::Write as _;
use std::path::PathBuf;

use ring::rand::SystemRandom;
use tpt_doc_core::Limits;
use tpt_doc_edi::x12::X12Parser;
use tpt_doc_layout::Renderer;
use tpt_doc_pdf::{Document, Font, Page};
use tpt_doc_sign::{PrivateKey, pades, pem};
use tpt_doc_spreadsheet::csv::CsvReader;
use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxWriter};
use tpt_doc_ubl::Invoice;

/// Signing time inside the shipped test certificate's validity (2026-11-09 UTC).
const SIGNING_TIME: i64 = 1_794_182_400;

fn template_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../templates")
        .join(name)
}

fn read_input(folder: &str, file: &str) -> Vec<u8> {
    let path = template_dir(folder).join(file);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Compare `actual` with the expected file in the template folder, or rewrite
/// the file when `TPT_UPDATE_GOLDEN` is set.
fn check_golden(folder: &str, expected_file: &str, actual: &[u8]) {
    let path = template_dir(folder).join(expected_file);
    if std::env::var_os("TPT_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).expect("write golden output");
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "missing {} ({e}); run with TPT_UPDATE_GOLDEN=1 to create it",
            path.display()
        )
    });
    assert!(
        expected.as_slice() == actual,
        "{} differs from the regenerated output ({} expected bytes, {} produced)",
        path.display(),
        expected.len(),
        actual.len()
    );
}

// 01 — HL7 v2 ADT^A01 admission to a FHIR R5 bundle.
#[test]
fn t01_hl7_adt_to_fhir() {
    let input = read_input("01-hl7-adt-to-fhir", "input.hl7");
    let conversion = tpt_doc_convert::hl7v2_to_fhir(&input).expect("convert");
    let json = conversion.bundle.to_json().expect("bundle json");
    check_golden("01-hl7-adt-to-fhir", "expected.json", &json);
}

// 02 — X12 835 remittance advice to a plain-text payment summary.
fn summarize_835(bytes: &[u8]) -> String {
    let mut out = String::new();
    for segment in X12Parser::new(bytes).expect("x12 envelope") {
        let segment = segment.expect("segment");
        let e = segment.elements();
        let at = |i: usize| e.get(i).copied().unwrap_or("");
        match segment.tag() {
            "BPR" => writeln!(out, "total payment {}", at(1)).expect("write"),
            "CLP" => writeln!(
                out,
                "claim {} status {} billed {} paid {}",
                at(0),
                at(1),
                at(2),
                at(3)
            )
            .expect("write"),
            "SVC" => writeln!(out, "  service {}", at(0)).expect("write"),
            _ => {}
        }
    }
    out
}

#[test]
fn t02_x12_835_remittance_summary() {
    let input = read_input("02-x12-835-remittance", "input.x12");
    let summary = summarize_835(&input);
    assert!(!summary.is_empty(), "the remittance has payment lines");
    check_golden("02-x12-835-remittance", "expected.txt", summary.as_bytes());
}

// 03 — PEPPOL invoice, validated, then embedded in a PDF as Factur-X.
#[test]
fn t03_ubl_invoice_to_facturx_pdf() {
    let input = read_input("03-ubl-invoice-facturx", "input.xml");
    let invoice = Invoice::from_xml(&input).expect("parse invoice");
    let violations = tpt_doc_ubl::validate(&invoice);
    assert!(violations.is_empty(), "{violations:?}");

    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text(
        format!("Invoice {} from {}", invoice.id, invoice.supplier.name),
        font,
        14.0,
        (72.0, 760.0),
    );
    doc.add_page(page);
    tpt_doc_ubl::invoice::facturx::attach_xml(&invoice, &mut doc).expect("attach xml");
    let pdf = doc.write().expect("write pdf");
    check_golden("03-ubl-invoice-facturx", "expected.pdf", &pdf);
}

// 04 — CSV data to a spreadsheet report; numeric-looking fields become numbers.
#[test]
fn t04_csv_to_spreadsheet_report() {
    let input = read_input("04-spreadsheet-report", "input.csv");
    let mut writer = XlsxWriter::new();
    let mut reader = CsvReader::new(input.as_slice());
    // The CSV header is row 1 of the report; the reader yields it separately.
    let header = reader.headers().expect("csv header");
    writer.push_row(Row {
        index: 1,
        cells: header.into_iter().map(Cell::String).collect(),
    });
    for (index, record) in reader.records().enumerate() {
        let fields = record.expect("csv record");
        let cells = fields
            .into_iter()
            .map(|field| match field.parse::<f64>() {
                Ok(number) => Cell::Number(number),
                Err(_) => Cell::String(field),
            })
            .collect();
        writer.push_row(Row {
            index: u32::try_from(index + 2).expect("row number fits"),
            cells,
        });
    }
    let xlsx = writer.finish().expect("write xlsx");

    // The workbook must read back with the values we put in.
    let mut rows = tpt_doc_spreadsheet::xlsx::XlsxReader::with_limits(&xlsx, Limits::default())
        .expect("read back");
    let first = rows.rows().next().expect("header row").expect("row");
    assert_eq!(first.cells()[0], Cell::String("Item".into()));

    check_golden("04-spreadsheet-report", "expected.xlsx", &xlsx);
}

// 05 — HTML services agreement rendered to PDF, then signed with PAdES.
#[test]
fn t05_html_contract_rendered_and_signed() {
    let input = read_input("05-signed-contract", "input.html");
    let html = std::str::from_utf8(&input).expect("utf-8 html");
    let pdf = Renderer::new().render_html(html).expect("render");
    check_golden("05-signed-contract", "expected.pdf", &pdf);

    let key = PrivateKey::from_pem(include_bytes!(
        "../../tpt-doc-sign/tests/fixtures/test_key.pem"
    ))
    .expect("key");
    let cert_der = pem::pem_to_der(include_bytes!(
        "../../tpt-doc-sign/tests/fixtures/test_cert.pem"
    ))
    .expect("cert");
    let signed =
        pades::sign_at(&pdf, &key, &cert_der, &SystemRandom::new(), SIGNING_TIME).expect("sign");
    // A PAdES signature is an incremental update: the rendered bytes survive
    // unchanged as a prefix, and the signature sits after them.
    assert!(
        signed.starts_with(&pdf),
        "the signed PDF must keep the original bytes"
    );
    assert!(
        signed.windows(10).any(|w| w == b"/ByteRange"),
        "no signature dictionary"
    );
}

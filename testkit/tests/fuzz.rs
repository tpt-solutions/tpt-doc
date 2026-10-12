//! In-process fuzzing of the untrusted-input parsers with `tpt_testkit::fuzz::Engine`.
//!
//! Environment knobs (all optional): `TPT_FUZZ_ITERATIONS` (default 2000 per parser),
//! `TPT_FUZZ_SEED`, `TPT_FUZZ_CORPUS` (a directory; one subdirectory per parser),
//! `TPT_FUZZ_CRASHES`, `TPT_FUZZ_REPORT` (one `<stem>-<parser>.json` per parser).

use std::env;
use std::path::PathBuf;

use tpt_testkit::fuzz::{self, Engine};

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Runs one parser through the engine and fails if any input panics.
fn fuzz_parser(name: &str, seeds: Vec<Vec<u8>>, dictionary: &[&[u8]], target: fn(&[u8])) {
    let corpus_dir = env::var_os("TPT_FUZZ_CORPUS").map(|d| PathBuf::from(d).join(name));
    let mut seeds = seeds;
    if let Some(dir) = corpus_dir.as_ref().filter(|dir| dir.is_dir()) {
        seeds.extend(fuzz::read_corpus(dir).expect("corpus directory is readable"));
    }
    let mut engine = Engine::new(env_u64("TPT_FUZZ_SEED", 1), seeds).with_dictionary(dictionary);
    // The engine learns from the features a target reports. Input length (log2 bucket) and first
    // byte give it a cheap spread without instrumentation.
    engine.run(env_u64("TPT_FUZZ_ITERATIONS", 2000), |data| {
        target(data);
        vec![
            u64::from(usize::BITS - data.len().leading_zeros()),
            0x100 | u64::from(data.first().copied().unwrap_or(0)),
        ]
    });
    eprintln!(
        "{name}: {} executions, {} corpus entries, {} crashes",
        engine.executions(),
        engine.corpus().len(),
        engine.crashes().len()
    );
    if let Some(dir) = corpus_dir {
        fuzz::save_corpus(engine.corpus(), dir).expect("corpus is writable");
    }
    if let Some(dir) = env::var_os("TPT_FUZZ_CRASHES") {
        fuzz::save_failures(engine.crashes(), PathBuf::from(dir).join(name))
            .expect("crash directory is writable");
    }
    if let Some(path) = env::var_os("TPT_FUZZ_REPORT") {
        let path = PathBuf::from(path);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fuzz-report");
        fuzz::write_report(
            engine.crashes(),
            path.with_file_name(format!("{stem}-{name}.json")),
        )
        .expect("report path is writable");
    }
    let groups = fuzz::dedup(engine.crashes());
    assert!(
        groups.is_empty(),
        "{name}: {} crash group(s); first: {:?}",
        groups.len(),
        groups.first().map(|g| (&g.message, &g.smallest))
    );
}

#[test]
fn hl7v2() {
    fuzz_parser(
        "hl7v2",
        vec![
            include_bytes!("../../crates/tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7").to_vec(),
            include_bytes!("../../crates/tpt-doc-hl7v2/tests/fixtures/oru_r01.hl7").to_vec(),
        ],
        &[
            b"MSH|^~\\&|",
            b"\r",
            b"|",
            b"^",
            b"~",
            b"&",
            b"\\F\\",
            b"PID|",
            b"OBX|",
        ],
        |data| {
            for segment in tpt_doc_hl7v2::Hl7Parser::new(data) {
                if segment.is_err() {
                    break;
                }
            }
        },
    );
}

#[test]
fn edifact() {
    fuzz_parser(
        "edifact",
        vec![include_bytes!("../../crates/tpt-doc-edi/tests/fixtures/sample.edifact").to_vec()],
        &[
            b"UNA:+.? '",
            b"UNB+",
            b"UNH+",
            b"UNT+",
            b"UNZ+",
            b"?",
            b"'",
            b"+",
            b":",
        ],
        |data| {
            for segment in tpt_doc_edi::edifact::EdifactParser::new(data) {
                if segment.is_err() {
                    break;
                }
            }
        },
    );
}

#[test]
fn x12() {
    fuzz_parser(
        "x12",
        vec![include_bytes!("../../crates/tpt-doc-edi/tests/fixtures/sample_835.x12").to_vec()],
        &[
            b"ISA*", b"GS*", b"ST*", b"SE*", b"GE*", b"IEA*", b"~", b"*", b":",
        ],
        |data| {
            if let Ok(parser) = tpt_doc_edi::x12::X12Parser::new(data) {
                for segment in parser {
                    if segment.is_err() {
                        break;
                    }
                }
            }
        },
    );
}

#[test]
fn fhir_json() {
    fuzz_parser(
        "fhir_json",
        vec![
            include_bytes!("../../crates/tpt-doc-fhir/tests/fixtures/patient_bundle.json").to_vec(),
        ],
        fuzz::JSON_TOKENS,
        |data| {
            let _ = tpt_doc_fhir::prelude::Bundle::from_json(data);
        },
    );
}

#[test]
fn xlsx() {
    fuzz_parser(
        "xlsx",
        vec![
            include_bytes!("../../crates/tpt-doc-spreadsheet/tests/fixtures/simple.xlsx").to_vec(),
        ],
        &[
            b"PK\x03\x04",
            b"PK\x01\x02",
            b"PK\x05\x06",
            b"<row",
            b"<c ",
            b"</sheetData>",
        ],
        |data| {
            if let Ok(mut reader) = tpt_doc_spreadsheet::xlsx::XlsxReader::new(data) {
                for row in reader.rows() {
                    if row.is_err() {
                        break;
                    }
                }
            }
        },
    );
}

#[test]
fn docx() {
    fuzz_parser(
        "docx",
        vec![include_bytes!("../../crates/tpt-doc-word/tests/fixtures/simple.docx").to_vec()],
        &[
            b"PK\x03\x04",
            b"PK\x01\x02",
            b"PK\x05\x06",
            b"<w:p>",
            b"<w:t>",
            b"</w:body>",
        ],
        |data| {
            let _ = tpt_doc_word::prelude::DocxReader::extract_text(data);
        },
    );
}

#[test]
fn ubl_xml() {
    fuzz_parser(
        "ubl_xml",
        vec![include_bytes!("../../crates/tpt-doc-ubl/tests/fixtures/peppol_invoice.xml").to_vec()],
        &[
            b"<",
            b">",
            b"</",
            b"<![CDATA[",
            b"<!--",
            b"&amp;",
            b"cbc:",
            b"cac:",
        ],
        |data| {
            let _ = tpt_doc_ubl::prelude::Invoice::from_xml(data);
        },
    );
}

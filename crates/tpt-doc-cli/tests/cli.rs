//! End-to-end tests of the `tpt` binary against the other crates' fixtures.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn fixture(rel: &str) -> String {
    format!("{}/../{rel}", env!("CARGO_MANIFEST_DIR"))
}

fn tpt(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tpt"))
        .args(args)
        .output()
        .expect("run tpt")
}

fn tpt_stdin(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tpt"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tpt");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input)
        .expect("write stdin");
    child.wait_with_output().expect("wait")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn sniffs_each_fixture_by_content() {
    for (rel, expected) in [
        ("tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7", "hl7v2"),
        ("tpt-doc-edi/tests/fixtures/sample.edifact", "edifact"),
        ("tpt-doc-edi/tests/fixtures/sample_835.x12", "x12"),
        (
            "tpt-doc-fhir/tests/fixtures/patient_bundle.json",
            "fhir-json",
        ),
        ("tpt-doc-ubl/tests/fixtures/peppol_invoice.xml", "ubl"),
        ("tpt-doc-spreadsheet/tests/fixtures/simple.xlsx", "xlsx"),
    ] {
        let out = tpt(&["sniff", &fixture(rel)]);
        assert!(out.status.success(), "{rel}");
        assert_eq!(stdout(&out).trim(), expected, "{rel}");
    }
}

#[test]
fn valid_fixtures_exit_zero() {
    for rel in [
        "tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7",
        "tpt-doc-edi/tests/fixtures/sample_835.x12",
        "tpt-doc-fhir/tests/fixtures/patient_bundle.json",
        "tpt-doc-ubl/tests/fixtures/peppol_invoice.xml",
    ] {
        let out = tpt(&["validate", &fixture(rel)]);
        assert_eq!(out.status.code(), Some(0), "{rel}: {}", stdout(&out));
    }
}

#[test]
fn tampered_invoice_total_exits_one_with_rule_id() {
    let xml = std::fs::read_to_string(fixture("tpt-doc-ubl/tests/fixtures/peppol_invoice.xml"))
        .expect("fixture");
    let tampered = xml.replace(
        "<cbc:PayableAmount currencyID=\"NZD\">",
        "<cbc:PayableAmount currencyID=\"NZD\">9",
    );
    assert_ne!(tampered, xml, "fixture shape changed; update the tamper");
    let out = tpt_stdin(&["validate", "-", "--json"], tampered.as_bytes());
    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(json["valid"], false);
    assert!(
        json["issues"][0]["rule"]
            .as_str()
            .is_some_and(|r| !r.is_empty())
    );
}

#[test]
fn inspect_reports_hl7_structure() {
    let out = tpt(&[
        "inspect",
        &fixture("tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7"),
    ]);
    let text = stdout(&out);
    assert!(text.contains("message type: ADT^A01"), "{text}");
    assert!(text.contains("layout: MSH EVN PID PV1"), "{text}");
}

#[test]
fn render_writes_a_pdf() {
    let out = tpt_stdin(&["render", "-"], b"<h1>Hello</h1><p>World</p>");
    assert!(out.status.success());
    assert!(out.stdout.starts_with(b"%PDF-"));
}

#[test]
fn usage_errors_exit_two() {
    assert_eq!(tpt(&["bogus", "x"]).status.code(), Some(2));
    assert_eq!(tpt(&["validate"]).status.code(), Some(2));
    assert_eq!(tpt(&["sniff", "does-not-exist"]).status.code(), Some(2));
    assert_eq!(tpt_stdin(&["sniff", "-"], b"???").status.code(), Some(2));
}

#[test]
fn convert_turns_hl7_into_a_fhir_bundle() {
    let out = tpt(&[
        "convert",
        &fixture("tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7"),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(json["resourceType"], "Bundle");
    assert_eq!(json["entry"].as_array().map(Vec::len), Some(2));
    // Mapping losses go to stderr, not into the JSON.
    assert!(String::from_utf8_lossy(&out.stderr).contains("warning:"));

    let bad = tpt_stdin(&["convert", "-"], b"<h1>not hl7</h1>");
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn render_merges_json_data_into_the_template() {
    let dir = std::env::temp_dir().join(format!("tpt-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let data = dir.join("data.json");
    std::fs::write(&data, br#"{"who": "World"}"#).expect("write data");
    let data = data.to_string_lossy().into_owned();

    let ok = tpt_stdin(&["render", "-", "--data", &data], b"<p>Hello {{who}}</p>");
    assert!(ok.status.success());
    assert!(ok.stdout.starts_with(b"%PDF-"));

    // A field the data does not provide is an error, not a blank.
    let missing = tpt_stdin(&["render", "-", "--data", &data], b"<p>{{nope}}</p>");
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("missing field"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn redact_removes_patient_identifiers_and_audits_positions() {
    let out = tpt(&[
        "redact",
        &fixture("tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7"),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        !text.contains("Smith") && !text.contains("12345"),
        "{text:?}"
    );
    assert!(text.contains("PATIENT-1"), "{text:?}");
    let audit = String::from_utf8_lossy(&out.stderr);
    assert!(audit.contains("PID[2]-5"), "{audit}");
    // The audit names positions, never values.
    assert!(!audit.contains("Smith"));

    let bad = tpt_stdin(&["redact", "-"], b"<h1>nope</h1>");
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn redact_handles_fhir_bundles_and_keeps_them_valid() {
    let out = tpt(&[
        "redact",
        &fixture("tpt-doc-fhir/tests/fixtures/patient_bundle.json"),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        !text.contains("Smith") && !text.contains("ABC1234"),
        "{text}"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("Patient:Patient.name"));

    // The de-identified bundle is still a bundle the CLI accepts and validates.
    let again = tpt_stdin(&["validate", "-"], text.as_bytes());
    assert_eq!(again.status.code(), Some(0), "{}", stdout(&again));
}

#[test]
fn diff_reports_changed_paths_and_exit_codes() {
    let a = fixture("tpt-doc-hl7v2/tests/fixtures/adt_a01.hl7");
    let same = tpt(&["diff", &a, &a]);
    assert_eq!(same.status.code(), Some(0));
    assert!(same.stdout.is_empty());

    let original = std::fs::read_to_string(&a).expect("fixture");
    let changed = original.replace("Smith^John", "Smyth^John");
    let dir = std::env::temp_dir().join(format!("tpt-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let b = dir.join("b.hl7");
    std::fs::write(&b, changed).expect("write");
    let b = b.to_string_lossy().into_owned();

    let out = tpt(&["diff", &a, &b]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stdout(&out).contains("PID[0]-5.1: Smith \u{2192} Smyth"),
        "{}",
        stdout(&out)
    );

    let json = tpt(&["diff", &a, &b, "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).expect("json");
    assert_eq!(v["identical"], false);
    assert_eq!(v["changes"][0]["path"], "PID[0]-5.1");

    let ignored = tpt(&["diff", &a, &b, "--ignore", "PID"]);
    assert_eq!(ignored.status.code(), Some(0));

    // Different formats, or one argument, are usage errors.
    let fhir = fixture("tpt-doc-fhir/tests/fixtures/patient_bundle.json");
    assert_eq!(tpt(&["diff", &a, &fhir]).status.code(), Some(2));
    assert_eq!(tpt(&["diff", &a]).status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}

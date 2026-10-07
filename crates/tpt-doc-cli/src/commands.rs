//! Command implementations. Each returns text for stdout, or an error string.

use std::fmt::Write as _;

use tpt_doc::core::{Location, Severity, ValidationReport};

use crate::sniff::{Format, sniff};

/// Result of `validate`: the rendered output and whether the document is valid.
pub struct Outcome {
    /// Text for stdout.
    pub output: String,
    /// `false` when any error-severity issue was found.
    pub valid: bool,
}

fn detect(bytes: &[u8]) -> Result<Format, String> {
    sniff(bytes).ok_or_else(|| "unrecognised document format".to_owned())
}

/// `tpt sniff`: name the format.
pub fn sniff_cmd(bytes: &[u8]) -> Result<String, String> {
    Ok(detect(bytes)?.to_string())
}

/// `tpt validate`: parse and validate, returning a structured report.
pub fn validate(bytes: &[u8], json: bool) -> Result<Outcome, String> {
    let format = detect(bytes)?;
    let report = build_report(format, bytes)?;
    let valid = !report.has_errors();
    let output = if json {
        report_json(format, &report)
    } else {
        report_text(format, &report)
    };
    Ok(Outcome { output, valid })
}

fn build_report(format: Format, bytes: &[u8]) -> Result<ValidationReport, String> {
    match format {
        Format::Hl7v2 => {
            let segments: Vec<_> = tpt_doc::hl7v2::Hl7Parser::new(bytes)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let violations = tpt_doc::hl7v2::schema::validate_message(
                &tpt_doc::hl7v2::schema::v251(),
                &segments,
            );
            Ok(tpt_doc::hl7v2::Hl7Violation::to_report(&violations))
        }
        Format::Edifact => {
            let segments: Vec<_> = tpt_doc::edi::EdifactParser::new(bytes)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let violations = tpt_doc::edi::schema::validate_edifact(
                &tpt_doc::edi::schema::edifact_d96a(),
                &segments,
            );
            Ok(tpt_doc::edi::SchemaViolation::to_report(&violations))
        }
        Format::X12 => {
            let segments: Vec<_> = tpt_doc::edi::x12::X12Parser::new(bytes)
                .map_err(|e| e.to_string())?
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let tables = tpt_doc::edi::schema::x12_elements();
            let mut violations = tpt_doc::edi::validate_x12_envelope(&segments);
            for (i, s) in segments.iter().enumerate() {
                violations.extend(tables.check_x12(i, s));
            }
            Ok(tpt_doc::edi::SchemaViolation::to_report(&violations))
        }
        Format::Ubl => {
            let invoice = tpt_doc::ubl::Invoice::from_xml(bytes).map_err(|e| e.to_string())?;
            let errors = tpt_doc::ubl::validate::validate(&invoice);
            Ok(tpt_doc::ubl::validate::UblValidationError::to_report(
                &errors,
            ))
        }
        Format::FhirJson => fhir_report(bytes),
        Format::Pdf | Format::Html => Err(format!("{format} has no validator yet")),
        Format::Docx => {
            tpt_doc::word::document::DocxReader::extract_text(bytes).map_err(|e| e.to_string())?;
            Ok(ValidationReport::default())
        }
        Format::Xlsx => {
            let mut reader =
                tpt_doc::spreadsheet::xlsx::XlsxReader::new(bytes).map_err(|e| e.to_string())?;
            for row in reader.rows() {
                row.map_err(|e| e.to_string())?;
            }
            Ok(ValidationReport::default())
        }
    }
}

fn fhir_report(bytes: &[u8]) -> Result<ValidationReport, String> {
    use tpt_doc::fhir::{Bundle, Resource, validate_encounter, validate_observation};
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let kind = value
        .get("resourceType")
        .and_then(|v| v.as_str())
        .ok_or("JSON has no `resourceType`")?;
    let resources = if kind == "Bundle" {
        Bundle::from_json(bytes).map_err(|e| e.to_string())?.entries
    } else {
        let wrap = serde_json::json!({
            "resourceType": "Bundle",
            "type": "collection",
            "entry": [{ "resource": value }],
        });
        let wrapped = serde_json::to_vec(&wrap).map_err(|e| e.to_string())?;
        Bundle::from_json(&wrapped)
            .map_err(|e| e.to_string())?
            .entries
    };
    let mut report = ValidationReport::default();
    for resource in &resources {
        match resource {
            Resource::Observation(o) => report.merge(validate_observation(o)),
            Resource::Encounter(e) => report.merge(validate_encounter(e)),
            Resource::Patient(_) | Resource::Organization(_) => {}
        }
    }
    Ok(report)
}

fn severity_name(s: &Severity) -> &'static str {
    match s {
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn report_text(format: Format, report: &ValidationReport) -> String {
    let mut out = String::new();
    for issue in report.issues() {
        let loc = match &issue.location {
            Location::Unknown => String::new(),
            other => format!(" [{other}]"),
        };
        let rule = if issue.rule_id.is_empty() {
            String::new()
        } else {
            format!(" ({})", issue.rule_id)
        };
        let _ = writeln!(
            out,
            "{}: {}{}{}: {}",
            severity_name(&issue.severity),
            issue.field_path,
            loc,
            rule,
            issue.message
        );
    }
    let errors = report.errors().count();
    let warnings = report.warnings().count();
    let _ = writeln!(out, "{format}: {errors} error(s), {warnings} warning(s)");
    out
}

fn report_json(format: Format, report: &ValidationReport) -> String {
    let issues: Vec<_> = report
        .issues()
        .iter()
        .map(|i| {
            serde_json::json!({
                "severity": severity_name(&i.severity),
                "path": i.field_path,
                "location": i.location.to_string(),
                "rule": i.rule_id,
                "message": i.message,
            })
        })
        .collect();
    let doc = serde_json::json!({
        "format": format.to_string(),
        "valid": !report.has_errors(),
        "issues": issues,
    });
    let mut s = serde_json::to_string_pretty(&doc).unwrap_or_default();
    s.push('\n');
    s
}

/// `tpt inspect`: a short structural summary.
pub fn inspect(bytes: &[u8]) -> Result<String, String> {
    let format = detect(bytes)?;
    let mut out = format!("format: {format}\nsize: {} bytes\n", bytes.len());
    match format {
        Format::Hl7v2 => {
            let segs: Vec<_> = tpt_doc::hl7v2::Hl7Parser::new(bytes)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let _ = writeln!(out, "segments: {}", segs.len());
            if let Some(msh) = segs.first() {
                let _ = writeln!(out, "message type: {}", msh.field(9).unwrap_or("?"));
                let _ = writeln!(out, "control id: {}", msh.field(10).unwrap_or("?"));
            }
            let tags: Vec<_> = segs.iter().map(|s| s.tag()).collect();
            let _ = writeln!(out, "layout: {}", tags.join(" "));
        }
        Format::Edifact => {
            let segs: Vec<_> = tpt_doc::edi::EdifactParser::new(bytes)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let tags: Vec<_> = segs.iter().map(|s| s.tag()).collect();
            let _ = writeln!(out, "segments: {}\nlayout: {}", segs.len(), tags.join(" "));
        }
        Format::X12 => {
            let segs: Vec<_> = tpt_doc::edi::x12::X12Parser::new(bytes)
                .map_err(|e| e.to_string())?
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let tags: Vec<_> = segs.iter().map(|s| s.tag()).collect();
            let _ = writeln!(out, "segments: {}\nlayout: {}", segs.len(), tags.join(" "));
        }
        Format::Ubl => {
            let inv = tpt_doc::ubl::Invoice::from_xml(bytes).map_err(|e| e.to_string())?;
            let _ = writeln!(out, "lines: {}\ntotal: {}", inv.lines.len(), inv.total());
        }
        Format::FhirJson => {
            let value: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            let kind = value
                .get("resourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let _ = writeln!(out, "resourceType: {kind}");
            if let Some(entries) = value.get("entry").and_then(|v| v.as_array()) {
                let _ = writeln!(out, "entries: {}", entries.len());
            }
        }
        Format::Docx => {
            let text = tpt_doc::word::document::DocxReader::extract_text(bytes)
                .map_err(|e| e.to_string())?;
            let _ = writeln!(out, "characters: {}", text.chars().count());
        }
        Format::Xlsx => {
            let mut reader =
                tpt_doc::spreadsheet::xlsx::XlsxReader::new(bytes).map_err(|e| e.to_string())?;
            let mut rows = 0usize;
            for row in reader.rows() {
                row.map_err(|e| e.to_string())?;
                rows += 1;
            }
            let _ = writeln!(out, "rows (first sheet): {rows}");
        }
        Format::Pdf | Format::Html => {}
    }
    Ok(out)
}

/// `tpt render`: HTML to PDF bytes.
pub fn render(html: &[u8], data: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let html = std::str::from_utf8(html).map_err(|e| format!("input is not UTF-8: {e}"))?;
    let renderer = tpt_doc::layout::Renderer::new();
    match data {
        None => renderer.render_html(html),
        Some(data) => {
            let data: serde_json::Value =
                serde_json::from_slice(data).map_err(|e| format!("--data is not JSON: {e}"))?;
            renderer.render_template(html, &data)
        }
    }
    .map_err(|e| e.to_string())
}

/// `tpt text`: extract plain text from a `.docx`.
pub fn text(bytes: &[u8]) -> Result<String, String> {
    match detect(bytes)? {
        Format::Docx => {
            tpt_doc::word::document::DocxReader::extract_text(bytes).map_err(|e| e.to_string())
        }
        other => Err(format!("text extraction is not supported for {other}")),
    }
}

/// `tpt convert`: HL7 v2 to a FHIR R5 bundle (JSON). Returns the JSON and the
/// warnings to show on stderr.
pub fn convert(bytes: &[u8]) -> Result<(Vec<u8>, Vec<String>), String> {
    match detect(bytes)? {
        Format::Hl7v2 => {
            let conversion = tpt_doc_convert::hl7v2_to_fhir(bytes).map_err(|e| e.to_string())?;
            let json = conversion.bundle.to_json().map_err(|e| e.to_string())?;
            let mut notes = conversion.warnings;
            notes.extend(
                conversion
                    .report
                    .issues()
                    .iter()
                    .map(|i| format!("{}: {}", i.field_path, i.message)),
            );
            Ok((json, notes))
        }
        other => Err(format!(
            "no converter for {other}; only hl7v2 -> FHIR is available"
        )),
    }
}

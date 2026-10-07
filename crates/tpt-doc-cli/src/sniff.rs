//! Content-based format detection.

use std::fmt;
use std::io::Cursor;

/// A document format the CLI recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// HL7 v2.x pipe-delimited message.
    Hl7v2,
    /// ANSI X12 interchange.
    X12,
    /// UN/EDIFACT interchange.
    Edifact,
    /// FHIR resource or bundle in JSON.
    FhirJson,
    /// UBL Invoice or `CreditNote` XML.
    Ubl,
    /// PDF file.
    Pdf,
    /// Word `.docx` package.
    Docx,
    /// Excel `.xlsx` package.
    Xlsx,
    /// HTML text.
    Html,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hl7v2 => "hl7v2",
            Self::X12 => "x12",
            Self::Edifact => "edifact",
            Self::FhirJson => "fhir-json",
            Self::Ubl => "ubl",
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Xlsx => "xlsx",
            Self::Html => "html",
        })
    }
}

fn trim_start(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

/// Detect the format of `bytes` from their content (not the file name).
#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<Format> {
    let head = trim_start(bytes);
    if head.starts_with(b"%PDF-") {
        return Some(Format::Pdf);
    }
    if head.starts_with(b"PK\x03\x04") {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
        if archive.by_name("word/document.xml").is_ok() {
            return Some(Format::Docx);
        }
        if archive.by_name("xl/workbook.xml").is_ok() {
            return Some(Format::Xlsx);
        }
        return None;
    }
    if head.starts_with(b"MSH|") {
        return Some(Format::Hl7v2);
    }
    if head.starts_with(b"ISA") {
        return Some(Format::X12);
    }
    if head.starts_with(b"UNA") || head.starts_with(b"UNB") {
        return Some(Format::Edifact);
    }
    if head.first() == Some(&b'{') {
        return Some(Format::FhirJson);
    }
    if head.first() == Some(&b'<') {
        let text = String::from_utf8_lossy(&head[..head.len().min(4096)]);
        if text.contains("<Invoice") || text.contains("<CreditNote") {
            return Some(Format::Ubl);
        }
        let lower = text.to_ascii_lowercase();
        if lower.contains("<html")
            || lower.contains("<!doctype html")
            || lower.contains("<p")
            || lower.contains("<h1")
        {
            return Some(Format::Html);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_text_formats() {
        assert_eq!(sniff(b"MSH|^~|A"), Some(Format::Hl7v2));
        assert_eq!(sniff(b"\n  UNB+UNOA:1'"), Some(Format::Edifact));
        assert_eq!(sniff(b"ISA*00*"), Some(Format::X12));
        assert_eq!(
            sniff(br#" {"resourceType":"Patient"}"#),
            Some(Format::FhirJson)
        );
        assert_eq!(sniff(b"%PDF-1.7"), Some(Format::Pdf));
        assert_eq!(sniff(b"<h1>x</h1>"), Some(Format::Html));
        assert_eq!(sniff(b"<html><body></body></html>"), Some(Format::Html));
        assert_eq!(
            sniff(b"<?xml version=\"1.0\"?><Invoice xmlns=\"x\"/>"),
            Some(Format::Ubl)
        );
        assert_eq!(sniff(b""), None);
    }

    #[test]
    fn detects_zip_packages_by_content() {
        use tpt_doc::spreadsheet::xlsx::{Row, XlsxWriter};
        let mut writer = XlsxWriter::new();
        writer.push_row(Row {
            index: 1,
            cells: vec![],
        });
        assert_eq!(sniff(&writer.finish().unwrap()), Some(Format::Xlsx));
        assert_eq!(sniff(b"PK\x03\x04garbage"), None);
    }
}

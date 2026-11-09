//! Exporting the audit log: the PDF carries the verified JSON Lines, and each
//! entry becomes a FHIR Provenance that points back at its hash.

use tpt_doc_audit::AuditLog;
use tpt_doc_pdf::{Document, Font, Page};

fn log() -> AuditLog {
    let mut log = AuditLog::new();
    log.append("2026-11-09T10:00:00Z", "invoice.created", b"invoice 1")
        .expect("append");
    log.append("2026-11-09T10:05:00Z", "invoice.approved", b"approved by A")
        .expect("append");
    log
}

fn document() -> Document {
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text("Audited invoice", font, 12.0, (72.0, 700.0));
    doc.add_page(page);
    doc
}

#[test]
fn the_pdf_carries_the_log_as_an_attachment() {
    let log = log();
    let mut doc = document();
    log.attach_to_pdf(&mut doc).expect("attach");
    let pdf = doc.write().expect("write");
    let needle = b"audit-log.jsonl";
    assert!(
        pdf.windows(needle.len()).any(|w| w == needle),
        "attachment name missing"
    );
    // The attached event text is in the file (attachments are stored uncompressed).
    assert!(
        pdf.windows(b"invoice.approved".len())
            .any(|w| w == b"invoice.approved")
    );
}

#[test]
fn provenance_names_the_target_time_event_and_hash() {
    let log = log();
    let provenance = log.provenance(1, "Invoice/inv-5001").expect("provenance");
    assert_eq!(provenance["resourceType"], "Provenance");
    assert_eq!(provenance["target"][0]["reference"], "Invoice/inv-5001");
    assert_eq!(provenance["recorded"], "2026-11-09T10:05:00Z");
    assert_eq!(provenance["activity"]["text"], "invoice.approved");
    assert_eq!(
        provenance["extension"][0]["valueString"],
        log.entries()[1].hash.as_str()
    );
}

#[test]
fn a_timestamp_that_is_not_an_instant_is_refused() {
    let mut log = AuditLog::new();
    log.append("yesterday", "invoice.created", b"x")
        .expect("append");
    let err = log.provenance(0, "Invoice/1").expect_err("not an instant");
    assert!(err.to_string().contains("RFC 3339"), "{err}");
}

#[test]
fn an_out_of_range_entry_is_refused() {
    assert!(log().provenance(9, "Invoice/1").is_err());
}

//! The audit chain: appending, verifying, persisting, tampering, and sealing.

use ring::rand::SystemRandom;
use tpt_doc_audit::{AuditLog, GENESIS};
use tpt_doc_sign::PrivateKey;

/// Signing time inside the fixture certificate's validity (2026-11-09 UTC).
const SIGNING_TIME: i64 = 1_794_182_400;

fn sample_log() -> AuditLog {
    let mut log = AuditLog::new();
    log.append("2026-11-09T10:00:00Z", "invoice.created", b"invoice 1")
        .expect("append");
    log.append("2026-11-09T10:05:00Z", "invoice.approved", b"approved by A")
        .expect("append");
    log.append("2026-11-09T10:09:00Z", "invoice.sent", b"peppol 42")
        .expect("append");
    log
}

#[test]
fn an_empty_log_verifies_and_its_head_is_genesis() {
    let log = AuditLog::new();
    assert!(log.is_empty());
    assert_eq!(log.head(), GENESIS);
    log.verify().expect("empty log verifies");
}

#[test]
fn entries_link_to_their_predecessors() {
    let log = sample_log();
    log.verify().expect("verifies");
    let entries = log.entries();
    assert_eq!(entries[0].prev, GENESIS);
    assert_eq!(entries[1].prev, entries[0].hash);
    assert_eq!(entries[2].prev, entries[1].hash);
    assert_eq!(log.head(), entries[2].hash);
}

#[test]
fn the_same_events_give_the_same_chain() {
    assert_eq!(sample_log(), sample_log());
}

#[test]
fn an_empty_event_name_is_refused() {
    let mut log = AuditLog::new();
    assert!(log.append("2026-11-09T10:00:00Z", "", b"x").is_err());
    assert!(log.is_empty());
}

#[test]
fn the_payload_is_hashed_not_stored() {
    let log = sample_log();
    let json = log.to_jsonl().expect("jsonl");
    assert!(!json.contains("invoice 1"), "payload leaked into the log");
}

#[test]
fn jsonl_round_trips() {
    let log = sample_log();
    let text = log.to_jsonl().expect("jsonl");
    assert_eq!(AuditLog::from_jsonl(&text).expect("parse"), log);
}

#[test]
fn editing_an_event_is_detected_at_its_entry() {
    let text = sample_log()
        .to_jsonl()
        .expect("jsonl")
        .replace("invoice.approved", "invoice.rejected");
    let err = AuditLog::from_jsonl(&text).expect_err("edited log must fail");
    assert!(err.to_string().contains("does not match"), "{err}");
    assert_eq!(err.location(), Some(&tpt_doc_core::Location::Index(1)));
}

#[test]
fn editing_a_payload_digest_is_detected() {
    let log = sample_log();
    let digest = log.entries()[0].payload_sha256.clone();
    let text = log
        .to_jsonl()
        .expect("jsonl")
        .replacen(&digest, &"0".repeat(64), 1);
    assert!(AuditLog::from_jsonl(&text).is_err());
}

#[test]
fn deleting_an_entry_is_detected() {
    let text = sample_log().to_jsonl().expect("jsonl");
    let without_middle: String = text
        .lines()
        .enumerate()
        .filter(|(i, _)| *i != 1)
        .map(|(_, line)| format!("{line}\n"))
        .collect();
    assert!(AuditLog::from_jsonl(&without_middle).is_err());
}

#[test]
fn reordering_entries_is_detected() {
    let text = sample_log().to_jsonl().expect("jsonl");
    let mut lines: Vec<&str> = text.lines().collect();
    lines.swap(1, 2);
    let reordered = lines.join("\n");
    assert!(AuditLog::from_jsonl(&reordered).is_err());
}

#[test]
fn a_line_that_is_not_an_entry_is_refused_with_its_number() {
    let text = format!("{}not json\n", sample_log().to_jsonl().expect("jsonl"));
    let err = AuditLog::from_jsonl(&text).expect_err("garbage line");
    assert!(err.to_string().contains("line 4"), "{err}");
}

#[test]
fn the_head_seal_verifies_with_openssl() {
    let key = PrivateKey::from_pem(include_bytes!(
        "../../tpt-doc-sign/tests/fixtures/test_key.pem"
    ))
    .expect("key");
    let cert_der = tpt_doc_sign::pem::pem_to_der(include_bytes!(
        "../../tpt-doc-sign/tests/fixtures/test_cert.pem"
    ))
    .expect("cert");
    let log = sample_log();
    let rng = SystemRandom::new();
    let seal = log.seal(&key, &cert_der, &rng, SIGNING_TIME).expect("seal");

    let dir = std::env::temp_dir();
    let head_path = dir.join("tpt-doc-audit-head.txt");
    let seal_path = dir.join("tpt-doc-audit-seal.der");
    std::fs::write(&head_path, log.head()).expect("write head");
    std::fs::write(&seal_path, &seal).expect("write seal");
    let output = std::process::Command::new("openssl")
        .args(["cms", "-verify", "-binary", "-inform", "DER", "-in"])
        .arg(&seal_path)
        .arg("-content")
        .arg(&head_path)
        .arg("-noverify")
        .arg("-out")
        .arg(dir.join("tpt-doc-audit-out"))
        .output();
    let _ = std::fs::remove_file(&head_path);
    let _ = std::fs::remove_file(&seal_path);
    match output {
        Ok(output) => assert!(
            output.status.success(),
            "openssl rejected the seal: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(_) => println!("skipping: openssl not available"),
    }
}

#[test]
fn a_seal_does_not_verify_against_a_different_head() {
    let key = PrivateKey::from_pem(include_bytes!(
        "../../tpt-doc-sign/tests/fixtures/test_key.pem"
    ))
    .expect("key");
    let cert_der = tpt_doc_sign::pem::pem_to_der(include_bytes!(
        "../../tpt-doc-sign/tests/fixtures/test_cert.pem"
    ))
    .expect("cert");
    let rng = SystemRandom::new();
    let sealed = sample_log();
    let mut extended = sample_log();
    extended
        .append("2026-11-09T10:10:00Z", "invoice.paid", b"x")
        .expect("append");
    let seal = sealed
        .seal(&key, &cert_der, &rng, SIGNING_TIME)
        .expect("seal");

    let dir = std::env::temp_dir();
    let seal_path = dir.join("tpt-doc-audit-mismatch-seal.der");
    let head_path = dir.join("tpt-doc-audit-mismatch-head.txt");
    std::fs::write(&seal_path, &seal).expect("write seal");
    std::fs::write(&head_path, extended.head()).expect("write head");
    let output = std::process::Command::new("openssl")
        .args(["cms", "-verify", "-binary", "-inform", "DER", "-in"])
        .arg(&seal_path)
        .arg("-content")
        .arg(&head_path)
        .arg("-noverify")
        .arg("-out")
        .arg(dir.join("tpt-doc-audit-mismatch-out"))
        .output();
    let _ = std::fs::remove_file(&seal_path);
    let _ = std::fs::remove_file(&head_path);
    match output {
        Ok(output) => assert!(
            !output.status.success(),
            "a seal over one head must not verify against another"
        ),
        Err(_) => println!("skipping: openssl not available"),
    }
}

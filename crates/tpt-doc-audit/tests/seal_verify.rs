//! Verifying a seal in Rust: it checks against the head it was made over, and
//! stops checking once the log has grown.

use ring::rand::SystemRandom;
use tpt_doc_audit::AuditLog;
use tpt_doc_sign::PrivateKey;

const KEY: &[u8] = include_bytes!("../../tpt-doc-sign/tests/fixtures/test_key.pem");
const CERT: &[u8] = include_bytes!("../../tpt-doc-sign/tests/fixtures/test_cert.pem");
const SIGNING_TIME: i64 = 1_794_182_400;

fn seal(log: &AuditLog) -> Vec<u8> {
    let key = PrivateKey::from_pem(KEY).expect("key");
    let cert = tpt_doc_sign::pem::pem_to_der(CERT).expect("cert");
    log.seal(&key, &cert, &SystemRandom::new(), SIGNING_TIME)
        .expect("seal")
}

fn log() -> AuditLog {
    let mut log = AuditLog::new();
    log.append("2026-11-09T10:00:00Z", "invoice.created", b"invoice 1")
        .expect("append");
    log
}

#[test]
fn a_seal_verifies_against_its_own_head() {
    let log = log();
    let verified = log
        .verify_seal(&seal(&log), Some(SIGNING_TIME))
        .expect("verify");
    assert!(!verified.signer_certificate_der.is_empty());
}

#[test]
fn a_seal_stops_verifying_once_the_log_grows() {
    let mut log = log();
    let old = seal(&log);
    log.append("2026-11-09T10:05:00Z", "invoice.approved", b"approved")
        .expect("append");
    assert!(log.verify_seal(&old, None).is_err(), "the head moved on");
    assert!(log.verify_seal(&seal(&log), None).is_ok());
}

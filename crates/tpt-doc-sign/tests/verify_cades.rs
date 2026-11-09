//! The Rust CAdES verifier: it accepts what the signer makes, and refuses
//! tampered content, a corrupted signature, and a certificate outside its
//! validity. It also accepts a signature made by OpenSSL, which is a
//! different implementation.

use ring::rand::SystemRandom;
use tpt_doc_sign::verify::verify_detached;
use tpt_doc_sign::{PrivateKey, cades, pem};

const KEY_PEM: &[u8] = include_bytes!("fixtures/test_key.pem");
const CERT_PEM: &[u8] = include_bytes!("fixtures/test_cert.pem");
/// Signing time inside the fixture certificate's validity (2026-11-09 UTC).
const SIGNING_TIME: i64 = 1_794_182_400;
const DATA: &[u8] = b"The quick brown fox jumps over the lazy dog";

fn key() -> PrivateKey {
    PrivateKey::from_pem(KEY_PEM).expect("key")
}

fn cert_der() -> Vec<u8> {
    pem::pem_to_der(CERT_PEM).expect("cert")
}

fn signed() -> Vec<u8> {
    cades::sign_detached_at(
        DATA,
        &key(),
        &cert_der(),
        &SystemRandom::new(),
        SIGNING_TIME,
    )
    .expect("sign")
}

#[test]
fn a_signature_made_by_the_signer_verifies() {
    let verified = verify_detached(DATA, &signed(), Some(SIGNING_TIME)).expect("verify");
    assert_eq!(verified.signer_certificate_der, cert_der());
}

#[test]
fn changed_content_is_refused_by_its_digest() {
    let err = verify_detached(b"a different document", &signed(), None).expect_err("tampered");
    assert!(err.to_string().contains("messageDigest"), "{err}");
}

#[test]
fn a_corrupted_signature_does_not_verify() {
    let mut cms = signed();
    let last = cms.len() - 10; // inside the trailing signature value
    cms[last] ^= 0x01;
    assert!(verify_detached(DATA, &cms, None).is_err());
}

#[test]
fn a_time_outside_the_certificate_validity_is_refused() {
    // 2040 is after the fixture certificate's notAfter (2036).
    assert!(verify_detached(DATA, &signed(), Some(2_208_988_800)).is_err());
}

#[test]
fn a_signature_without_its_certificate_is_refused() {
    let cms = signed();
    // The certificate is embedded in the SignedData; a CMS without it cannot be
    // checked, and the verifier says so.
    let cert = cert_der();
    let position = cms
        .windows(cert.len())
        .position(|w| w == cert.as_slice())
        .expect("certificate embedded");
    let mut stripped = cms.clone();
    for byte in &mut stripped[position..position + cert.len()] {
        *byte = 0;
    }
    assert!(verify_detached(DATA, &stripped, None).is_err());
}

#[test]
fn a_signature_made_by_openssl_verifies() {
    let dir = std::env::temp_dir();
    let data_path = dir.join("tpt-doc-verify-data.bin");
    let key_path = dir.join("tpt-doc-verify-key.pem");
    let cert_path = dir.join("tpt-doc-verify-cert.pem");
    let sig_path = dir.join("tpt-doc-verify-openssl.der");
    std::fs::write(&data_path, DATA).expect("write data");
    std::fs::write(&key_path, KEY_PEM).expect("write key");
    std::fs::write(&cert_path, CERT_PEM).expect("write cert");
    let output = std::process::Command::new("openssl")
        .args(["cms", "-sign", "-binary", "-in"])
        .arg(&data_path)
        .arg("-signer")
        .arg(&cert_path)
        .arg("-inkey")
        .arg(&key_path)
        .args(["-outform", "DER", "-out"])
        .arg(&sig_path)
        .output();
    let result = match output {
        Ok(output) if output.status.success() => {
            let cms = std::fs::read(&sig_path).expect("read openssl signature");
            Some(verify_detached(DATA, &cms, None))
        }
        Ok(output) => panic!(
            "openssl could not sign: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(_) => None,
    };
    for path in [&data_path, &key_path, &cert_path, &sig_path] {
        let _ = std::fs::remove_file(path);
    }
    match result {
        Some(verified) => {
            verified.expect("the Rust verifier accepts an OpenSSL CAdES signature");
        }
        None => println!("skipping: openssl not available"),
    }
}

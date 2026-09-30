//! Integration tests: verify generated signatures with the `openssl` CLI.
//!
//! These tests are the interop gate: our hand-built DER/CMS structures are
//! only correct if an independent implementation accepts them. They skip
//! silently when `openssl` is not on PATH (e.g. minimal CI images).

use ring::rand::SystemRandom;
use tpt_doc_sign::{PrivateKey, pades};

const TEST_KEY_PEM: &str = include_str!("fixtures/test_key.pem");
const TEST_CERT_PEM: &str = include_str!("fixtures/test_cert.pem");

/// Write a temp file with a unique-ish name and return its path.
fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("tpt-doc-sign-test-{}-{name}", std::process::id()));
    std::fs::write(&path, bytes).expect("write temp file");
    path
}

fn openssl_available() -> bool {
    std::process::Command::new("openssl")
        .arg("version")
        .output()
        .is_ok_and(|out| out.status.success())
}

fn sample_pdf() -> Vec<u8> {
    let mut doc = tpt_doc_pdf::Document::new();
    let font = doc.embed_builtin_font(tpt_doc_pdf::Font::Helvetica);
    let mut page = tpt_doc_pdf::Page::a4();
    page.text("Signed with tpt-doc-sign", font, 12.0, (72.0, 700.0));
    doc.add_page(page);
    doc.write().expect("pdf")
}

#[test]
fn cades_signature_verifies_with_openssl() {
    if !openssl_available() {
        println!("skipping: openssl not available");
        return;
    }
    let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
    let cert_der = tpt_doc_sign::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert der");
    let data = b"The quick brown fox jumps over the lazy dog";
    let rng = SystemRandom::new();
    let cms = tpt_doc_sign::cades::sign_detached(data, &key, &cert_der, &rng).expect("sign");

    let data_path = temp_file("cades-data.bin", data);
    let cms_path = temp_file("cades-sig.der", &cms);
    let output = std::process::Command::new("openssl")
        .args(["cms", "-verify", "-binary", "-inform", "DER", "-in"])
        .arg(&cms_path)
        .arg("-content")
        .arg(&data_path)
        .arg("-noverify")
        .arg("-out")
        .arg(std::env::temp_dir().join("tpt-doc-sign-verify-out"))
        .output()
        .expect("run openssl");
    let _ = std::fs::remove_file(data_path);
    let _ = std::fs::remove_file(cms_path);
    assert!(
        output.status.success(),
        "openssl cms verify failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pades_signature_preserves_pdf_and_verifies() {
    if !openssl_available() {
        println!("skipping: openssl not available");
        return;
    }
    let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
    let cert_der = tpt_doc_sign::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert der");
    let rng = SystemRandom::new();
    let original = sample_pdf();
    let signed = pades::sign(&original, &key, &cert_der, &rng).expect("sign");

    // The original bytes are untouched (incremental update).
    assert!(
        signed.starts_with(&original),
        "original PDF must be a prefix"
    );
    assert!(signed.ends_with(b"%%EOF\n"));

    // Exactly one /ByteRange covering the whole file except /Contents.
    // Search raw bytes — compressed streams are not valid UTF-8.
    let marker = b"/ByteRange [";
    let byte_range_pos = signed
        .windows(marker.len())
        .rposition(|w| w == marker)
        .expect("ByteRange present");
    let range = &signed[byte_range_pos..byte_range_pos + 64];
    let close = range
        .iter()
        .position(|b| *b == b']')
        .expect("close bracket");
    let parts_str = std::str::from_utf8(&range[marker.len()..close]).expect("ASCII range");
    let parts: Vec<u64> = parts_str
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    assert_eq!(
        parts.len(),
        4,
        "/ByteRange must have 4 values: {parts_str:?}"
    );
    assert_eq!(parts[0], 0, "first range starts at 0");
    // Reconstruct the signed content from the ranges and verify with openssl.
    let (x1, y1, z1) = (parts[1] as usize, parts[2] as usize, parts[3] as usize);
    let mut signed_content = Vec::with_capacity(x1 + z1);
    signed_content.extend_from_slice(&signed[..x1]);
    signed_content.extend_from_slice(&signed[y1..y1 + z1]);

    // The hex payload lives between the two ranges (x1..y1).
    let cms_hex: String = String::from_utf8_lossy(&signed[x1..y1])
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    let cms: Vec<u8> = (0..cms_hex.len() / 2)
        .filter_map(|i| u8::from_str_radix(&cms_hex[2 * i..2 * i + 2], 16).ok())
        .collect();
    assert!(!cms.is_empty(), "CMS extracted from /Contents");

    let content_path = temp_file("pades-content.bin", &signed_content);
    let cms_path = temp_file("pades-cms.der", &cms);
    let output = std::process::Command::new("openssl")
        .args(["cms", "-verify", "-binary", "-inform", "DER", "-in"])
        .arg(&cms_path)
        .arg("-content")
        .arg(&content_path)
        .arg("-noverify")
        .output()
        .expect("run openssl");
    let _ = std::fs::remove_file(content_path);
    let _ = std::fs::remove_file(cms_path);
    assert!(
        output.status.success(),
        "openssl cms verify of PAdES ranges failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn xades_signature_value_verifies_with_openssl() {
    if !openssl_available() {
        println!("skipping: openssl not available");
        return;
    }
    let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
    let cert_der = tpt_doc_sign::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert der");
    let rng = SystemRandom::new();
    let xml = br#"<Invoice xmlns="urn:example"><id>INV-1</id></Invoice>"#;
    let signed = tpt_doc_sign::xades::sign(xml, &key, &cert_der, &rng).expect("sign");
    let text = String::from_utf8(signed).expect("UTF-8");

    // Extract SignedInfo (input to the signature) and SignatureValue.
    let si_start = text.find("<ds:SignedInfo").expect("SignedInfo");
    let si_end =
        text.find("</ds:SignedInfo>").expect("SignedInfo close") + "</ds:SignedInfo>".len();
    let signed_info = &text[si_start..si_end];
    let sv_start =
        text.find("<ds:SignatureValue>").expect("SignatureValue") + "<ds:SignatureValue>".len();
    let sv_end = text
        .find("</ds:SignatureValue>")
        .expect("SignatureValue close");
    use base64::Engine as _;
    let signature = base64::engine::general_purpose::STANDARD
        .decode(&text[sv_start..sv_end])
        .expect("base64 signature");

    let pub_pem = base64_pubkey_pem();
    let si_path = temp_file("xades-signedinfo.xml", signed_info.as_bytes());
    let sig_path = temp_file("xades-sig.bin", &signature);
    let pub_path = temp_file("xades-pub.pem", pub_pem.as_bytes());
    let output = std::process::Command::new("openssl")
        .args(["dgst", "-sha256", "-verify"])
        .arg(&pub_path)
        .arg("-signature")
        .arg(&sig_path)
        .arg(&si_path)
        .output()
        .expect("run openssl");
    let _ = std::fs::remove_file(si_path);
    let _ = std::fs::remove_file(sig_path);
    let _ = std::fs::remove_file(pub_path);
    assert!(
        output.status.success(),
        "openssl dgst verify of XAdES SignedInfo failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Extract the RSA public key PEM from the test certificate via openssl.
fn base64_pubkey_pem() -> String {
    let cert_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_cert.pem");
    let output = std::process::Command::new("openssl")
        .args(["x509", "-in"])
        .arg(&cert_path)
        .args(["-pubkey", "-noout"])
        .output()
        .expect("extract pubkey");
    String::from_utf8(output.stdout).expect("PEM")
}

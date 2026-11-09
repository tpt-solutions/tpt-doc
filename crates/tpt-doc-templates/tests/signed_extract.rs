//! A signed PDF must still yield its text. PAdES appends the signature as an
//! incremental update, so the reader has to follow the trailer chain (`/Prev`)
//! back to the original content.

use ring::rand::SystemRandom;
use tpt_doc_pdf::extract::extract_text;
use tpt_doc_sign::{PrivateKey, pades, pem};

const KEY: &[u8] = include_bytes!("../../tpt-doc-sign/tests/fixtures/test_key.pem");
const CERT: &[u8] = include_bytes!("../../tpt-doc-sign/tests/fixtures/test_cert.pem");
/// Signing time inside the shipped test certificate's validity (2026-11-09 UTC).
const SIGNING_TIME: i64 = 1_794_182_400;

#[test]
fn the_text_of_a_signed_contract_is_still_readable() {
    let unsigned = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../templates/05-signed-contract/expected.pdf"
    ))
    .expect("template pdf");
    let key = PrivateKey::from_pem(KEY).expect("key");
    let cert = pem::pem_to_der(CERT).expect("cert");
    let signed =
        pades::sign_at(&unsigned, &key, &cert, &SystemRandom::new(), SIGNING_TIME).expect("sign");
    assert!(signed.len() > unsigned.len(), "the signature was appended");

    let text = extract_text(&signed).expect("extract signed");
    assert!(text.contains("Services Agreement"), "{text}");
    assert!(text.contains("Consulting"), "{text}");
}

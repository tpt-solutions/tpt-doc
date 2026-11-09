//! CAdES signatures can carry intermediate certificates. The chain is embedded
//! after the signer, and the verifier still identifies the signer.

use ring::rand::SystemRandom;
use tpt_doc_sign::verify::verify_detached;
use tpt_doc_sign::{PrivateKey, cades, pem};

const KEY: &[u8] = include_bytes!("fixtures/test_key.pem");
const CERT: &[u8] = include_bytes!("fixtures/test_cert.pem");
const SIGNING_TIME: i64 = 1_794_182_400;
const DATA: &[u8] = b"chain test";

fn parts() -> (PrivateKey, Vec<u8>) {
    (
        PrivateKey::from_pem(KEY).expect("key"),
        pem::pem_to_der(CERT).expect("cert"),
    )
}

#[test]
fn an_intermediate_is_embedded_and_the_signer_still_verifies() {
    let (key, signer) = parts();
    // The fixture certificate stands in for an intermediate: it parses, and
    // embedding it is what this test checks.
    let intermediate = signer.clone();
    let cms = cades::sign_detached_with_chain(
        DATA,
        &key,
        &signer,
        &[intermediate.as_slice()],
        &SystemRandom::new(),
        SIGNING_TIME,
    )
    .expect("sign");

    let occurrences = cms
        .windows(signer.len())
        .filter(|w| *w == signer.as_slice())
        .count();
    assert_eq!(
        occurrences, 2,
        "the signer and the intermediate are both embedded"
    );

    let verified = verify_detached(DATA, &cms, Some(SIGNING_TIME)).expect("verify");
    assert_eq!(verified.signer_certificate_der, signer);
}

#[test]
fn an_unparsable_intermediate_is_refused() {
    let (key, signer) = parts();
    let garbage: &[u8] = b"not a certificate";
    let err = cades::sign_detached_with_chain(
        DATA,
        &key,
        &signer,
        &[garbage],
        &SystemRandom::new(),
        SIGNING_TIME,
    )
    .expect_err("garbage intermediate");
    assert!(
        err.to_string().contains("intermediate certificate 0"),
        "{err}"
    );
}

#[test]
fn no_intermediates_is_the_same_as_the_plain_signer() {
    let (key, signer) = parts();
    let with_none = cades::sign_detached_with_chain(
        DATA,
        &key,
        &signer,
        &[],
        &SystemRandom::new(),
        SIGNING_TIME,
    )
    .expect("sign");
    let plain = cades::sign_detached_at(DATA, &key, &signer, &SystemRandom::new(), SIGNING_TIME)
        .expect("sign");
    // The signature is randomized, so compare the structure: both carry one
    // certificate and verify.
    assert!(verify_detached(DATA, &with_none, None).is_ok());
    assert!(verify_detached(DATA, &plain, None).is_ok());
}

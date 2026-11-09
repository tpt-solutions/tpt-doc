//! The incremental update's `startxref` must be a file offset: the byte at
//! that offset is the `xref` keyword of the appended cross-reference table.
//! Readers that follow the trailer chain depend on this.

use ring::rand::SystemRandom;
use tpt_doc_sign::{PrivateKey, pades, pem};

const KEY: &[u8] = include_bytes!("fixtures/test_key.pem");
const CERT: &[u8] = include_bytes!("fixtures/test_cert.pem");

fn sample_pdf() -> Vec<u8> {
    let mut doc = tpt_doc_pdf::Document::new();
    let font = doc.embed_builtin_font(tpt_doc_pdf::Font::Helvetica);
    let mut page = tpt_doc_pdf::Page::a4();
    page.text("startxref check", font, 12.0, (72.0, 700.0));
    doc.add_page(page);
    doc.write().expect("pdf")
}

/// The offset written after the last `startxref`.
fn last_startxref(bytes: &[u8]) -> usize {
    let marker = b"startxref";
    let at = bytes
        .windows(marker.len())
        .rposition(|w| w == marker)
        .expect("startxref");
    let text = std::str::from_utf8(&bytes[at + marker.len()..]).expect("ascii tail");
    text.split_whitespace()
        .next()
        .expect("offset")
        .parse()
        .expect("numeric offset")
}

#[test]
fn the_signed_update_points_at_its_own_xref_table() {
    let unsigned = sample_pdf();
    let key = PrivateKey::from_pem(KEY).expect("key");
    let cert = pem::pem_to_der(CERT).expect("cert");
    let signed =
        pades::sign_at(&unsigned, &key, &cert, &SystemRandom::new(), 1_794_182_400).expect("sign");

    let offset = last_startxref(&signed);
    assert!(
        signed[offset..].starts_with(b"xref"),
        "startxref {offset} does not point at an xref table"
    );
    assert!(
        offset >= unsigned.len(),
        "the update starts after the original bytes"
    );
}

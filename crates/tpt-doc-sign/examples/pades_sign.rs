//! Sign a generated PDF with a PAdES (CAdES-based) signature.
//!
//! The key and certificate are the self-signed test pair shipped with the
//! test suite; never sign real documents with them.
//!
//! Run with `cargo run -p tpt-doc-sign --example pades_sign`.

use ring::rand::SystemRandom;
use tpt_doc_pdf::{Document, Font, Page};
use tpt_doc_sign::{PrivateKey, pades, pem};

const KEY_PEM: &[u8] = include_bytes!("../tests/fixtures/test_key.pem");
const CERT_PEM: &[u8] = include_bytes!("../tests/fixtures/test_cert.pem");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text("Contract for signature", font, 14.0, (72.0, 760.0));
    doc.add_page(page);
    let pdf = doc.write()?;

    let key = PrivateKey::from_pem(KEY_PEM)?;
    let cert_der = pem::pem_to_der(CERT_PEM)?;
    let signed = pades::sign(&pdf, &key, &cert_der, &SystemRandom::new())?;

    let path = std::env::temp_dir().join("tpt-doc-signed.pdf");
    std::fs::write(&path, &signed)?;
    println!(
        "wrote {} ({} bytes, {} bytes added by the signature)",
        path.display(),
        signed.len(),
        signed.len() - pdf.len()
    );
    Ok(())
}

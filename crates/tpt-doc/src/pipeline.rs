//! The e-invoice pipeline: an invoice as JSON in, a signed Factur-X PDF out.
//!
//! The steps are the ones an invoice goes through before it is sent:
//!
//! 1. Read the invoice from its JSON form (the serde field names of
//!    [`tpt_doc_ubl::Invoice`]).
//! 2. Check it against the PEPPOL BIS Billing 3.0 rules. A failing invoice is
//!    not rendered.
//! 3. Render a summary page, and attach the UBL XML to it as Factur-X.
//! 4. Sign the PDF with a PAdES signature.
//!
//! The signature is made with the caller's key and certificate, and the
//! signing time is supplied by the caller, so the output is reproducible
//! from its inputs apart from the signature's own randomness.

use ring::rand::SecureRandom;
use tpt_doc_core::DocError;
use tpt_doc_pdf::{Document, Font, Page};
use tpt_doc_sign::PrivateKey;
use tpt_doc_ubl::Invoice;

/// The most violations listed in an error, so a broken invoice gives a
/// readable message rather than a page of them.
const MAX_LISTED_VIOLATIONS: usize = 5;

/// Run the pipeline on an invoice given as JSON.
///
/// # Errors
/// Returns [`DocError`] if the JSON is not an invoice, the invoice breaks a
/// PEPPOL rule (the first few are listed), the PDF cannot be written, or the
/// signature cannot be made, including when the certificate is not valid at
/// `unix_seconds` or does not belong to `key`.
pub fn invoice_json_to_signed_pdf(
    json: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
    unix_seconds: i64,
) -> Result<Vec<u8>, DocError> {
    let invoice: Invoice = serde_json::from_slice(json)
        .map_err(|e| DocError::invalid_format(format!("not an invoice: {e}")))?;

    let violations = tpt_doc_ubl::validate(&invoice);
    if !violations.is_empty() {
        let listed: Vec<String> = violations
            .iter()
            .take(MAX_LISTED_VIOLATIONS)
            .map(ToString::to_string)
            .collect();
        return Err(DocError::validation_failed(format!(
            "invoice {} breaks {} rule(s): {}",
            invoice.id,
            violations.len(),
            listed.join("; ")
        )));
    }

    let mut document = Document::new();
    let font = document.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text(
        format!("Invoice {} from {}", invoice.id, invoice.supplier.name),
        font,
        14.0,
        (72.0, 760.0),
    );
    page.text(
        format!("Total {}", invoice.total()),
        font,
        12.0,
        (72.0, 736.0),
    );
    document.add_page(page);
    tpt_doc_ubl::invoice::facturx::attach_xml(&invoice, &mut document)?;

    let pdf = document.write()?;
    tpt_doc_sign::pades::sign_at(&pdf, key, cert_der, rng, unix_seconds)
}

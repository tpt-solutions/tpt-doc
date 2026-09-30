use std::fmt::Write as _;

use base64::Engine as _;
use ring::digest;
use ring::rand::SecureRandom;
use tpt_doc_core::DocError;

use crate::pem::PrivateKey;

/// Sign an XML document with an `XAdES`-style enveloped signature.
///
/// A `<ds:Signature>` element is inserted inside the document's root
/// element. The `SignedInfo` block is generated in canonical form by this
/// crate; the reference digest covers the document bytes as provided (the
/// verifier applies the standard enveloped-signature transform, which
/// removes the `<ds:Signature>` element before digesting).
///
/// The signature algorithm is `rsa-sha256` (RSASSA-PKCS1-v1_5), the XML-DSig
/// default for RSA keys.
///
/// # Errors
/// Returns [`DocError`] if the XML is malformed or the signing operation fails.
pub fn sign(
    xml_bytes: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
) -> Result<Vec<u8>, DocError> {
    let text = std::str::from_utf8(xml_bytes)
        .map_err(|_| DocError::invalid_format("XML is not valid UTF-8"))?;
    let trimmed = text.trim_end();
    if !trimmed.ends_with('>') {
        return Err(DocError::invalid_format("XML document is malformed"));
    }
    let root_close = trimmed
        .rfind("</")
        .ok_or_else(|| DocError::invalid_format("XML document has no closing root element"))?;

    let document_digest = digest::digest(&digest::SHA256, trimmed.as_bytes());
    let signed_info = build_signed_info(document_digest.as_ref());
    // The signature covers the exact bytes of the canonical SignedInfo.
    let signature_value = key.sign_pkcs1_sha256(signed_info.as_bytes(), rng)?;
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature_value);
    let cert_b64 = base64::engine::general_purpose::STANDARD.encode(cert_der);

    let mut out = String::with_capacity(text.len() + signed_info.len() + 1024);
    out.push_str(&text[..root_close]);
    out.push_str("<ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\">");
    out.push_str(&signed_info);
    let _ = write!(
        out,
        "<ds:SignatureValue>{signature_b64}</ds:SignatureValue><ds:KeyInfo><ds:X509Data><ds:X509Certificate>{cert_b64}</ds:X509Certificate></ds:X509Data></ds:KeyInfo>"
    );
    out.push_str("</ds:Signature>");
    out.push_str(&text[root_close..]);
    Ok(out.into_bytes())
}

/// Build the canonical `<ds:SignedInfo>` element.
fn build_signed_info(document_digest: &[u8]) -> String {
    let digest_b64 = base64::engine::general_purpose::STANDARD.encode(document_digest);
    format!(
        "<ds:SignedInfo xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\">\
<ds:CanonicalizationMethod Algorithm=\"http://www.w3.org/TR/2001/REC-xml-c14n-20010315\"/>\
<ds:SignatureMethod Algorithm=\"http://www.w3.org/2001/04/xmldsig-more#rsa-sha256\"/>\
<ds:Reference URI=\"\">\
<ds:Transforms><ds:Transform Algorithm=\"http://www.w3.org/2000/09/xmldsig#enveloped-signature\"/></ds:Transforms>\
<ds:DigestMethod Algorithm=\"http://www.w3.org/2001/04/xmlenc#sha256\"/>\
<ds:DigestValue>{digest_b64}</ds:DigestValue>\
</ds:Reference>\
</ds:SignedInfo>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY_PEM: &str = include_str!("../tests/fixtures/test_key.pem");
    const TEST_CERT_PEM: &str = include_str!("../tests/fixtures/test_cert.pem");

    #[test]
    fn signature_is_enveloped_inside_root() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert");
        let rng = ring::rand::SystemRandom::new();
        let xml = br#"<Invoice xmlns="urn:example"><id>INV-1</id></Invoice>"#;
        let signed = sign(xml, &key, &cert_der, &rng).expect("sign");
        let text = String::from_utf8(signed).expect("UTF-8");
        assert!(text.starts_with("<Invoice"));
        assert!(text.ends_with("</Invoice>"));
        assert!(text.contains("<ds:SignedInfo"));
        assert!(text.contains("<ds:DigestValue>"));
        assert!(text.contains("<ds:SignatureValue>"));
        assert!(text.contains("<ds:X509Certificate>"));
    }

    #[test]
    fn non_xml_input_is_rejected() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert");
        let rng = ring::rand::SystemRandom::new();
        assert!(sign(b"no closing tag", &key, &cert_der, &rng).is_err());
    }
}

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
/// The certificate's validity window is checked against the system clock. Use
/// [`sign_at`] to supply the time explicitly.
///
/// # Errors
/// Returns [`DocError`] if the XML is malformed, the certificate is not valid
/// now or does not belong to `key`, or the signing operation fails.
pub fn sign(
    xml_bytes: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
) -> Result<Vec<u8>, DocError> {
    sign_at(xml_bytes, key, cert_der, rng, crate::pades::unix_now())
}

/// Sign an XML document, taking the signing time as an explicit number of
/// seconds since the Unix epoch.
///
/// The certificate's validity window is checked against `unix_seconds`, so an
/// expired or not-yet-valid certificate fails here rather than producing a
/// signature that no conforming verifier accepts.
///
/// # Errors
/// Returns [`DocError`] if the XML is malformed, the certificate is not valid at
/// `unix_seconds` or does not belong to `key`, or the signing operation fails.
pub fn sign_at(
    xml_bytes: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
    unix_seconds: i64,
) -> Result<Vec<u8>, DocError> {
    // A key that is not the private half of the embedded certificate yields a
    // `<ds:Signature>` that fails verification at the far end; catching it here
    // names the actual mistake.
    let certificate = crate::pem::Certificate::from_der(cert_der)?;
    certificate.check_key_matches(key)?;
    certificate.check_validity(unix_seconds)?;

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
    let qualified = build_qualifying_properties(cert_der);
    // `SignedInfo` carries a digest of the signed properties, so those are
    // built first and the reference is computed over exactly the bytes that
    // will be emitted.
    let signed_properties_digest =
        digest::digest(&digest::SHA256, qualified.signed_properties.as_bytes());
    let signed_info =
        build_signed_info(document_digest.as_ref(), signed_properties_digest.as_ref());
    // The signature covers the exact bytes of the canonical SignedInfo. The
    // `xmlns:ds` declaration sits on `SignedInfo` itself, so the element is
    // signed in its own namespace context rather than relying on an ancestor.
    let signature_value = key.sign_pkcs1_sha256(signed_info.as_bytes(), rng)?;
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature_value);
    let cert_b64 = base64::engine::general_purpose::STANDARD.encode(cert_der);

    let mut out =
        String::with_capacity(text.len() + signed_info.len() + qualified.total_len() + 1024);
    out.push_str(&text[..root_close]);
    let _ = write!(
        out,
        "<ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\" Id=\"{SIGNATURE_ID}\">{signed_info}<ds:SignatureValue>{signature_b64}</ds:SignatureValue><ds:KeyInfo><ds:X509Data><ds:X509Certificate>{cert_b64}</ds:X509Certificate></ds:X509Data></ds:KeyInfo>"
    );
    out.push_str(&qualified.document_object);
    out.push_str("</ds:Signature>");
    out.push_str(&text[root_close..]);
    Ok(out.into_bytes())
}

/// `Id` of the `<ds:Signature>` element, which `QualifyingProperties/@Target`
/// points at.
const SIGNATURE_ID: &str = "tptdoc-signature";

/// `Id` of the `<xades:SignedProperties>` element.
const SIGNED_PROPERTIES_ID: &str = "tptdoc-signed-properties";

/// The `Type` attribute identifying a reference as `XAdES` signed properties.
const URI_SIGNED_PROPERTIES: &str = "http://uri.etsi.org/01903#SignedProperties";

/// The `XAdES` pieces of a signature: the signed properties themselves, and the
/// `<ds:Object>` wrapper that goes inside `<ds:Signature>`.
struct QualifiedProperties {
    /// The `<xades:SignedProperties>` element, digested by `SignedInfo`.
    signed_properties: String,
    /// The full `<ds:Object>…` block to emit after `<ds:KeyInfo>`.
    document_object: String,
}

impl QualifiedProperties {
    /// Combined length, for output pre-allocation.
    fn total_len(&self) -> usize {
        self.signed_properties.len() + self.document_object.len()
    }
}

/// Build the `XAdES` `QualifyingProperties` block.
///
/// This is what makes the signature `XAdES` rather than plain `XML-DSig`: without
/// it the signature carries no proof of which certificate signed it, and ETSI
/// validators reject the document. `SigningCertificate` binds the signer
/// certificate's digest (and serial number) into the signed properties, so a
/// substituted certificate is detectable.
fn build_qualifying_properties(cert_der: &[u8]) -> QualifiedProperties {
    let cert_digest = digest::digest(&digest::SHA256, cert_der);
    let digest_b64 = base64::engine::general_purpose::STANDARD.encode(cert_digest.as_ref());
    let issuer_serial = xades_issuer_serial(cert_der);

    let signed_properties = format!(
        "<xades:SignedProperties xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\" Id=\"{SIGNED_PROPERTIES_ID}\">\
<xades:SignedSignatureProperties>\
<xades:SigningCertificate><xades:Cert>\
<xades:CertDigest><ds:DigestMethod Algorithm=\"http://www.w3.org/2001/04/xmlenc#sha256\"/><ds:DigestValue>{digest_b64}</ds:DigestValue></xades:CertDigest>\
{issuer_serial}\
</xades:Cert></xades:SigningCertificate>\
</xades:SignedSignatureProperties></xades:SignedProperties>"
    );
    let document_object = format!(
        "<ds:Object xmlns:xades=\"http://uri.etsi.org/01903/v1.3.2#\"><xades:QualifyingProperties Target=\"#{SIGNATURE_ID}\">{signed_properties}</xades:QualifyingProperties></ds:Object>"
    );
    QualifiedProperties {
        signed_properties,
        document_object,
    }
}

/// Build the canonical `<ds:SignedInfo>` element.
///
/// `xmlns:ds` is declared on the element itself, so the signature is computed
/// over `SignedInfo` in its own namespace context rather than relying on an
/// inherited declaration. The text is emitted with no incidental whitespace,
/// which is what exclusive C14N would produce, so a verifier canonicalising the
/// element reproduces these bytes exactly.
fn build_signed_info(document_digest: &[u8], signed_properties_digest: &[u8]) -> String {
    let digest_b64 = base64::engine::general_purpose::STANDARD.encode(document_digest);
    let properties_b64 = base64::engine::general_purpose::STANDARD.encode(signed_properties_digest);
    format!(
        "<ds:SignedInfo xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\">\
<ds:CanonicalizationMethod Algorithm=\"http://www.w3.org/TR/2001/REC-xml-c14n-20010315\"/>\
<ds:SignatureMethod Algorithm=\"http://www.w3.org/2001/04/xmldsig-more#rsa-sha256\"/>\
<ds:Reference URI=\"\">\
<ds:Transforms><ds:Transform Algorithm=\"http://www.w3.org/2000/09/xmldsig#enveloped-signature\"/></ds:Transforms>\
<ds:DigestMethod Algorithm=\"http://www.w3.org/2001/04/xmlenc#sha256\"/>\
<ds:DigestValue>{digest_b64}</ds:DigestValue>\
</ds:Reference>\
<ds:Reference URI=\"#{SIGNED_PROPERTIES_ID}\" Type=\"{URI_SIGNED_PROPERTIES}\">\
<ds:DigestMethod Algorithm=\"http://www.w3.org/2001/04/xmlenc#sha256\"/>\
<ds:DigestValue>{properties_b64}</ds:DigestValue>\
</ds:Reference>\
</ds:SignedInfo>"
    )
}

/// Build `<xades:IssuerSerial>`, pinning the certificate to its issuer.
///
/// The serial number distinguishes a certificate from others sharing a digest,
/// and `IssuerName` identifies the authority. `x509-parser` 0.16 does not
/// render the issuer back to an RDN structure, so the serial number is emitted
/// on its own; the certificate digest above remains the primary binding.
fn xades_issuer_serial(cert_der: &[u8]) -> String {
    let Ok((_, parsed)) = x509_parser::parse_x509_certificate(cert_der) else {
        // The certificate was already validated by `sign`, so this cannot
        // happen; an empty element is still well-formed if it somehow does.
        return String::from("<xades:IssuerSerial/>");
    };
    let serial = parsed.tbs_certificate.serial.to_bytes_be();
    let mut hex = String::with_capacity(serial.len() * 2);
    for byte in serial {
        let _ = write!(hex, "{byte:02X}");
    }
    format!(
        "<xades:IssuerSerial><xades:SerialNumber>{hex}</xades:SerialNumber></xades:IssuerSerial>"
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

    /// The `XAdES` `QualifyingProperties` block is what distinguishes an `XAdES`
    /// signature from plain `XML-DSig`; without it `ETSI` validators reject the
    /// document and the signer cannot be checked from the signature itself.
    #[test]
    fn qualifying_properties_are_emitted() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert");
        let rng = ring::rand::SystemRandom::new();
        let xml = br#"<Invoice xmlns="urn:example"><id>INV-1</id></Invoice>"#;
        let text =
            String::from_utf8(sign(xml, &key, &cert_der, &rng).expect("sign")).expect("UTF-8");

        assert!(text.contains("xades:QualifyingProperties"), "{text}");
        assert!(text.contains("xades:SignedProperties"), "{text}");
        assert!(text.contains("xades:SigningCertificate"), "{text}");
        assert!(text.contains("xades:CertDigest"), "{text}");
        assert!(text.contains("xades:IssuerSerial"), "{text}");
        // The properties must be signed, so SignedInfo references them.
        assert!(text.contains(URI_SIGNED_PROPERTIES), "{text}");
        assert!(text.contains(SIGNED_PROPERTIES_ID), "{text}");
        // Target must name the signature that carries the properties.
        assert!(
            text.contains(&format!("Target=\"#{SIGNATURE_ID}\"")),
            "{text}"
        );
        assert!(text.contains(&format!("Id=\"{SIGNATURE_ID}\"")), "{text}");
    }

    /// The digest in `SignedInfo` must be the digest of the `SignedProperties`
    /// bytes actually emitted. A reference that does not match would make the
    /// signature unverifiable.
    #[test]
    fn the_signed_properties_digest_matches_the_emitted_bytes() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert");
        let rng = ring::rand::SystemRandom::new();
        let xml = br#"<Invoice xmlns="urn:example"><id>INV-1</id></Invoice>"#;
        let text =
            String::from_utf8(sign(xml, &key, &cert_der, &rng).expect("sign")).expect("UTF-8");

        // Extract the SignedProperties element verbatim from the output.
        let start = text
            .find("<xades:SignedProperties")
            .expect("SignedProperties");
        let end = text
            .find("</xades:SignedProperties>")
            .map(|i| i + "</xades:SignedProperties>".len())
            .expect("SignedProperties close");
        let emitted = &text[start..end];

        let expected = digest::digest(&digest::SHA256, emitted.as_bytes());
        let expected_b64 = base64::engine::general_purpose::STANDARD.encode(expected.as_ref());

        // The SignedInfo reference must carry exactly that digest.
        let uri = format!("URI=\"#{SIGNED_PROPERTIES_ID}\"");
        let ref_start = text.find(&uri).expect("SignedProperties reference");
        let rest = &text[ref_start..];
        let value_start =
            rest.find("<ds:DigestValue>").expect("DigestValue") + "<ds:DigestValue>".len();
        let value_end = rest[value_start..]
            .find("</ds:DigestValue>")
            .expect("DigestValue close");
        assert_eq!(
            &rest[value_start..value_start + value_end],
            expected_b64,
            "SignedInfo must digest the bytes actually emitted"
        );
    }

    /// The signer certificate digest in `CertDigest` must match the embedded
    /// certificate, which is what binds the signature to a certificate.
    #[test]
    fn the_certificate_digest_matches_the_embedded_certificate() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert");
        let rng = ring::rand::SystemRandom::new();
        let xml = br#"<Invoice xmlns="urn:example"><id>INV-1</id></Invoice>"#;
        let text =
            String::from_utf8(sign(xml, &key, &cert_der, &rng).expect("sign")).expect("UTF-8");

        let expected = digest::digest(&digest::SHA256, &cert_der);
        let expected_b64 = base64::engine::general_purpose::STANDARD.encode(expected.as_ref());

        let marker = "<xades:CertDigest>";
        let start = text.find(marker).expect("CertDigest") + marker.len();
        let rest = &text[start..];
        let value_start =
            rest.find("<ds:DigestValue>").expect("DigestValue") + "<ds:DigestValue>".len();
        let value_end = rest[value_start..]
            .find("</ds:DigestValue>")
            .expect("close");
        assert_eq!(&rest[value_start..value_start + value_end], expected_b64);
    }

    /// An expired certificate must fail here, not produce a `<ds:Signature>`
    /// that a verifier rejects.
    #[test]
    fn an_expired_certificate_is_rejected_at_signing_time() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert");
        let certificate = crate::pem::Certificate::from_der(&cert_der).expect("cert");
        let rng = ring::rand::SystemRandom::new();
        let xml = br#"<Invoice xmlns="urn:example"><id>INV-1</id></Invoice>"#;

        let err = sign_at(xml, &key, &cert_der, &rng, certificate.not_after() + 1)
            .expect_err("expired certificate must fail");
        assert!(err.to_string().contains("expired"), "{err}");

        assert!(
            sign_at(xml, &key, &cert_der, &rng, certificate.not_before()).is_ok(),
            "notBefore is inclusive"
        );
    }

    #[test]
    fn non_xml_input_is_rejected() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert");
        let rng = ring::rand::SystemRandom::new();
        assert!(sign(b"no closing tag", &key, &cert_der, &rng).is_err());
    }
}

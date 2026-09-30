use ring::digest;
use ring::rand::SecureRandom;
use tpt_doc_core::DocError;
use x509_parser::prelude::*;

use crate::der;
use crate::pem::PrivateKey;

/// OID arcs used in CMS/CAdES structures.
mod oid {
    pub const SIGNED_DATA: &str = "1.2.840.113549.1.7.2";
    pub const DATA: &str = "1.2.840.113549.1.7.1";
    pub const SHA256: &str = "2.16.840.1.101.3.4.2.1";
    pub const CONTENT_TYPE: &str = "1.2.840.113549.1.9.3";
    pub const MESSAGE_DIGEST: &str = "1.2.840.113549.1.9.4";
    pub const SIGNING_CERTIFICATE_V2: &str = "1.2.840.113549.1.9.16.2.47";
    pub const RSA_PSS: &str = "1.2.840.113549.1.1.10";
    pub const MGF1: &str = "1.2.840.113549.1.1.8";
}

/// Create a `CAdES-BES` detached signature over `data`.
///
/// Returns the DER-encoded CMS `SignedData` with the signer certificate
/// embedded. The content itself is not embedded (detached); a verifier
/// digests `data` independently via the `messageDigest` signed attribute.
///
/// The signature is RSA-PSS with SHA-256, MGF1-SHA-256, and a 32-byte salt.
///
/// # Errors
/// Returns [`DocError`] if the certificate cannot be parsed or signing fails.
pub fn sign_detached(
    data: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
) -> Result<Vec<u8>, DocError> {
    let (_, cert) = parse_x509_certificate(cert_der)
        .map_err(|e| DocError::invalid_format(format!("signer certificate unparsable: {e}")))?;
    let issuer_der = issuer_tlv(cert_der)?;
    let serial = cert.tbs_certificate.serial.to_bytes_be();
    let cert_digest = digest::digest(&digest::SHA256, cert_der);

    let message_digest = digest::digest(&digest::SHA256, data);
    let signed_attrs = build_signed_attrs(message_digest.as_ref(), cert_digest.as_ref());

    // The signature covers the DER encoding of SignedAttrs re-tagged as a
    // SET OF (0x31), per RFC 5652 §11.4.
    let set_of_attrs = re_tag_set_of(&signed_attrs);
    let signature = key.sign_pss_sha256(&set_of_attrs, rng)?;

    let signer_info_body = [
        der::integer(&[1]),
        issuer_and_serial(&issuer_der, &serial),
        digest_algorithm_sha256(),
        der::context_constructed(0, &signed_attrs),
        signature_algorithm_pss(),
        der::octet_string(&signature),
    ]
    .concat();
    let signer_info = der::sequence(&signer_info_body);

    let signed_data_body = [
        der::integer(&[1]),
        der::set_of(&digest_algorithm_sha256()),
        encapsulated_content_info_detached(),
        der::context_constructed(0, cert_der),
        der::set_of(&signer_info),
    ]
    .concat();
    let signed_data = der::sequence(&signed_data_body);
    Ok(der::sequence(
        &[
            der::oid(oid::SIGNED_DATA),
            der::context_constructed(0, &signed_data),
        ]
        .concat(),
    ))
}

fn issuer_and_serial(issuer_der: &[u8], serial: &[u8]) -> Vec<u8> {
    der::sequence(&[issuer_der.to_vec(), der::integer(serial)].concat())
}

/// Extract the raw DER TLV of the `issuer` Name from a certificate.
///
/// `Certificate ::= SEQUENCE { tbsCertificate SEQUENCE { version [0]?,
/// serial INTEGER, signature AlgorithmIdentifier, issuer Name, … } }` —
/// walk those top-level TLVs inside the TBS certificate. (x509-parser 0.16
/// leaves `X509Name::as_raw` empty on this parse path.)
fn issuer_tlv(cert_der: &[u8]) -> Result<Vec<u8>, DocError> {
    let err = || DocError::invalid_format("certificate issuer extraction failed");
    // Certificate SEQUENCE header; its content starts with the TBS.
    let (_, _, outer_header) = read_tlv(cert_der, 0).ok_or_else(err)?;
    // TBS SEQUENCE header.
    let (_, _, tbs_header) = read_tlv(cert_der, outer_header).ok_or_else(err)?;
    let mut pos = outer_header + tbs_header;
    // Optional [0] EXPLICIT version.
    if cert_der.get(pos) == Some(&0xA0) {
        let (_, content_len, header_len) = read_tlv(cert_der, pos).ok_or_else(err)?;
        pos += header_len + content_len;
    }
    // serialNumber INTEGER.
    let (_, len, header) = read_tlv(cert_der, pos).ok_or_else(err)?;
    pos += header + len;
    // signature AlgorithmIdentifier.
    let (_, len, header) = read_tlv(cert_der, pos).ok_or_else(err)?;
    pos += header + len;
    // issuer Name TLV.
    let (_, len, header) = read_tlv(cert_der, pos).ok_or_else(err)?;
    let end = pos + header + len;
    cert_der.get(pos..end).map(<[u8]>::to_vec).ok_or_else(err)
}

/// Read one TLV: returns (tag, content length, total header length).
fn read_tlv(data: &[u8], pos: usize) -> Option<(u8, usize, usize)> {
    let tag = *data.get(pos)?;
    let first = *data.get(pos + 1)?;
    let (content_len, header_len) = if first & 0x80 == 0 {
        (usize::from(first), 2)
    } else {
        let n = usize::from(first & 0x7F);
        let mut len = 0usize;
        for i in 0..n {
            len = len << 8 | usize::from(*data.get(pos + 2 + i)?);
        }
        (len, 2 + n)
    };
    Some((tag, content_len, header_len))
}

fn digest_algorithm_sha256() -> Vec<u8> {
    der::sequence(&der::oid(oid::SHA256))
}

fn encapsulated_content_info_detached() -> Vec<u8> {
    der::sequence(&der::oid(oid::DATA))
}

/// RSA-PSS `SignatureAlgorithmIdentifier`: SHA-256 hash, MGF1-SHA-256,
/// 32-byte salt (RFC 4055 §3.1). The hash `AlgorithmIdentifier`s carry the
/// conventional NULL parameter, matching what OpenSSL and Windows emit.
fn signature_algorithm_pss() -> Vec<u8> {
    let sha256_with_null = der::sequence(&[der::oid(oid::SHA256), der::null()].concat());
    let hash_alg = der::context_constructed(0, &sha256_with_null);
    let mgf1 = der::sequence(&[der::oid(oid::MGF1), sha256_with_null].concat());
    let mask_gen = der::context_constructed(1, &mgf1);
    let salt = der::context_constructed(2, &der::integer(&[32]));
    // RSA_PSS_PARAMS ::= SEQUENCE { hashAlg [0], maskGen [1], saltLength [2] }
    let params = der::sequence(&[hash_alg, mask_gen, salt].concat());
    der::sequence(&[der::oid(oid::RSA_PSS), params].concat())
}

/// Build the concatenated DER encoding of the signed attributes:
/// `contentType`, `messageDigest`, and `signingCertificateV2` (`ESSCertIDv2`
/// with SHA-256). The encodings are DER-sorted as `SET OF` requires.
fn build_signed_attrs(message_digest: &[u8], cert_digest: &[u8]) -> Vec<u8> {
    let attr =
        |oid_der: Vec<u8>, value: Vec<u8>| der::sequence(&[oid_der, der::set_of(&value)].concat());
    let content_type = attr(der::oid(oid::CONTENT_TYPE), der::oid(oid::DATA));
    let message_digest_attr = attr(
        der::oid(oid::MESSAGE_DIGEST),
        der::octet_string(message_digest),
    );
    // ESSCertIDv2 ::= SEQUENCE { hashAlgorithm AlgorithmIdentifier,
    //                            certHash OCTET STRING,
    //                            issuerSerial [0] IssuerSerial OPTIONAL }
    let ess_cert_id = der::sequence(
        &[
            der::sequence(&der::oid(oid::SHA256)),
            der::octet_string(cert_digest),
        ]
        .concat(),
    );
    let signing_certificate_v2 = attr(
        der::oid(oid::SIGNING_CERTIFICATE_V2),
        der::sequence(&[ess_cert_id].concat()),
    );

    let mut attrs: Vec<Vec<u8>> = vec![content_type, message_digest_attr, signing_certificate_v2];
    attrs.sort();
    let mut encoded = Vec::new();
    for attr in &attrs {
        encoded.extend_from_slice(attr);
    }
    encoded
}

/// Wrap the concatenated attribute TLVs in a real SET OF (0x31) tag —
/// inside `SignerInfo` they carry the implicit `[0]` tag, but the signature
/// is computed over the SET OF encoding (RFC 5652 §5.4).
fn re_tag_set_of(attributes_body: &[u8]) -> Vec<u8> {
    der::set_of(attributes_body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY_PEM: &str = include_str!("../tests/fixtures/test_key.pem");
    const TEST_CERT_PEM: &str = include_str!("../tests/fixtures/test_cert.pem");

    #[test]
    fn cades_structure_embeds_issuer_and_cert() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert der");
        let rng = ring::rand::SystemRandom::new();
        let cms = sign_detached(b"document bytes", &key, &cert_der, &rng).expect("sign");

        // The issuer DER is embedded via IssuerAndSerialNumber.
        let (_, cert) = parse_x509_certificate(&cert_der).expect("cert parses");
        let issuer = cert.tbs_certificate.issuer.as_raw();
        assert!(
            cms.windows(issuer.len()).any(|w| w == issuer),
            "issuer DER embedded in CMS"
        );
        // The certificate itself is embedded in the certificates [0] field.
        assert!(
            cms.windows(cert_der.len()).any(|w| w == cert_der),
            "certificate embedded in CMS"
        );
    }

    #[test]
    fn cades_length_is_value_independent() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert der");
        let rng = ring::rand::SystemRandom::new();
        let a = sign_detached(b"short", &key, &cert_der, &rng).expect("sign a");
        let b = sign_detached(&vec![7u8; 4096], &key, &cert_der, &rng).expect("sign b");
        assert_eq!(a.len(), b.len(), "CMS length must not depend on content");
    }
}

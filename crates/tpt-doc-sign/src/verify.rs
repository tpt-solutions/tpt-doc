//! Verification of CAdES detached signatures, in Rust.
//!
//! This checks what the signer in [`cades`](crate::cades) produces, and the
//! standard RSA CMS forms: a detached `SignedData` with one signer, whose
//! signed attributes bind the content through `messageDigest`, and whose
//! signature (RSA-PSS or PKCS#1 v1.5, SHA-256) covers those attributes. The
//! signer's certificate must be embedded and must match the signer identifier.
//!
//! Verification does not check the certificate chain to a trust anchor. It
//! checks that the signature was made by the key in the embedded certificate,
//! and, when a time is given, that the certificate was valid then. Deciding
//! whether that certificate is trusted is the caller's job.

use ring::digest;
use ring::signature::{RSA_PKCS1_2048_8192_SHA256, RSA_PSS_2048_8192_SHA256, UnparsedPublicKey};
use tpt_doc_core::DocError;
use x509_parser::parse_x509_certificate;

use crate::cades::{issuer_tlv, oid, read_tlv};
use crate::der;
use crate::pem::Certificate;

/// The result of a successful verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// DER of the certificate whose key made the signature.
    pub signer_certificate_der: Vec<u8>,
}

/// Verify a detached `CAdES` signature over `content`.
///
/// `cms_der` is the DER `ContentInfo` the signer produced. `at_unix_seconds`,
/// when given, also checks that the signer's certificate was valid at that
/// time.
///
/// # Errors
/// Returns [`DocError`] if the signature is malformed or uses an unsupported
/// algorithm, the content does not match the signed `messageDigest`, the
/// signer's certificate is missing or does not match the signer identifier, the
/// signature does not verify, or the certificate was not valid at the given
/// time. Only a single signer is supported.
pub fn verify_detached(
    content: &[u8],
    cms_der: &[u8],
    at_unix_seconds: Option<i64>,
) -> Result<Verified, DocError> {
    let malformed = || DocError::invalid_format("CMS structure is malformed");

    // ContentInfo ::= SEQUENCE { contentType, [0] EXPLICIT SignedData }
    let content_info = Tlv::read(cms_der)
        .ok_or_else(malformed)?
        .children()
        .ok_or_else(malformed)?;
    let [content_type, wrapped] = content_info.as_slice() else {
        return Err(malformed());
    };
    if content_type.raw != der::oid(oid::SIGNED_DATA)?.as_slice() {
        return Err(DocError::invalid_format("CMS content is not SignedData"));
    }
    let signed_data = Tlv::read(wrapped.content).ok_or_else(malformed)?;
    let fields = signed_data.children().ok_or_else(malformed)?;

    // Certificates [0] IMPLICIT, and the single SignerInfo in the SET at the end.
    let certificates: Vec<Vec<u8>> = fields
        .iter()
        .find(|f| f.tag == 0xA0)
        .and_then(Tlv::children)
        .ok_or_else(|| DocError::invalid_format("CMS carries no certificates"))?
        .iter()
        .map(|c| c.raw.to_vec())
        .collect();
    let signer_infos = fields
        .last()
        .filter(|f| f.tag == 0x31)
        .and_then(Tlv::children)
        .ok_or_else(malformed)?;
    let [signer_info] = signer_infos.as_slice() else {
        return Err(DocError::invalid_format(
            "only a single signer is supported",
        ));
    };

    // SignerInfo ::= SEQUENCE { version, sid, digestAlg, [0] signedAttrs,
    //                           signatureAlg, signature OCTET STRING, … }
    let info = signer_info.children().ok_or_else(malformed)?;
    let [
        _version,
        sid,
        digest_alg,
        signed_attrs,
        signature_alg,
        signature,
        ..,
    ] = info.as_slice()
    else {
        return Err(malformed());
    };
    if signed_attrs.tag != 0xA0 || signature.tag != 0x04 {
        return Err(malformed());
    }
    // The digest is SHA-256 in every form this verifier accepts.
    let digest_oid = digest_alg
        .children()
        .and_then(|parts| parts.into_iter().next())
        .ok_or_else(malformed)?;
    if digest_oid.raw != der::oid(oid::SHA256)?.as_slice() {
        return Err(DocError::invalid_format(
            "the digest algorithm is not SHA-256",
        ));
    }

    // The signed messageDigest must be the digest of the content.
    let message_digest = attribute_value(signed_attrs, &der::oid(oid::MESSAGE_DIGEST)?)
        .ok_or_else(|| DocError::invalid_format("signed attributes carry no messageDigest"))?;
    let actual = digest::digest(&digest::SHA256, content);
    if message_digest != actual.as_ref() {
        return Err(DocError::validation_failed(
            "the content does not match the signed messageDigest",
        ));
    }

    // The signer certificate: the one whose issuer and serial match the sid.
    let sid_fields = sid.children().ok_or_else(malformed)?;
    let (sid_issuer, sid_serial) = match sid_fields.as_slice() {
        [issuer, serial, ..] => (issuer.raw, serial.content),
        _ => return Err(malformed()),
    };
    let signer_der = certificates
        .iter()
        .find(|cert| matches_signer(cert, sid_issuer, sid_serial))
        .ok_or_else(|| DocError::invalid_format("the signer's certificate is not embedded"))?
        .clone();
    let (_, parsed) = parse_x509_certificate(&signer_der)
        .map_err(|e| DocError::invalid_format(format!("signer certificate unparsable: {e}")))?;

    // The signature covers the signed attributes re-tagged as SET OF (0x31).
    let mut to_verify = signed_attrs.raw.to_vec();
    to_verify[0] = 0x31;
    let public_key = parsed.public_key().subject_public_key.data.as_ref();
    let algorithm = signature_algorithm(signature_alg.content)?;
    UnparsedPublicKey::new(algorithm, public_key)
        .verify(&to_verify, signature.content)
        .map_err(|_| DocError::validation_failed("the signature does not verify"))?;

    if let Some(at) = at_unix_seconds {
        Certificate::from_der(&signer_der)?.check_validity(at)?;
    }
    Ok(Verified {
        signer_certificate_der: signer_der,
    })
}

/// Which verification algorithm a `signatureAlgorithm` names.
fn signature_algorithm(
    alg_content: &[u8],
) -> Result<&'static dyn ring::signature::VerificationAlgorithm, DocError> {
    let oid_tlv = Tlv::read(alg_content)
        .ok_or_else(|| DocError::invalid_format("signature algorithm is malformed"))?;
    if oid_tlv.raw == der::oid(oid::RSA_PSS)?.as_slice() {
        Ok(&RSA_PSS_2048_8192_SHA256)
    } else if oid_tlv.raw == der::oid(oid::SHA256_WITH_RSA)?.as_slice()
        || oid_tlv.raw == der::oid(oid::RSA_ENCRYPTION)?.as_slice()
    {
        // The digest is SHA-256 (checked by the caller), so rsaEncryption means
        // PKCS#1 v1.5 over SHA-256, which is what OpenSSL writes.
        Ok(&RSA_PKCS1_2048_8192_SHA256)
    } else {
        Err(DocError::invalid_format(
            "signature algorithm is not RSA-PSS or SHA-256 with RSA",
        ))
    }
}

/// The first value (inside the attribute's SET) of an attribute with the
/// given OID, as the OCTET STRING content of `messageDigest`.
fn attribute_value<'a>(signed_attrs: &Tlv<'a>, attribute_oid: &[u8]) -> Option<&'a [u8]> {
    for attribute in signed_attrs.children()? {
        let parts = attribute.children()?;
        let [oid_tlv, values] = parts.as_slice() else {
            continue;
        };
        if oid_tlv.raw == attribute_oid {
            let value = values.children()?.into_iter().next()?;
            return Some(value.content);
        }
    }
    None
}

/// Whether a certificate is the signer named by an issuer and serial.
fn matches_signer(cert_der: &[u8], sid_issuer: &[u8], sid_serial: &[u8]) -> bool {
    let Ok((_, cert)) = parse_x509_certificate(cert_der) else {
        return false;
    };
    let serial = cert.tbs_certificate.serial.to_bytes_be();
    let trim = |bytes: &[u8]| {
        bytes
            .iter()
            .skip_while(|b| **b == 0)
            .copied()
            .collect::<Vec<u8>>()
    };
    let Ok(issuer) = issuer_tlv(cert_der) else {
        return false;
    };
    issuer == sid_issuer && trim(&serial) == trim(sid_serial)
}

/// One DER element: its tag, its content, and its complete encoding.
struct Tlv<'a> {
    tag: u8,
    content: &'a [u8],
    raw: &'a [u8],
}

impl<'a> Tlv<'a> {
    /// Read the first element of `data`.
    fn read(data: &'a [u8]) -> Option<Self> {
        let (tag, content_len, header_len) = read_tlv(data, 0)?;
        let end = header_len.checked_add(content_len)?;
        Some(Self {
            tag,
            content: data.get(header_len..end)?,
            raw: data.get(..end)?,
        })
    }

    /// The elements inside a constructed element, in order.
    fn children(&self) -> Option<Vec<Tlv<'a>>> {
        let mut out = Vec::new();
        let mut rest = self.content;
        while !rest.is_empty() {
            let (tag, content_len, header_len) = read_tlv(rest, 0)?;
            let end = header_len.checked_add(content_len)?;
            out.push(Tlv {
                tag,
                content: rest.get(header_len..end)?,
                raw: rest.get(..end)?,
            });
            rest = rest.get(end..)?;
        }
        Some(out)
    }
}

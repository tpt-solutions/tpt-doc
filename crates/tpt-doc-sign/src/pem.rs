use base64::Engine as _;
use ring::signature::RsaKeyPair;
use tpt_doc_core::DocError;

/// A loaded RSA signing key.
///
/// Wraps `ring::signature::RsaKeyPair` loaded from a PKCS#8 PEM document.
pub struct PrivateKey {
    key_pair: RsaKeyPair,
    /// Modulus length in bytes (e.g. 256 for RSA-2048).
    modulus_len: usize,
}

impl std::fmt::Debug for PrivateKey {
    // Key material is intentionally omitted from the output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivateKey")
            .field("modulus_len", &self.modulus_len)
            .finish_non_exhaustive()
    }
}

/// A parsed X.509 certificate, with the fields signing needs to check.
pub struct Certificate {
    /// The DER bytes exactly as supplied, for embedding in a signature.
    der: Vec<u8>,
    /// `SubjectPublicKeyInfo` subjectPublicKey BIT STRING contents: the DER
    /// body of the RSA public key or the EC point.
    public_key: Vec<u8>,
    /// Algorithm OID of the public key: `rsaEncryption` or `id-ecPublicKey`.
    public_key_algorithm: String,
    /// Inclusive validity window.
    not_before: i64,
    not_after: i64,
}

impl std::fmt::Debug for Certificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Certificate")
            .field("public_key_algorithm", &self.public_key_algorithm)
            .field("not_before", &self.not_before)
            .field("not_after", &self.not_after)
            .finish_non_exhaustive()
    }
}

impl Certificate {
    /// Parse a DER-encoded X.509 certificate.
    ///
    /// # Errors
    /// Returns [`DocError`] if the certificate cannot be parsed, or if its
    /// subject public key is not RSA or EC.
    pub fn from_der(der: &[u8]) -> Result<Self, DocError> {
        let (_, parsed) = x509_parser::parse_x509_certificate(der)
            .map_err(|e| DocError::invalid_format(format!("certificate unparsable: {e}")))?;
        let spki = parsed.public_key();
        let public_key_algorithm = match spki.algorithm.algorithm.to_id_string().as_str() {
            // rsaEncryption
            "1.2.840.113549.1.1.1" => "rsaEncryption".to_owned(),
            // id-ecPublicKey
            "1.2.840.10045.2.1" => "id-ecPublicKey".to_owned(),
            other => {
                return Err(DocError::invalid_format(format!(
                    "unsupported public key algorithm {other}"
                )));
            }
        };
        let public_key = spki.subject_public_key.data.to_vec();
        Ok(Self {
            der: der.to_vec(),
            public_key,
            public_key_algorithm,
            not_before: parsed.validity().not_before.timestamp(),
            not_after: parsed.validity().not_after.timestamp(),
        })
    }

    /// Parse a PEM-encoded certificate.
    ///
    /// # Errors
    /// Returns [`DocError`] if the PEM armor is malformed or the DER does not
    /// parse as a certificate.
    pub fn from_pem(pem: &[u8]) -> Result<Self, DocError> {
        Self::from_der(&crate::pem::pem_to_der(pem)?)
    }

    /// The DER bytes exactly as supplied.
    #[must_use]
    pub fn der(&self) -> &[u8] {
        &self.der
    }

    /// The signing algorithm this certificate's public key implies.
    #[must_use]
    pub fn public_key_algorithm(&self) -> &str {
        &self.public_key_algorithm
    }

    /// Check the certificate is valid at `unix_seconds`.
    ///
    /// Both bounds are inclusive, matching RFC 5280 §4.1.2.5.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if the timestamp is outside the
    /// certificate's `notBefore`/`notAfter` window.
    pub fn check_validity(&self, unix_seconds: i64) -> Result<(), DocError> {
        if unix_seconds < self.not_before {
            return Err(DocError::invalid_format(format!(
                "certificate is not valid until unix {}, but signing at {unix_seconds}",
                self.not_before
            )));
        }
        if unix_seconds > self.not_after {
            return Err(DocError::invalid_format(format!(
                "certificate expired at unix {}, but signing at {unix_seconds}",
                self.not_after
            )));
        }
        Ok(())
    }

    /// Check that `key` is the private half of this certificate's public key.
    ///
    /// A mismatch produces a signature no verifier can attribute to the
    /// embedded certificate, so it is far cheaper to catch here than to
    /// diagnose from a rejected document.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if the key is not RSA or the
    /// modulus differs from the certificate's.
    pub fn check_key_matches(&self, key: &PrivateKey) -> Result<(), DocError> {
        if self.public_key_algorithm != "rsaEncryption" {
            return Err(DocError::invalid_format(format!(
                "certificate uses {}, but the supplied key is RSA",
                self.public_key_algorithm
            )));
        }
        // RSAPublicKey ::= SEQUENCE { modulus INTEGER, publicExponent INTEGER }
        // so the modulus is the first INTEGER inside the SPKI bit string.
        let modulus = read_rsa_modulus(&self.public_key)
            .ok_or_else(|| DocError::invalid_format("certificate RSA public key is malformed"))?;
        let components = key.public_key();
        if modulus != components.n {
            return Err(DocError::invalid_format(
                "private key does not match the certificate's public key",
            ));
        }
        Ok(())
    }
}

/// Extract the RSA modulus from an `RSAPublicKey` DER body.
fn read_rsa_modulus(spki_bits: &[u8]) -> Option<Vec<u8>> {
    let sequence = read_tlv(spki_bits, 0, 0x30)?;
    let modulus = read_tlv(sequence.content, 0, 0x02)?;
    // Strip the DER sign pad so the comparison is against the raw magnitude.
    let significant = modulus
        .content
        .iter()
        .position(|b| *b != 0)
        .unwrap_or(modulus.content.len().saturating_sub(1));
    Some(modulus.content[significant..].to_vec())
}

impl PrivateKey {
    /// Load an RSA private key from a PEM document.
    ///
    /// Accepts both PKCS#8 (`-----BEGIN PRIVATE KEY-----`) and PKCS#1
    /// (`-----BEGIN RSA PRIVATE KEY-----`) armor, since `openssl genrsa`
    /// emits the latter by default and PKCS#1 keys are common in the wild.
    /// PKCS#1 is wrapped into PKCS#8 before loading.
    ///
    /// # Errors
    /// Returns [`DocError`] if the PEM armor is malformed, base64 decoding
    /// fails, or the DER document is neither a valid PKCS#8 nor a valid
    /// PKCS#1 RSA key.
    pub fn from_pem(pem: &[u8]) -> Result<Self, DocError> {
        let der = pem_to_der(pem)?;
        // PKCS#8 first: if the document is already wrapped, this succeeds and
        // the PKCS#1 fallback is skipped.
        let key_pair = if let Ok(pair) = RsaKeyPair::from_pkcs8(&der) {
            pair
        } else {
            let pkcs8 = pkcs1_to_pkcs8(&der)?;
            RsaKeyPair::from_pkcs8(&pkcs8)
                .map_err(|e| DocError::invalid_format(format!("invalid RSA key: {e}")))?
        };
        let modulus_len = key_pair.public().modulus_len();
        Ok(Self {
            key_pair,
            modulus_len,
        })
    }

    /// The public key components `(modulus, exponent)`, for checking that a
    /// certificate's public key belongs to this private key.
    ///
    /// Both are big-endian without leading zeros.
    #[must_use]
    pub fn public_key(&self) -> ring::rsa::PublicKeyComponents<Vec<u8>> {
        ring::rsa::PublicKeyComponents::from(self.key_pair.public())
    }

    /// Sign `data` with RSA-PSS (SHA-256, MGF1-SHA-256, 32-byte salt),
    /// the algorithm used for CMS/CAdES signatures.
    ///
    /// # Errors
    /// Returns [`DocError`] if the signing operation fails.
    pub fn sign_pss_sha256(
        &self,
        data: &[u8],
        rng: &dyn ring::rand::SecureRandom,
    ) -> Result<Vec<u8>, DocError> {
        let mut signature = vec![0u8; self.modulus_len];
        let algorithm = &ring::signature::RSA_PSS_SHA256;
        key_pair_sign(&self.key_pair, algorithm, data, &mut signature, rng)?;
        Ok(signature)
    }

    /// Sign `data` with RSASSA-PKCS1-v1_5 (SHA-256), the algorithm used for
    /// XML-DSig/XAdES `rsa-sha256` signatures.
    ///
    /// # Errors
    /// Returns [`DocError`] if the signing operation fails.
    pub fn sign_pkcs1_sha256(
        &self,
        data: &[u8],
        rng: &dyn ring::rand::SecureRandom,
    ) -> Result<Vec<u8>, DocError> {
        let mut signature = vec![0u8; self.modulus_len];
        let algorithm = &ring::signature::RSA_PKCS1_SHA256;
        key_pair_sign(&self.key_pair, algorithm, data, &mut signature, rng)?;
        Ok(signature)
    }

    /// Signature size in bytes (the modulus length).
    #[must_use]
    pub fn signature_len(&self) -> usize {
        self.modulus_len
    }
}

fn key_pair_sign(
    key_pair: &RsaKeyPair,
    algorithm: &'static dyn ring::signature::RsaEncoding,
    data: &[u8],
    signature: &mut [u8],
    rng: &dyn ring::rand::SecureRandom,
) -> Result<(), DocError> {
    key_pair
        .sign(algorithm, rng, data, signature)
        .map_err(|e| DocError::invalid_format(format!("signing failed: {e}")))
}

/// Wrap a PKCS#1 `RSAPrivateKey` in a PKCS#8 `PrivateKeyInfo`.
///
/// PKCS#8 is `SEQUENCE { version, AlgorithmIdentifier, privateKey OCTET STRING }`
/// where the inner `privateKey` carries the whole PKCS#1 document.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if the DER is not a well-formed PKCS#1
/// RSA private key (wrong tag, truncated, or trailing bytes).
fn pkcs1_to_pkcs8(pkcs1: &[u8]) -> Result<Vec<u8>, DocError> {
    let bad = |why: &str| DocError::invalid_format(format!("not a PKCS#1 RSA private key: {why}"));

    // The outer SEQUENCE must consume the whole document, so trailing bytes are
    // not silently dropped.
    let outer = read_tlv(pkcs1, 0, 0x30).ok_or_else(|| bad("no SEQUENCE"))?;
    if outer.consumed != pkcs1.len() {
        return Err(bad("trailing bytes after the SEQUENCE"));
    }

    // `RSAPrivateKey ::= SEQUENCE { version INTEGER, modulus INTEGER,
    //   publicExponent INTEGER, privateExponent INTEGER, … }`, so the fields
    //   are the SEQUENCE content directly. version is 0 for two-prime and 1
    //   for multi-prime; anything else is not a key `ring` can use.
    let version = read_tlv(outer.content, 0, 0x02).ok_or_else(|| bad("no version INTEGER"))?;
    if version.content != [0x00] && version.content != [0x01] {
        return Err(bad("unsupported RSAPrivateKey version"));
    }
    // The modulus must follow for this to be an RSA key at all.
    read_tlv(outer.content, version.consumed, 0x02).ok_or_else(|| bad("no modulus INTEGER"))?;

    // rsaEncryption with NULL parameters (RFC 3279 §2.3.1).
    let algorithm = crate::der::sequence(
        &[crate::der::oid("1.2.840.113549.1.1.1")?, crate::der::null()].concat(),
    );
    Ok(crate::der::sequence(
        &[
            crate::der::integer(&[0]),
            algorithm,
            crate::der::octet_string(pkcs1),
        ]
        .concat(),
    ))
}

/// One DER TLV: its content octets and total bytes consumed.
struct Tlv<'a> {
    content: &'a [u8],
    consumed: usize,
}

/// Read the TLV at `pos` and require `expected_tag`.
///
/// # Errors
/// Returns `None` if the tag does not match or the length is indefinite or runs
/// past the end of the data.
fn read_tlv(data: &[u8], pos: usize, expected_tag: u8) -> Option<Tlv<'_>> {
    let tag = *data.get(pos)?;
    if tag != expected_tag {
        return None;
    }
    let (content_len, length_bytes) = read_length(data, pos + 1)?;
    let start = pos + 1 + length_bytes;
    let end = start.checked_add(content_len)?;
    if end > data.len() {
        return None;
    }
    Some(Tlv {
        content: &data[start..end],
        consumed: end - pos,
    })
}

/// Read a DER definite-form length, returning `(length, bytes consumed)`.
fn read_length(data: &[u8], pos: usize) -> Option<(usize, usize)> {
    let first = *data.get(pos)?;
    if first & 0x80 == 0 {
        return Some((usize::from(first), 1));
    }
    let count = usize::from(first & 0x7F);
    // 0x80 is the indefinite form, which BER permits and DER does not.
    if count == 0 || count > 4 {
        return None;
    }
    let mut length = 0usize;
    for index in 0..count {
        length = (length << 8) | usize::from(*data.get(pos + 1 + index)?);
    }
    Some((length, 1 + count))
}

/// Decode PEM armor into the DER payload of the first PEM section.
///
/// Public so downstream signers can pass certificate PEM documents through
/// the same decoder used for keys.
///
/// # Errors
/// Returns [`DocError`] if the armor is missing or base64 decoding fails.
pub fn pem_to_der(pem: &[u8]) -> Result<Vec<u8>, DocError> {
    let text =
        std::str::from_utf8(pem).map_err(|_| DocError::invalid_format("PEM is not valid UTF-8"))?;
    let mut in_body = false;
    let mut body = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("-----BEGIN ") {
            in_body = true;
            continue;
        }
        if line.starts_with("-----END ") {
            break;
        }
        if in_body && !line.is_empty() {
            body.push_str(line);
        }
    }
    if body.is_empty() {
        return Err(DocError::invalid_format("PEM armor not found"));
    }
    base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .map_err(|e| DocError::invalid_format(format!("PEM base64 decode failed: {e}")))
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    const TEST_KEY_PEM: &str = include_str!("../tests/fixtures/test_key.pem");
    /// The same key as `TEST_KEY_PEM`, re-encoded by `openssl rsa
    /// -traditional` under PKCS#1 armor.
    const TEST_KEY_PKCS1_PEM: &str = include_str!("../tests/fixtures/test_key_pkcs1.pem");
    /// A different, unrelated RSA key, so a mismatch is a real mismatch.
    const OTHER_KEY_PKCS1_PEM: &str = include_str!("../tests/fixtures/other_key_pkcs1.pem");
    const TEST_CERT_PEM: &str = include_str!("../tests/fixtures/test_cert.pem");

    fn pem_armor(label: &str, der: &[u8]) -> Vec<u8> {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(der);
        let mut out = format!("-----BEGIN {label}-----\n");
        for chunk in encoded.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(chunk).expect("base64 is ASCII"));
            out.push('\n');
        }
        let _ = writeln!(out, "-----END {label}-----");
        out.into_bytes()
    }

    #[test]
    fn loads_pkcs8_rsa_key() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("load key");
        assert_eq!(key.signature_len(), 256); // RSA-2048
    }

    /// `openssl genrsa` emits PKCS#1 by default, so this armor has to load.
    #[test]
    fn loads_pkcs1_rsa_key() {
        let key = PrivateKey::from_pem(TEST_KEY_PKCS1_PEM.as_bytes()).expect("load PKCS#1 key");
        assert_eq!(key.signature_len(), 256);
        // The PKCS#1 fixture is the same key as the PKCS#8 one, so the public
        // components must match exactly.
        let from_pkcs8 = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("load PKCS#8 key");
        assert_eq!(key.public_key().n, from_pkcs8.public_key().n);
        assert_eq!(key.public_key().e, from_pkcs8.public_key().e);
    }

    /// A key that is neither PKCS#8 nor PKCS#1 must be refused, not guessed at.
    #[test]
    fn a_structurally_valid_but_wrong_der_is_rejected() {
        let not_a_key = pem_armor("RSA PRIVATE KEY", &[0x30, 0x03, 0x02, 0x01, 0x07]);
        assert!(PrivateKey::from_pem(&not_a_key).is_err(), "version 7");

        // Truncated SEQUENCE: the length runs past the end.
        let truncated = pem_armor("RSA PRIVATE KEY", &[0x30, 0x10, 0x02, 0x01, 0x00]);
        assert!(PrivateKey::from_pem(&truncated).is_err(), "truncated");

        // Trailing bytes after the SEQUENCE must not be silently dropped.
        let trailing = pem_armor(
            "RSA PRIVATE KEY",
            &[0x30, 0x05, 0x02, 0x01, 0x00, 0x02, 0x01, 0x01, 0xFF],
        );
        assert!(PrivateKey::from_pem(&trailing).is_err(), "trailing bytes");
    }

    #[test]
    fn certificate_parses_and_exposes_its_window() {
        let cert = Certificate::from_pem(TEST_CERT_PEM.as_bytes()).expect("cert");
        assert_eq!(cert.public_key_algorithm(), "rsaEncryption");
        assert!(cert.not_after > cert.not_before, "sane validity window");
    }

    /// The signing key must be the private half of the certificate's public key.
    #[test]
    fn a_matching_key_and_certificate_agree() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("key");
        let cert = Certificate::from_pem(TEST_CERT_PEM.as_bytes()).expect("cert");
        cert.check_key_matches(&key).expect("matching key");
    }

    /// A key that is not the private half of the certificate must be caught,
    /// rather than silently producing a signature no verifier can attribute.
    #[test]
    fn a_mismatched_key_is_rejected() {
        let mismatched = PrivateKey::from_pem(OTHER_KEY_PKCS1_PEM.as_bytes()).expect("other key");
        let cert = Certificate::from_pem(TEST_CERT_PEM.as_bytes()).expect("cert");
        let err = cert
            .check_key_matches(&mismatched)
            .expect_err("mismatched key must be rejected");
        assert!(err.to_string().contains("does not match"), "{err}");
    }

    /// The same check must run inside `sign_detached`, so the mistake cannot be
    /// reintroduced by calling the lower-level API directly.
    #[test]
    fn signing_with_a_mismatched_key_fails() {
        let mismatched = PrivateKey::from_pem(OTHER_KEY_PKCS1_PEM.as_bytes()).expect("other key");
        let cert_der = crate::pem::pem_to_der(TEST_CERT_PEM.as_bytes()).expect("cert der");
        let rng = ring::rand::SystemRandom::new();
        let err = crate::cades::sign_detached(b"data", &mismatched, &cert_der, &rng)
            .expect_err("mismatched key must fail");
        assert!(err.to_string().contains("does not match"), "{err}");
    }

    /// The validity window is inclusive at both ends (RFC 5280 §4.1.2.5).
    #[test]
    fn validity_window_bounds_are_inclusive() {
        let cert = Certificate::from_pem(TEST_CERT_PEM.as_bytes()).expect("cert");
        let before = cert.not_before - 1;
        let after = cert.not_after + 1;

        let err = cert
            .check_validity(before)
            .expect_err("before notBefore must fail");
        assert!(err.to_string().contains("not valid until"), "{err}");
        let err = cert
            .check_validity(after)
            .expect_err("after notAfter must fail");
        assert!(err.to_string().contains("expired"), "{err}");

        assert!(cert.check_validity(cert.not_before).is_ok(), "notBefore");
        assert!(cert.check_validity(cert.not_after).is_ok(), "notAfter");
        assert!(
            cert.check_validity((cert.not_before + cert.not_after) / 2)
                .is_ok(),
            "mid-window"
        );
    }

    #[test]
    fn a_non_certificate_is_rejected() {
        assert!(Certificate::from_der(b"not a certificate").is_err());
    }

    #[test]
    fn rejects_garbage_pem() {
        assert!(PrivateKey::from_pem(b"not a pem").is_err());
        assert!(PrivateKey::from_pem(b"-----BEGIN X-----\n!!!!\n-----END X-----").is_err());
    }

    #[test]
    fn pss_signature_has_modulus_length() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("load key");
        let rng = ring::rand::SystemRandom::new();
        let sig = key.sign_pss_sha256(b"payload", &rng).expect("sign");
        assert_eq!(sig.len(), 256);
    }

    #[test]
    fn pkcs1_signature_is_deterministic() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("load key");
        let rng = ring::rand::SystemRandom::new();
        let a = key.sign_pkcs1_sha256(b"payload", &rng).expect("sign");
        let b = key.sign_pkcs1_sha256(b"payload", &rng).expect("sign");
        assert_eq!(a, b, "PKCS#1 v1.5 signatures are deterministic");
    }
}

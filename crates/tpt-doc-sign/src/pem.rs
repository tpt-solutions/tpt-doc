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

impl PrivateKey {
    /// Load an RSA private key from a PEM document.
    ///
    /// Supports PKCS#8 (`-----BEGIN PRIVATE KEY-----`) RSA keys.
    ///
    /// # Errors
    /// Returns [`DocError`] if the PEM armor is malformed, base64 decoding
    /// fails, or the DER document is not a PKCS#8 RSA key.
    pub fn from_pem(pem: &[u8]) -> Result<Self, DocError> {
        let der = pem_to_der(pem)?;
        let key_pair = RsaKeyPair::from_pkcs8(&der)
            .map_err(|e| DocError::invalid_format(format!("invalid PKCS#8 RSA key: {e}")))?;
        let modulus_len = key_pair.public().modulus_len();
        Ok(Self {
            key_pair,
            modulus_len,
        })
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
    use super::*;

    const TEST_KEY_PEM: &str = include_str!("../tests/fixtures/test_key.pem");

    #[test]
    fn loads_pkcs8_rsa_key() {
        let key = PrivateKey::from_pem(TEST_KEY_PEM.as_bytes()).expect("load key");
        assert_eq!(key.signature_len(), 256); // RSA-2048
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

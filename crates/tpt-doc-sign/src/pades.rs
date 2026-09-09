use tpt_doc_core::DocError;

/// A loaded private key for signing operations.
pub struct PrivateKey {
    // TODO(phase-3): wrap ring::signature::RsaKeyPair
    _inner: (),
}

impl PrivateKey {
    /// Load a private key from PEM bytes.
    ///
    /// # Errors
    /// Returns [`DocError`] if the PEM cannot be parsed or is not an RSA key.
    pub fn from_pem(_pem: &[u8]) -> Result<Self, DocError> {
        // TODO(phase-3): implement PEM → ring::signature::RsaKeyPair
        Err(DocError::invalid_format("PAdES signing not yet implemented"))
    }
}

/// Sign a PDF document with a PAdES-B signature.
///
/// The signature is integrated into the PDF using the `/ByteRange` mechanism
/// so the document remains a valid, openable PDF after signing.
///
/// # Errors
/// Returns [`DocError`] if the PDF is malformed or the signing operation fails.
pub fn sign(_pdf_bytes: &[u8], _key: &PrivateKey) -> Result<Vec<u8>, DocError> {
    // TODO(phase-3): implement ByteRange signing
    Err(DocError::invalid_format("PAdES signing not yet implemented"))
}

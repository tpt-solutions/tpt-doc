use tpt_doc_core::DocError;

/// Create a CAdES detached signature over arbitrary bytes.
///
/// Returns the DER-encoded `SignedData` CMS structure (without the signed
/// content embedded — a detached signature).
///
/// # Errors
/// Returns [`DocError`] if the signing operation fails.
pub fn sign_detached(
    _data: &[u8],
    _key: &crate::pades::PrivateKey,
) -> Result<Vec<u8>, DocError> {
    // TODO(phase-3): implement CMS SignedData via ring SHA-256/RSA-PSS
    Err(DocError::invalid_format("CAdES signing not yet implemented"))
}

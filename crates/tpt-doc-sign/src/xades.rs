use tpt_doc_core::DocError;

/// Sign an XML document with an XAdES enveloped signature.
///
/// Produces a `<ds:Signature>` element embedded in the document's root element.
///
/// # Errors
/// Returns [`DocError`] if the XML is malformed or the signing operation fails.
pub fn sign(_xml_bytes: &[u8], _key: &crate::pades::PrivateKey) -> Result<Vec<u8>, DocError> {
    // TODO(phase-3): implement XAdES enveloped signature
    Err(DocError::invalid_format("XAdES signing not yet implemented"))
}

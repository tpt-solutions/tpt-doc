#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]
//! Pure-Rust cryptographic document signing — `PAdES`, `XAdES`, `CAdES`.
//!
//! Leverages `ring` for cryptographic primitives. No C-FFI.

/// `CAdES` detached signatures (CMS `SignedData`, RSA-PSS).
pub mod cades;
/// Minimal DER (TLV) encoding for CMS structures.
pub mod der;
/// `PAdES` PDF signature integration (`/ByteRange` incremental updates).
pub mod pades;
/// PEM decoding and the RSA [`pem::PrivateKey`](crate::pem::PrivateKey) handle.
pub mod pem;
/// `XAdES` enveloped XML signatures (`rsa-sha256`).
pub mod xades;

pub use pades::sign;
pub use pem::PrivateKey;
pub use xades::sign as sign_xml;

#[cfg(test)]
pub(crate) mod test_support {
    use super::PrivateKey;

    pub fn test_key() -> PrivateKey {
        PrivateKey::from_pem(include_bytes!("../tests/fixtures/test_key.pem")).expect("test key")
    }

    pub fn test_cert_der() -> Vec<u8> {
        crate::pem::pem_to_der(include_bytes!("../tests/fixtures/test_cert.pem"))
            .expect("test cert")
    }
}

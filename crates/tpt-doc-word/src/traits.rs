//! Implementations of the shared `tpt-doc-core` traits for [`DocxDocument`].
//!
//! [`DocxDocument`] is both read and written. The reader rebuilds the model
//! from `word/document.xml` and the header and footer parts.

use tpt_doc_core::{DocError, DocReader, DocWriter};

use crate::{DocxDocument, DocxWriter};

impl DocReader for DocxDocument {
    /// Read a `.docx` package with the default limits; see [`DocxDocument::read`].
    fn from_bytes(bytes: &[u8]) -> Result<Self, DocError> {
        DocxDocument::read(bytes)
    }
}

impl DocWriter for DocxDocument {
    /// Serialize to `.docx` bytes; same as [`DocxWriter::write`].
    fn to_bytes(&self) -> Result<Vec<u8>, DocError> {
        DocxWriter::write(self)
    }
}

//! The shared `tpt-doc-core` writer trait for [`Document`](crate::Document).

use tpt_doc_core::{DocError, DocWriter};

use crate::Document;

impl DocWriter for Document {
    /// Serialize to PDF bytes; same as [`Document::write`].
    fn to_bytes(&self) -> Result<Vec<u8>, DocError> {
        self.write()
    }
}

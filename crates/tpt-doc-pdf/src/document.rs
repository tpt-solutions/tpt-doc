use tpt_doc_core::DocError;

use crate::{Font, Page};

/// A PDF document: the root object that owns pages, fonts, and resources.
pub struct Document {
    pages: Vec<Page>,
    fonts: Vec<Font>,
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            fonts: Vec::new(),
        }
    }

    /// Embed one of the 14 PDF standard (built-in) fonts.
    pub fn embed_builtin_font(&mut self, font: Font) -> usize {
        let id = self.fonts.len();
        self.fonts.push(font);
        id
    }

    /// Add a page to the document.
    pub fn add_page(&mut self, page: Page) {
        self.pages.push(page);
    }

    /// Serialize the document to PDF bytes.
    ///
    /// # Errors
    /// Returns [`DocError`] on serialization failure.
    pub fn write(self) -> Result<Vec<u8>, DocError> {
        // TODO(phase-3): implement full PDF serialization with xref table
        Err(DocError::invalid_format("Document::write not yet implemented"))
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

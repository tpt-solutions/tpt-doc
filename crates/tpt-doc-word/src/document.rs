use tpt_doc_core::DocError;

use crate::{Paragraph, Table};

/// A Word document body element — either a paragraph or a table.
#[derive(Debug, Clone)]
pub enum BodyElement {
    /// A paragraph (may contain multiple [`Run`](crate::Run)s).
    Paragraph(Paragraph),
    /// A table.
    Table(Table),
}

/// An in-memory Word document model.
#[derive(Debug, Default)]
pub struct DocxDocument {
    body: Vec<BodyElement>,
}

impl DocxDocument {
    /// Create an empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a paragraph.
    pub fn push_paragraph(&mut self, p: Paragraph) {
        self.body.push(BodyElement::Paragraph(p));
    }

    /// Append a table.
    pub fn push_table(&mut self, t: Table) {
        self.body.push(BodyElement::Table(t));
    }

    /// Body elements in document order.
    pub fn body(&self) -> &[BodyElement] {
        &self.body
    }
}

/// Serializes a [`DocxDocument`] to `.docx` bytes.
pub struct DocxWriter;

impl DocxWriter {
    /// Serialize the document to `.docx` bytes (ZIP + OOXML XML parts).
    ///
    /// # Errors
    /// Returns [`DocError`] if ZIP assembly or XML serialization fails.
    pub fn write(_doc: &DocxDocument) -> Result<Vec<u8>, DocError> {
        // TODO(phase-1.5): implement OOXML ZIP assembly
        // Required parts:
        //   [Content_Types].xml
        //   _rels/.rels
        //   word/document.xml
        //   word/_rels/document.xml.rels
        //   word/styles.xml
        //   word/settings.xml
        Err(DocError::invalid_format("DocxWriter not yet implemented"))
    }
}

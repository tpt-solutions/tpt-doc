use tpt_doc_core::DocError;

/// Headless HTML/CSS-to-PDF renderer.
pub struct Renderer {
    // TODO(phase-3): add configuration: page size, margins, base font size
}

impl Renderer {
    /// Create a new renderer with A4 page defaults.
    pub fn new() -> Self {
        Self {}
    }

    /// Render an HTML string to PDF bytes.
    ///
    /// Supports the restricted HTML/CSS subset documented in the crate README.
    ///
    /// # Errors
    /// Returns [`DocError`] if the HTML cannot be parsed or the PDF cannot be
    /// generated.
    pub fn render_html(&self, _html: &str) -> Result<Vec<u8>, DocError> {
        // TODO(phase-3): parse HTML → layout tree → tpt_doc_pdf::Document
        Err(DocError::invalid_format("Renderer::render_html not yet implemented"))
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

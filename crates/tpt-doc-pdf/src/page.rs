/// A single PDF page with a content stream.
pub struct Page {
    /// Page width in points (1 pt = 1/72 inch).
    pub width: f32,
    /// Page height in points.
    pub height: f32,
    operations: Vec<ContentOp>,
}

/// A PDF content stream operation.
#[derive(Debug, Clone)]
pub(crate) enum ContentOp {
    Text {
        content: String,
        font_id: usize,
        size: f32,
        x: f32,
        y: f32,
    },
}

impl Page {
    /// A4 page (595 × 842 pt).
    pub fn a4() -> Self {
        Self {
            width: 595.0,
            height: 842.0,
            operations: Vec::new(),
        }
    }

    /// US Letter page (612 × 792 pt).
    pub fn letter() -> Self {
        Self {
            width: 612.0,
            height: 792.0,
            operations: Vec::new(),
        }
    }

    /// Add a text string at the given position.
    ///
    /// `font_id` is the index returned by [`Document::embed_builtin_font`].
    /// `size` is in points. `position` is `(x, y)` in points from the
    /// bottom-left corner of the page.
    pub fn text(&mut self, content: impl Into<String>, font_id: usize, size: f32, position: (f32, f32)) {
        self.operations.push(ContentOp::Text {
            content: content.into(),
            font_id,
            size,
            x: position.0,
            y: position.1,
        });
    }

    /// The raw content operations (used by the serializer).
    pub(crate) fn operations(&self) -> &[ContentOp] {
        &self.operations
    }
}

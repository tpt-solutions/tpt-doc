/// A single PDF page with a content stream.
#[derive(Debug, Clone)]
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
    /// Draw an embedded image `XObject` at a position and size.
    Image {
        image_id: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Stroke a rectangle outline (used for table borders).
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

impl Page {
    /// A4 page (595 × 842 pt).
    #[must_use]
    pub fn a4() -> Self {
        Self {
            width: 595.0,
            height: 842.0,
            operations: Vec::new(),
        }
    }

    /// US Letter page (612 × 792 pt).
    #[must_use]
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
    pub fn text(
        &mut self,
        content: impl Into<String>,
        font_id: usize,
        size: f32,
        position: (f32, f32),
    ) {
        self.operations.push(ContentOp::Text {
            content: content.into(),
            font_id,
            size,
            x: position.0,
            y: position.1,
        });
    }

    /// Draw an embedded image (`image_id` from `Document::embed_image`) in a
    /// box of the given size.
    pub fn image(&mut self, image_id: usize, x: f32, y: f32, width: f32, height: f32) {
        self.operations.push(ContentOp::Image {
            image_id,
            x,
            y,
            width,
            height,
        });
    }

    /// Stroke a rectangle outline at the given position and size (used for
    /// table cell borders).
    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.operations.push(ContentOp::Rect {
            x,
            y,
            width,
            height,
        });
    }

    /// The raw content operations (used by the serializer).
    pub(crate) fn operations(&self) -> &[ContentOp] {
        &self.operations
    }

    /// The fonts referenced by this page's operations.
    pub(crate) fn used_fonts(&self) -> Vec<usize> {
        let mut fonts = Vec::new();
        for op in &self.operations {
            if let ContentOp::Text { font_id, .. } = op
                && !fonts.contains(font_id)
            {
                fonts.push(*font_id);
            }
        }
        fonts
    }

    /// The images referenced by this page's operations, in resource order.
    pub(crate) fn used_images(&self) -> Vec<usize> {
        let mut images = Vec::new();
        for op in &self.operations {
            if let ContentOp::Image { image_id, .. } = op
                && !images.contains(image_id)
            {
                images.push(*image_id);
            }
        }
        images
    }
}

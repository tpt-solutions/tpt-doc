/// Parsed CSS properties relevant to the layout engine.
#[derive(Debug, Clone, Default)]
pub struct ComputedStyle {
    /// Font size in points.
    pub font_size_pt: Option<f32>,
    /// Font weight: 400 = normal, 700 = bold.
    pub font_weight: Option<u16>,
    /// Margin in points (top, right, bottom, left).
    pub margin: Option<[f32; 4]>,
    /// Padding in points (top, right, bottom, left).
    pub padding: Option<[f32; 4]>,
    /// Text alignment.
    pub text_align: Option<TextAlign>,
    /// Page break before this element.
    pub page_break_before: bool,
    /// Page break after this element.
    pub page_break_after: bool,
}

/// CSS `text-align` values supported by the layout engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// Left-aligned.
    Left,
    /// Right-aligned.
    Right,
    /// Centered.
    Center,
    /// Justified.
    Justify,
}

/// Parse inline CSS style declarations.
///
/// Returns a [`ComputedStyle`] with only the properties that were explicitly set.
pub fn parse_inline(_style: &str) -> ComputedStyle {
    // TODO(phase-3): implement CSS property parser
    ComputedStyle::default()
}

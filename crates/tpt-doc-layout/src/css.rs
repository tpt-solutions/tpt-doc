/// Parsed CSS properties relevant to the layout engine.
#[derive(Debug, Clone, Default, PartialEq)]
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

/// Parse inline CSS style declarations (`font-size: 14pt; text-align: center`).
///
/// Returns a [`ComputedStyle`] with only the properties that were explicitly
/// set. Unknown properties and malformed values are ignored; lengths accept
/// `pt` and `px` (1 px = 0.75 pt).
#[must_use]
pub fn parse_inline(style: &str) -> ComputedStyle {
    let mut computed = ComputedStyle::default();
    for declaration in style.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        match property.as_str() {
            "font-size" => {
                computed.font_size_pt = parse_length_pt(value);
            }
            "font-weight" => match value.to_ascii_lowercase().as_str() {
                "bold" | "bolder" => computed.font_weight = Some(700),
                "normal" | "lighter" => computed.font_weight = Some(400),
                numeric => {
                    if let Ok(weight) = numeric.parse::<u16>() {
                        computed.font_weight = Some(weight);
                    }
                }
            },
            "text-align" => {
                computed.text_align = match value.to_ascii_lowercase().as_str() {
                    "left" => Some(TextAlign::Left),
                    "right" => Some(TextAlign::Right),
                    "center" => Some(TextAlign::Center),
                    "justify" => Some(TextAlign::Justify),
                    _ => None,
                };
            }
            "margin" => {
                computed.margin = parse_box(value);
            }
            "padding" => {
                computed.padding = parse_box(value);
            }
            "page-break-before" => {
                computed.page_break_before = value.eq_ignore_ascii_case("always");
            }
            "page-break-after" => {
                computed.page_break_after = value.eq_ignore_ascii_case("always");
            }
            _ => {}
        }
    }
    computed
}

/// Parse a CSS length in `pt` or `px` into points.
fn parse_length_pt(value: &str) -> Option<f32> {
    let lowered = value.trim().to_ascii_lowercase();
    let split_at = lowered
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(lowered.len());
    let (number, unit) = lowered.split_at(split_at);
    let amount: f32 = number.trim().parse().ok()?;
    match unit {
        "pt" => Some(amount),
        "px" => Some(amount * 0.75),
        _ => None,
    }
}

/// Parse a CSS box (margin/padding): 1–4 space-separated lengths, applied
/// with the standard CSS shorthand expansion.
fn parse_box(value: &str) -> Option<[f32; 4]> {
    let parts: Vec<Option<f32>> = value.split_whitespace().map(parse_length_pt).collect();
    match parts.as_slice() {
        [Some(v)] => Some([*v, *v, *v, *v]),
        [Some(v), Some(h)] => Some([*v, *h, *v, *h]),
        [Some(top), Some(h), Some(bottom)] => Some([*top, *h, *bottom, *h]),
        [Some(top), Some(right), Some(bottom), Some(left)] => Some([*top, *right, *bottom, *left]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_font_size_in_pt_and_px() {
        assert_eq!(parse_inline("font-size: 14pt").font_size_pt, Some(14.0));
        assert_eq!(parse_inline("font-size:16px").font_size_pt, Some(12.0));
    }

    #[test]
    fn parses_font_weight_and_alignment() {
        let style = parse_inline("font-weight: bold; text-align: center");
        assert_eq!(style.font_weight, Some(700));
        assert_eq!(style.text_align, Some(TextAlign::Center));
        assert_eq!(parse_inline("font-weight: 700").font_weight, Some(700));
    }

    #[test]
    fn parses_margin_shorthands() {
        assert_eq!(
            parse_inline("margin: 10pt").margin,
            Some([10.0, 10.0, 10.0, 10.0])
        );
        assert_eq!(
            parse_inline("margin: 10pt 20pt").margin,
            Some([10.0, 20.0, 10.0, 20.0])
        );
        assert_eq!(
            parse_inline("margin: 1pt 2pt 3pt 4pt").margin,
            Some([1.0, 2.0, 3.0, 4.0])
        );
        assert_eq!(parse_inline("margin: bogus").margin, None);
    }

    #[test]
    fn parses_page_breaks() {
        assert!(parse_inline("page-break-before: always").page_break_before);
        assert!(!parse_inline("page-break-before: auto").page_break_before);
        assert!(parse_inline("page-break-after: always").page_break_after);
    }

    #[test]
    fn unknown_properties_are_ignored() {
        let style = parse_inline("color: red; transform: rotate(0deg); font-size: 11pt");
        assert_eq!(
            style,
            ComputedStyle {
                font_size_pt: Some(11.0),
                ..ComputedStyle::default()
            }
        );
    }

    #[test]
    fn empty_and_malformed_input_yield_defaults() {
        assert_eq!(parse_inline(""), ComputedStyle::default());
        assert_eq!(parse_inline(";;:;x"), ComputedStyle::default());
    }
}

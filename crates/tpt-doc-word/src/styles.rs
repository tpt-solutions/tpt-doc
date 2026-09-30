/// Built-in paragraph styles that `word/styles.xml` must define.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuiltinStyle {
    /// Default body text.
    Normal,
    /// Top-level heading.
    Heading1,
    /// Second-level heading.
    Heading2,
    /// Third-level heading.
    Heading3,
    /// Fourth-level heading.
    Heading4,
    /// Fifth-level heading.
    Heading5,
    /// Sixth-level heading.
    Heading6,
    /// Document title.
    Title,
}

impl BuiltinStyle {
    /// The `w:styleId` used in `word/document.xml` and `word/styles.xml`.
    #[must_use]
    pub fn style_id(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Heading1 => "Heading1",
            Self::Heading2 => "Heading2",
            Self::Heading3 => "Heading3",
            Self::Heading4 => "Heading4",
            Self::Heading5 => "Heading5",
            Self::Heading6 => "Heading6",
            Self::Title => "Title",
        }
    }

    /// The human-readable style name Word displays (`w:name val`).
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Heading1 => "heading 1",
            Self::Heading2 => "heading 2",
            Self::Heading3 => "heading 3",
            Self::Heading4 => "heading 4",
            Self::Heading5 => "heading 5",
            Self::Heading6 => "heading 6",
            Self::Title => "Title",
        }
    }

    /// Look up a built-in style by its `w:styleId` string.
    ///
    /// Returns `None` for custom style IDs defined outside this enum.
    #[must_use]
    pub fn from_style_id(style_id: &str) -> Option<Self> {
        match style_id {
            "Normal" => Some(Self::Normal),
            "Heading1" => Some(Self::Heading1),
            "Heading2" => Some(Self::Heading2),
            "Heading3" => Some(Self::Heading3),
            "Heading4" => Some(Self::Heading4),
            "Heading5" => Some(Self::Heading5),
            "Heading6" => Some(Self::Heading6),
            "Title" => Some(Self::Title),
            _ => None,
        }
    }
}

/// The set of styles to emit into `word/styles.xml`.
///
/// `Normal` is always present: OOXML requires it as the default paragraph
/// style. Table borders use the fixed `TableGrid` style ID.
#[derive(Debug, Clone, Default)]
pub struct StyleSet {
    styles: Vec<BuiltinStyle>,
}

impl StyleSet {
    /// Create a style set containing only `Normal`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            styles: vec![BuiltinStyle::Normal],
        }
    }

    /// Record a style as used. Returns `true` if it was newly added.
    pub fn mark_used(&mut self, style: BuiltinStyle) -> bool {
        if self.styles.contains(&style) {
            return false;
        }
        self.styles.push(style);
        true
    }

    /// Styles to serialize, always starting with `Normal`.
    pub fn iter(&self) -> impl Iterator<Item = BuiltinStyle> + '_ {
        self.styles.iter().copied()
    }

    /// The fixed style ID applied to tables with borders.
    pub const TABLE_GRID: &'static str = "TableGrid";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_ids_match_builtin_names() {
        assert_eq!(BuiltinStyle::Normal.style_id(), "Normal");
        assert_eq!(BuiltinStyle::Heading1.style_id(), "Heading1");
        assert_eq!(BuiltinStyle::Heading1.name(), "heading 1");
    }

    #[test]
    fn new_set_contains_only_normal() {
        let set = StyleSet::new();
        let ids: Vec<_> = set.iter().map(|s| s.style_id()).collect();
        assert_eq!(ids, ["Normal"]);
    }

    #[test]
    fn mark_used_is_idempotent() {
        let mut set = StyleSet::new();
        assert!(set.mark_used(BuiltinStyle::Heading1));
        assert!(!set.mark_used(BuiltinStyle::Heading1));
        let ids: Vec<_> = set.iter().map(|s| s.style_id()).collect();
        assert_eq!(ids, ["Normal", "Heading1"]);
    }
}

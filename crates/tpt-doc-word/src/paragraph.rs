/// A run of text with uniform character formatting.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// The run's text content.
    pub text: String,
    /// Character formatting applied to the run.
    pub style: RunStyle,
}

impl Run {
    /// Create a run with default (unformatted) style.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: RunStyle::default(),
        }
    }

    /// Create a run with the given style.
    pub fn styled(text: impl Into<String>, style: RunStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

/// Character-level formatting for a [`Run`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStyle {
    /// Render in a bold typeface.
    pub bold: bool,
    /// Render in an italic typeface.
    pub italic: bool,
    /// Render with a single underline.
    pub underline: bool,
    /// Font size in half-points (e.g. `24` = 12pt).
    pub font_size_half_points: Option<u16>,
}

impl RunStyle {
    /// Enable bold formatting.
    #[must_use]
    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Enable italic formatting.
    #[must_use]
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Enable underline formatting.
    #[must_use]
    pub const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Set the font size in half-points (e.g. `24` = 12pt).
    #[must_use]
    pub const fn font_size_half_points(mut self, half_points: u16) -> Self {
        self.font_size_half_points = Some(half_points);
        self
    }
}

/// A paragraph: a named style plus an ordered sequence of [`Run`]s.
#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    /// Style ID referencing `word/styles.xml` (e.g. `"Heading1"`).
    pub style_id: Option<String>,
    runs: Vec<Run>,
}

impl Paragraph {
    /// Create a Normal-style paragraph containing a single run.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            style_id: None,
            runs: vec![Run::new(text)],
        }
    }

    /// Create a paragraph with the given style ID and a single run.
    pub fn styled(style_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            style_id: Some(style_id.into()),
            runs: vec![Run::new(text)],
        }
    }

    /// Append a run to the paragraph.
    pub fn push_run(&mut self, run: Run) {
        self.runs.push(run);
    }

    /// The paragraph's runs, in order.
    #[must_use]
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    /// Concatenated text of all runs, with no formatting.
    #[must_use]
    pub fn plain_text(&self) -> String {
        let mut text = String::new();
        for run in &self.runs {
            text.push_str(&run.text);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_paragraph_has_single_run() {
        let p = Paragraph::new("Hello, world!");
        assert_eq!(p.runs().len(), 1);
        assert_eq!(p.plain_text(), "Hello, world!");
        assert!(p.style_id.is_none());
    }

    #[test]
    fn styled_paragraph_carries_style_id() {
        let p = Paragraph::styled("Heading1", "Section 1");
        assert_eq!(p.style_id.as_deref(), Some("Heading1"));
        assert_eq!(p.plain_text(), "Section 1");
    }

    #[test]
    fn mixed_runs_concatenate_in_order() {
        let mut p = Paragraph::new("plain ");
        p.push_run(Run::styled("bold", RunStyle::default().bold()));
        p.push_run(Run::new(" tail"));
        assert_eq!(p.plain_text(), "plain bold tail");
    }

    #[test]
    fn run_style_builders_set_flags() {
        let style = RunStyle::default()
            .bold()
            .italic()
            .underline()
            .font_size_half_points(28);
        assert!(style.bold);
        assert!(style.italic);
        assert!(style.underline);
        assert_eq!(style.font_size_half_points, Some(28));
    }
}

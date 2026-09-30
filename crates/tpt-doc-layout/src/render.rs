//! Headless HTML/CSS-to-PDF rendering: block layout with line boxes, page
//! breaks, and fixed-width tables.
//!
//! Text measurement uses a deterministic per-character approximation of the
//! Helvetica metrics — adequate for wrapping report-style content, with
//! byte-identical output for identical input (no font subsystem, no
//! randomness, no time).
//!
//! Font ids are stable by construction: the engine lays text out as one of
//! the four Helvetica faces and [`finish`] embeds exactly those faces in the
//! same order, so the ids used in page operations match the final document.

use tpt_doc_core::DocError;
use tpt_doc_pdf::{Document, Font, Page};

use crate::css::TextAlign;
use crate::html::{self, BlockElement, HtmlNode, TableRow};

/// Target page size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    /// A4 (595 × 842 pt).
    A4,
    /// US Letter (612 × 792 pt).
    Letter,
}

/// Headless HTML/CSS-to-PDF renderer with configurable page geometry.
#[derive(Debug, Clone)]
pub struct Renderer {
    page_kind: PageKind,
    margin_pt: f32,
    base_font_size_pt: f32,
}

/// Approximate advance width of a character as a fraction of the font size,
/// tuned for Helvetica-style faces.
fn approx_char_width(c: char) -> f32 {
    match c {
        ' ' | '.' | ',' | ':' | ';' | '\'' | '|' | '!' | '(' | ')' | '[' | ']' => 0.30,
        'i' | 'j' | 'l' | 't' | 'f' | 'I' | '-' => 0.33,
        'm' | 'w' | 'M' | 'W' | '@' => 0.88,
        c if c.is_ascii_uppercase() => 0.70,
        c if c.is_ascii_digit() => 0.556,
        _ => 0.52,
    }
}

/// Approximate rendered width of `text` at `size` points.
#[must_use]
pub fn approx_text_width(text: &str, size: f32) -> f32 {
    text.chars().map(approx_char_width).sum::<f32>() * size
}

/// The four built-in faces, in embedding order.
const FACES: [Font; 4] = [
    Font::Helvetica,
    Font::HelveticaBold,
    Font::HelveticaOblique,
    Font::HelveticaBoldOblique,
];

fn face_id(bold: bool, italic: bool) -> usize {
    match (bold, italic) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    }
}

fn new_page(renderer: &Renderer) -> Page {
    match renderer.page_kind {
        PageKind::A4 => Page::a4(),
        PageKind::Letter => Page::letter(),
    }
}

fn page_dimensions(renderer: &Renderer) -> (f32, f32) {
    match renderer.page_kind {
        PageKind::A4 => (595.0, 842.0),
        PageKind::Letter => (612.0, 792.0),
    }
}

impl Renderer {
    /// Create a new renderer with A4 defaults (72 pt margins, 11 pt body).
    #[must_use]
    pub fn new() -> Self {
        Self {
            page_kind: PageKind::A4,
            margin_pt: 72.0,
            base_font_size_pt: 11.0,
        }
    }

    /// Target US Letter pages instead of A4.
    #[must_use]
    pub fn letter(mut self) -> Self {
        self.page_kind = PageKind::Letter;
        self
    }

    /// Set the page margin in points.
    #[must_use]
    pub fn with_margin(mut self, margin_pt: f32) -> Self {
        self.margin_pt = margin_pt;
        self
    }

    /// Set the base body font size in points.
    #[must_use]
    pub fn with_base_font_size(mut self, size_pt: f32) -> Self {
        self.base_font_size_pt = size_pt;
        self
    }

    /// Render an HTML string to PDF bytes.
    ///
    /// Supports the restricted HTML/CSS subset documented in the crate docs:
    /// block elements (`p`, `div`, `h1`–`h6`, tables), inline bold/italic,
    /// and the inline CSS properties parsed by
    /// [`parse_inline`](crate::css::parse_inline) — including
    /// `page-break-before`/`after`.
    ///
    /// # Errors
    /// Returns [`DocError`] if the PDF cannot be generated.
    pub fn render_html(&self, html: &str) -> Result<Vec<u8>, DocError> {
        let blocks = html::parse(html);
        let mut engine = Engine::new(self);
        engine.layout_blocks(&blocks);
        engine.finish()
    }

    /// Render an HTML string to laid-out pages without serializing a PDF.
    #[must_use]
    pub fn render_pages(&self, html: &str) -> Vec<Page> {
        let blocks = html::parse(html);
        let mut engine = Engine::new(self);
        engine.layout_blocks(&blocks);
        engine.pages
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Mutable layout state: the page list plus the current drawing position.
struct Engine<'r> {
    renderer: &'r Renderer,
    pages: Vec<Page>,
    y: f32,
    x0: f32,
    content_width: f32,
    top: f32,
    bottom: f32,
}

impl<'r> Engine<'r> {
    fn new(renderer: &'r Renderer) -> Self {
        let (_, page_height) = page_dimensions(renderer);
        let margin = renderer.margin_pt;
        Self {
            pages: vec![new_page(renderer)],
            y: page_height - margin,
            x0: margin,
            content_width: page_width(renderer) - 2.0 * margin,
            top: page_height - margin,
            bottom: margin,
            renderer,
        }
    }

    fn current(&mut self) -> &mut Page {
        self.pages.last_mut().expect("at least one page")
    }

    fn start_page(&mut self) {
        self.pages.push(new_page(self.renderer));
        self.y = self.top;
    }

    /// Ensure room for `needed` points; start a new page if required.
    fn ensure_room(&mut self, needed: f32) {
        if self.y - needed < self.bottom {
            self.start_page();
        }
    }

    fn layout_blocks(&mut self, blocks: &[HtmlNode]) {
        for block in blocks {
            self.layout_block(block);
        }
    }

    fn layout_block(&mut self, block: &HtmlNode) {
        match block {
            HtmlNode::Text { .. } => {}
            HtmlNode::Block(BlockElement::Paragraph { children, style }) => {
                if style.page_break_before {
                    self.start_page();
                }
                let size = style
                    .font_size_pt
                    .unwrap_or(self.renderer.base_font_size_pt);
                self.draw_wrapped(children, size, style.text_align, self.content_width);
                self.y -= 6.0;
                if style.page_break_after {
                    self.start_page();
                }
            }
            HtmlNode::Block(BlockElement::Heading {
                level,
                children,
                style,
            }) => {
                if style.page_break_before {
                    self.start_page();
                }
                let sizes = [24.0, 18.0, 14.0, 12.0, 11.0, 10.0];
                let size = style
                    .font_size_pt
                    .unwrap_or_else(|| sizes[(*level as usize).clamp(1, 6) - 1]);
                self.y -= 8.0;
                self.ensure_room(size * 1.5);
                self.draw_wrapped(children, size, style.text_align, self.content_width);
                self.y -= 4.0;
                if style.page_break_after {
                    self.start_page();
                }
            }
            HtmlNode::Block(BlockElement::Table { rows, style }) => {
                if style.page_break_before {
                    self.start_page();
                }
                self.draw_table(rows);
                self.y -= 10.0;
                if style.page_break_after {
                    self.start_page();
                }
            }
        }
    }

    /// Word-wrap inline content into `available_width` and draw each line
    /// box at the cursor. Advances the cursor past the text block.
    fn draw_wrapped(
        &mut self,
        children: &[HtmlNode],
        size: f32,
        align: Option<TextAlign>,
        available_width: f32,
    ) {
        // Break inline content into styled words; '\n' in text content
        // (from explicit line structure) forces a break.
        let mut words: Vec<(String, bool, bool)> = Vec::new();
        for node in children {
            if let HtmlNode::Text {
                content,
                bold,
                italic,
            } = node
            {
                for line in content.split('\n') {
                    for word in line.split(' ') {
                        if !word.is_empty() {
                            words.push((word.to_owned(), *bold, *italic));
                        }
                    }
                    words.push((String::from("\n"), *bold, *italic));
                }
                words.pop(); // drop the trailing break marker
            }
        }

        let line_height = size * 1.35;
        let space_width = approx_text_width(" ", size);
        let mut line: Vec<(String, bool, bool)> = Vec::new();
        let mut line_width = 0.0f32;

        for (word, bold, italic) in &words {
            if word == "\n" {
                self.flush_line(&line, line_width, size, align, available_width, line_height);
                line.clear();
                line_width = 0.0;
                continue;
            }
            let word_width = approx_text_width(word, size);
            let new_width = if line.is_empty() {
                word_width
            } else {
                line_width + space_width + word_width
            };
            if !line.is_empty() && new_width > available_width {
                self.flush_line(&line, line_width, size, align, available_width, line_height);
                line.clear();
                line_width = word_width;
            } else {
                line_width = new_width;
            }
            line.push((word.clone(), *bold, *italic));
        }
        if !line.is_empty() {
            self.flush_line(&line, line_width, size, align, available_width, line_height);
        }
    }

    fn flush_line(
        &mut self,
        line: &[(String, bool, bool)],
        line_width: f32,
        size: f32,
        align: Option<TextAlign>,
        available_width: f32,
        line_height: f32,
    ) {
        self.ensure_room(line_height);
        self.y -= line_height;
        let offset = match align.unwrap_or(TextAlign::Left) {
            TextAlign::Left | TextAlign::Justify => 0.0,
            TextAlign::Center => ((available_width - line_width) / 2.0).max(0.0),
            TextAlign::Right => (available_width - line_width).max(0.0),
        };
        let space_width = approx_text_width(" ", size);
        let baseline = self.y;
        let mut pen_x = self.x0 + offset;
        for (word, bold, italic) in line {
            self.current()
                .text(word, face_id(*bold, *italic), size, (pen_x, baseline));
            pen_x += approx_text_width(word, size) + space_width;
        }
    }

    /// Draw a table with fixed equal-width columns and stroked cell borders.
    fn draw_table(&mut self, rows: &[TableRow]) {
        let cols = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
        if cols == 0 {
            return;
        }
        let padding = 3.0;
        let size = self.renderer.base_font_size_pt;
        let line_height = size * 1.35;
        let char_width = 0.52 * size;
        #[allow(clippy::cast_precision_loss)] // column counts are tiny
        let col_width = self.content_width / cols as f32;

        for row in rows {
            let row_height = row
                .cells
                .iter()
                .map(|cell| {
                    let text_len: f32 = cell
                        .iter()
                        .map(|n| match n {
                            HtmlNode::Text { content, .. } => {
                                #[allow(clippy::cast_precision_loss)]
                                let len = content.len() as f32;
                                len
                            }
                            HtmlNode::Block(_) => 0.0,
                        })
                        .sum();
                    let usable = (col_width - 2.0 * padding).max(char_width);
                    let lines = ((text_len * char_width) / usable).ceil().max(1.0);
                    lines * line_height + 2.0 * padding
                })
                .fold(line_height + 2.0 * padding, f32::max);
            self.ensure_room(row_height);
            let top = self.y;
            let bottom = top - row_height;

            for (index, cell) in row.cells.iter().enumerate() {
                #[allow(clippy::cast_precision_loss)]
                let index_f = index as f32;
                let x = self.x0 + col_width * index_f;
                self.current().rect(x, bottom, col_width, row_height);
                let text: String = cell
                    .iter()
                    .filter_map(|n| match n {
                        HtmlNode::Text { content, .. } => Some(content.clone()),
                        HtmlNode::Block(_) => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if !text.is_empty() {
                    let bold = matches!(cell.first(), Some(HtmlNode::Text { bold: true, .. }));
                    self.current().text(
                        &text,
                        face_id(bold, false),
                        size,
                        (x + padding, top - padding - size),
                    );
                }
            }
            self.y = bottom;
        }
    }

    fn finish(self) -> Result<Vec<u8>, DocError> {
        self.into_document().write()
    }

    fn into_document(self) -> Document {
        let mut doc = Document::new();
        for font in FACES {
            doc.embed_builtin_font(font);
        }
        for page in self.pages {
            doc.add_page(page);
        }
        doc
    }
}

fn page_width(renderer: &Renderer) -> f32 {
    page_dimensions(renderer).0
}

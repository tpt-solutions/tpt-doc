//! Minimal HTML document model and parsing.
//!
//! The parser accepts the restricted block-oriented HTML subset the layout
//! engine renders: `p`, `div`, `h1`–`h6`, `table`/`tr`/`td`/`th`, and the
//! inline formatters `b`/`strong`/`i`/`em`. `html`/`head`/`body` wrappers are
//! transparent, unknown tags are ignored (their children still parse), and
//! malformed input never panics — unclosed elements are closed implicitly.

use crate::css::ComputedStyle;

/// A node in the parsed HTML tree.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlNode {
    /// A block-level element.
    Block(BlockElement),
    /// Inline text content with character formatting.
    Text {
        /// The text content (entities decoded, whitespace collapsed).
        content: String,
        /// Rendered in a bold typeface.
        bold: bool,
        /// Rendered in an italic typeface.
        italic: bool,
    },
}

/// Block-level HTML elements supported by the layout engine.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockElement {
    /// `<div>` or `<p>`, with inline child nodes.
    Paragraph {
        /// The element's child nodes.
        children: Vec<HtmlNode>,
        /// Parsed `style` attribute.
        style: ComputedStyle,
    },
    /// `<h1>` through `<h6>`.
    Heading {
        /// Heading level, 1–6.
        level: u8,
        /// The element's child nodes.
        children: Vec<HtmlNode>,
        /// Parsed `style` attribute.
        style: ComputedStyle,
    },
    /// `<table>`.
    Table {
        /// The table's rows.
        rows: Vec<TableRow>,
        /// Parsed `style` attribute.
        style: ComputedStyle,
    },
}

/// A `<tr>` row in a table.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    /// `<td>` or `<th>` cells; `th` cells render bold.
    pub cells: Vec<Vec<HtmlNode>>,
}

/// Parse an HTML fragment into block nodes.
///
/// Parsing is lenient: mismatched or unknown tags are tolerated, and the
/// function always returns successfully.
#[must_use]
pub fn parse(html: &str) -> Vec<HtmlNode> {
    Builder::run(html)
}

/// One open element on the builder stack.
struct Frame {
    kind: FrameKind,
    children: Vec<HtmlNode>,
    style: ComputedStyle,
    /// Cells collected while a `<tr>` frame is open.
    cells: Vec<Vec<HtmlNode>>,
    /// Rows collected while a `<table>` frame is open.
    rows: Vec<TableRow>,
}

enum FrameKind {
    Root,
    Paragraph,
    Heading(u8),
    Table,
    Row,
    Cell(bool),
}

struct Builder {
    chars: Vec<char>,
    pos: usize,
    stack: Vec<Frame>,
    bold: usize,
    italic: usize,
    /// Depth of non-content contexts (`head`, `title`, `script`, `style`);
    /// text inside them is discarded.
    suppressed: usize,
    text: String,
}

impl Builder {
    fn run(html: &str) -> Vec<HtmlNode> {
        let mut builder = Builder {
            chars: html.chars().collect(),
            pos: 0,
            stack: vec![Frame {
                kind: FrameKind::Root,
                children: Vec::new(),
                style: ComputedStyle::default(),
                cells: Vec::new(),
                rows: Vec::new(),
            }],
            bold: 0,
            italic: 0,
            suppressed: 0,
            text: String::new(),
        };
        builder.build();
        match builder.stack.pop() {
            Some(root) => root.children,
            None => Vec::new(),
        }
    }

    fn build(&mut self) {
        while let Some(token) = self.next_token() {
            match token {
                Token::Text(raw) => {
                    self.text.push_str(&raw);
                }
                Token::Tag {
                    name,
                    closing,
                    style,
                } => {
                    self.handle_tag(&name, closing, style);
                }
            }
        }
        self.flush_text();
        while self.stack.len() > 1 {
            self.close_top();
        }
    }

    /// Tokenize the next tag or text run.
    fn next_token(&mut self) -> Option<Token> {
        if self.pos >= self.chars.len() {
            return None;
        }
        if self.chars[self.pos] == '<' {
            let mut raw = String::new();
            self.pos += 1;
            while let Some(&c) = self.chars.get(self.pos) {
                if c == '>' {
                    self.pos += 1;
                    break;
                }
                raw.push(c);
                self.pos += 1;
            }
            return Some(Self::lex_tag(&raw));
        }
        let mut raw = String::new();
        while let Some(&c) = self.chars.get(self.pos) {
            if c == '<' {
                break;
            }
            raw.push(c);
            self.pos += 1;
        }
        Some(Token::Text(raw))
    }

    fn lex_tag(raw: &str) -> Token {
        let raw = raw.trim();
        let closing = raw.starts_with('/');
        let body = raw.strip_prefix('/').unwrap_or(raw);
        let name_end = body
            .find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(body.len());
        let name = body[..name_end].to_ascii_lowercase();
        let attrs = body[name_end..].to_owned();
        let style = find_attr(&attrs, "style")
            .map(|value| crate::css::parse_inline(&value))
            .unwrap_or_default();
        Token::Tag {
            name,
            closing,
            style,
        }
    }

    fn handle_tag(&mut self, name: &str, closing: bool, style: ComputedStyle) {
        match (closing, name) {
            (false, "head" | "title" | "script" | "style") => {
                self.flush_text();
                self.suppressed += 1;
            }
            (true, "head" | "title" | "script" | "style") => {
                self.flush_text();
                self.suppressed = self.suppressed.saturating_sub(1);
            }
            (false, "b" | "strong") => {
                self.flush_text();
                self.bold += 1;
            }
            (true, "b" | "strong") => {
                self.flush_text();
                self.bold = self.bold.saturating_sub(1);
            }
            (false, "i" | "em") => {
                self.flush_text();
                self.italic += 1;
            }
            (true, "i" | "em") => {
                self.flush_text();
                self.italic = self.italic.saturating_sub(1);
            }
            (false, "p" | "div") => {
                self.flush_text();
                self.open(FrameKind::Paragraph, style);
            }
            (true, "p" | "div") => {
                self.flush_text();
                self.close_named(|kind| matches!(kind, FrameKind::Paragraph));
            }
            (false, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") => {
                self.flush_text();
                let level = name[1..].parse::<u8>().unwrap_or(6);
                self.open(FrameKind::Heading(level), style);
            }
            (true, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") => {
                self.flush_text();
                self.close_named(|kind| matches!(kind, FrameKind::Heading(_)));
            }
            (false, "table") => {
                self.flush_text();
                self.open(FrameKind::Table, style);
            }
            (true, "table") => {
                self.flush_text();
                self.close_named(|kind| matches!(kind, FrameKind::Table));
            }
            (false, "tr") => {
                self.flush_text();
                self.close_named(|kind| matches!(kind, FrameKind::Row | FrameKind::Cell(_)));
                self.open(FrameKind::Row, ComputedStyle::default());
            }
            (true, "tr") => {
                self.flush_text();
                self.close_named(|kind| matches!(kind, FrameKind::Row | FrameKind::Cell(_)));
            }
            (false, "td" | "th") => {
                self.flush_text();
                self.close_named(|kind| matches!(kind, FrameKind::Cell(_)));
                self.open(FrameKind::Cell(name == "th"), ComputedStyle::default());
            }
            (true, "td" | "th") => {
                self.flush_text();
                self.close_named(|kind| matches!(kind, FrameKind::Cell(_)));
            }
            // Void elements and anything unknown (incl. wrappers and
            // closers with no matching frame) are ignored.
            _ => {}
        }
    }

    fn open(&mut self, kind: FrameKind, style: ComputedStyle) {
        self.stack.push(Frame {
            kind,
            children: Vec::new(),
            style,
            cells: Vec::new(),
            rows: Vec::new(),
        });
    }

    /// Close the innermost frame matching `predicate` (and any frames opened
    /// inside it), folding the result into its parent.
    fn close_named(&mut self, predicate: impl Fn(&FrameKind) -> bool) {
        let Some(depth) = self.stack.iter().rposition(|f| predicate(&f.kind)) else {
            return;
        };
        while self.stack.len() > depth + 1 {
            self.close_top();
        }
        self.close_top();
    }

    fn close_top(&mut self) {
        let Some(mut frame) = self.stack.pop() else {
            return;
        };
        self.flush_text_into(&mut frame.children);
        let node = match frame.kind {
            FrameKind::Root => {
                // Nothing above the root to fold into.
                self.stack.push(frame);
                return;
            }
            FrameKind::Paragraph => HtmlNode::Block(BlockElement::Paragraph {
                children: frame.children,
                style: frame.style,
            }),
            FrameKind::Heading(level) => HtmlNode::Block(BlockElement::Heading {
                level,
                children: frame.children,
                style: frame.style,
            }),
            FrameKind::Cell(header) => {
                let mut children = frame.children;
                if header {
                    children = children
                        .into_iter()
                        .map(|node| match node {
                            HtmlNode::Text {
                                content, italic, ..
                            } => HtmlNode::Text {
                                content,
                                bold: true,
                                italic,
                            },
                            other @ HtmlNode::Block(_) => other,
                        })
                        .collect();
                }
                if let Some(row) = self.stack.last_mut() {
                    row.cells.push(children);
                }
                return;
            }
            FrameKind::Row => {
                let row = TableRow { cells: frame.cells };
                if let Some(table) = self.stack.last_mut() {
                    table.rows.push(row);
                }
                return;
            }
            FrameKind::Table => HtmlNode::Block(BlockElement::Table {
                rows: frame.rows,
                style: frame.style,
            }),
        };
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(node);
        }
    }

    fn flush_text(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let raw = std::mem::take(&mut self.text);
        if self.suppressed > 0 {
            return;
        }
        let content = collapse_whitespace(&decode_entities(&raw));
        if content.is_empty() {
            return;
        }
        if let Some(top) = self.stack.last_mut() {
            top.children.push(HtmlNode::Text {
                content,
                bold: self.bold > 0,
                italic: self.italic > 0,
            });
        }
    }

    /// Flush pending text into an explicit target (used when closing frames).
    fn flush_text_into(&mut self, target: &mut Vec<HtmlNode>) {
        if self.text.is_empty() || self.suppressed > 0 {
            self.text.clear();
            return;
        }
        let raw = std::mem::take(&mut self.text);
        let content = collapse_whitespace(&decode_entities(&raw));
        if content.is_empty() {
            return;
        }
        target.push(HtmlNode::Text {
            content,
            bold: self.bold > 0,
            italic: self.italic > 0,
        });
    }
}

enum Token {
    Text(String),
    Tag {
        name: String,
        closing: bool,
        style: ComputedStyle,
    },
}

/// Extract an attribute value from raw attribute text: `name="value"`,
/// `name='value'`, or `name=value`.
fn find_attr(attrs: &str, wanted: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let mut search_from = 0;
    loop {
        let pos = lower[search_from..].find(wanted)? + search_from;
        let after = &attrs[pos + wanted.len()..];
        let trimmed = after.trim_start();
        if !trimmed.starts_with('=') {
            // The attribute name merely *contains* the wanted text.
            search_from = pos + wanted.len();
            continue;
        }
        let value_area = trimmed[1..].trim_start();
        let quote = value_area.chars().next().filter(|c| is_quote(*c));
        let value = match quote {
            Some(q) => {
                let inner = &value_area[1..];
                match inner.find(q) {
                    Some(end) => &inner[..end],
                    None => inner,
                }
            }
            None => value_area.split_whitespace().next().unwrap_or(""),
        };
        return Some(value.to_owned());
    }
}

/// Whether `c` is a quote character that can delimit an attribute value.
fn is_quote(c: char) -> bool {
    c == '"' || c == '\u{27}' // apostrophe
}

/// Collapse runs of whitespace into single spaces and trim.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(c);
        }
    }
    out
}

/// Decode the five predefined HTML entities and numeric references.
#[must_use]
pub fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != '&' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        // Find the terminating ';' within a bounded window.
        let mut end = None;
        for (offset, c) in bytes.iter().enumerate().skip(i + 1).take(12) {
            if *c == ';' {
                end = Some(offset);
                break;
            }
            if !c.is_ascii_alphanumeric() && *c != '#' && *c != 'x' && *c != 'X' {
                break;
            }
        }
        let Some(end) = end else {
            out.push('&');
            i += 1;
            continue;
        };
        let entity: String = bytes[i + 1..end].iter().collect();
        match entity.as_str() {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            "nbsp" => out.push(' '),
            _ => {
                let decoded =
                    if let Some(hex) = entity.strip_prefix("#x").or(entity.strip_prefix("#X")) {
                        u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                    } else if let Some(dec) = entity.strip_prefix('#') {
                        dec.parse::<u32>().ok().and_then(char::from_u32)
                    } else {
                        None
                    };
                if let Some(c) = decoded {
                    out.push(c);
                } else {
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
            }
        }
        i = end + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paragraph_with_text() {
        let blocks = parse("<p>Hello world</p>");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            HtmlNode::Block(BlockElement::Paragraph { children, style }) => {
                assert_eq!(children.len(), 1);
                assert!(style.font_size_pt.is_none());
                match &children[0] {
                    HtmlNode::Text {
                        content,
                        bold,
                        italic,
                    } => {
                        assert_eq!(content, "Hello world");
                        assert!(!*bold && !*italic);
                    }
                    HtmlNode::Block(_) => panic!("expected text"),
                }
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn headings_carry_levels() {
        let blocks = parse("<h1>Title</h1><h3>Section</h3>");
        match &blocks[0] {
            HtmlNode::Block(BlockElement::Heading { level, .. }) => assert_eq!(*level, 1),
            _ => panic!("expected heading"),
        }
        match &blocks[1] {
            HtmlNode::Block(BlockElement::Heading { level, .. }) => assert_eq!(*level, 3),
            _ => panic!("expected heading"),
        }
    }

    #[test]
    fn bold_and_italic_apply_to_text() {
        let blocks = parse("<p><b>bold</b> and <i>italic</i> and <b><i>both</i></b></p>");
        match &blocks[0] {
            HtmlNode::Block(BlockElement::Paragraph { children, .. }) => {
                assert_eq!(children.len(), 5);
                assert!(matches!(
                    &children[0],
                    HtmlNode::Text {
                        bold: true,
                        italic: false,
                        ..
                    }
                ));
                assert!(matches!(
                    &children[2],
                    HtmlNode::Text {
                        bold: false,
                        italic: true,
                        ..
                    }
                ));
                assert!(matches!(
                    &children[4],
                    HtmlNode::Text {
                        bold: true,
                        italic: true,
                        ..
                    }
                ));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn tables_nest_rows_and_cells() {
        let blocks = parse(
            "<table><tr><th>Name</th><td>Value</td></tr><tr><td>a</td><td>b</td></tr></table>",
        );
        match &blocks[0] {
            HtmlNode::Block(BlockElement::Table { rows, .. }) => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].cells.len(), 2);
                assert_eq!(rows[1].cells.len(), 2);
                // `th` cells are bold.
                assert!(matches!(
                    &rows[0].cells[0][0],
                    HtmlNode::Text { bold: true, .. }
                ));
                assert!(matches!(
                    &rows[0].cells[1][0],
                    HtmlNode::Text { bold: false, .. }
                ));
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn entities_are_decoded() {
        assert_eq!(
            decode_entities("a &amp; b &lt;c&gt; &quot;d&quot; &#39;e&#39; &#65;"),
            "a & b <c> \"d\" 'e' A"
        );
        assert_eq!(decode_entities("plain"), "plain");
        assert_eq!(decode_entities("dangling &amp"), "dangling &amp");
    }

    #[test]
    fn wrappers_and_unknown_tags_are_transparent() {
        let blocks = parse(
            "<html><head><title>x</title></head><body><section><p>Text</p></section></body></html>",
        );
        assert_eq!(blocks.len(), 1);
        assert!(matches!(
            &blocks[0],
            HtmlNode::Block(BlockElement::Paragraph { .. })
        ));
    }

    #[test]
    fn malformed_input_never_panics() {
        for input in [
            "",
            "<",
            "<p",
            "</>",
            "<<<>>>",
            "<p>unclosed",
            "</p>stray",
            "<table><td>orphan</table>",
            "<b><i>crossed</b></i>",
            "<p style= >empty</p>",
        ] {
            let _ = parse(input);
        }
    }

    #[test]
    fn style_attribute_is_parsed_into_blocks() {
        let blocks = parse("<p style=\"font-size: 14pt; text-align: center\">Hi</p>");
        match &blocks[0] {
            HtmlNode::Block(BlockElement::Paragraph { style, .. }) => {
                assert_eq!(style.font_size_pt, Some(14.0));
                assert_eq!(style.text_align, Some(crate::css::TextAlign::Center));
            }
            _ => panic!("expected paragraph"),
        }
    }
}

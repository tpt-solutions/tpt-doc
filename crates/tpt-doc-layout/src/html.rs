/// Supported HTML elements in the layout subset.
#[derive(Debug, Clone, PartialEq)]
pub enum HtmlNode {
    /// A block-level element.
    Block(BlockElement),
    /// Raw text content.
    Text(String),
}

/// Block-level HTML elements supported by the layout engine.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockElement {
    /// `<div>` or `<p>`.
    Paragraph { children: Vec<HtmlNode> },
    /// `<h1>` through `<h6>`.
    Heading { level: u8, children: Vec<HtmlNode> },
    /// `<table>`.
    Table { rows: Vec<TableRow> },
}

/// A `<tr>` row in a table.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    /// `<td>` or `<th>` cells.
    pub cells: Vec<Vec<HtmlNode>>,
}

/// Parse a restricted HTML string into a node tree.
///
/// # Errors
/// Currently unimplemented — returns `None`.
pub fn parse(_html: &str) -> Option<Vec<HtmlNode>> {
    // TODO(phase-3): implement HTML parser using html5ever or scraper
    None
}

//! Structural reader for `.docx` files: paragraphs with their styles and runs,
//! tables with their grid and cells, and the header and footer text.
//!
//! The reader is the inverse of [`DocxWriter`](crate::DocxWriter) for the
//! features the model carries. Anything the model does not represent (fields,
//! images, comments, section properties, numbering) is skipped rather than
//! guessed at, so reading a document never invents content.
//!
//! The XML is first parsed into a small element tree, bounded by
//! [`Limits::max_xml_depth`] and by the entry size limit, and the tree is then
//! converted into the model.

use std::collections::HashMap;

use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};
use tpt_doc_core::{DocError, Limits};

use crate::DocxDocument;
use crate::document::resolve_ref;
use crate::paragraph::{ListKind, Paragraph, Run, RunStyle};
use crate::table::{BorderStyle, Cell, Row, Table};

impl DocxDocument {
    /// Read a `.docx` package with the default [`Limits`].
    ///
    /// # Errors
    /// Returns [`DocError`] if the bytes are not a `.docx` package, its
    /// document part is missing or malformed, or a part exceeds the limits.
    pub fn read(bytes: &[u8]) -> Result<Self, DocError> {
        Self::read_with_limits(bytes, &Limits::default())
    }

    /// Read a `.docx` package with explicit [`Limits`].
    ///
    /// # Errors
    /// As [`DocxDocument::read`], and if a part is larger than
    /// `limits.max_entry_bytes` or nests deeper than `limits.max_xml_depth`.
    pub fn read_with_limits(bytes: &[u8], limits: &Limits) -> Result<Self, DocError> {
        read_docx(bytes, limits)
    }
}

/// Read a complete `.docx` package into a [`DocxDocument`].
fn read_docx(bytes: &[u8], limits: &Limits) -> Result<DocxDocument, DocError> {
    let mut archive = tpt_doc_ooxml::open_archive(bytes)?;
    let document = tpt_doc_ooxml::read_part(&mut archive, "word/document.xml", limits)?;
    let tree = parse_tree(&document, limits)?;
    let refs = Refs {
        links: read_links(&mut archive, limits)?,
        lists: read_numbering(&mut archive, limits)?,
    };
    let body = root(&tree)
        .and_then(|doc| doc.child("w:body"))
        .ok_or_else(|| DocError::invalid_format("word/document.xml has no w:body"))?;

    let mut out = DocxDocument::new();
    for element in &body.children {
        match element.name.as_str() {
            "w:p" => out.push_paragraph(paragraph_from(element, &refs)),
            "w:tbl" => out.push_table(table_from(element, &refs)),
            _ => {}
        }
    }

    for (part, set) in [
        (
            "word/header1.xml",
            DocxDocument::set_header as fn(&mut DocxDocument, String),
        ),
        ("word/footer1.xml", DocxDocument::set_footer),
    ] {
        if archive.index_for_name(part).is_some() {
            let xml = tpt_doc_ooxml::read_part(&mut archive, part, limits)?;
            let tree = parse_tree(&xml, limits)?;
            if let Some(element) = root(&tree) {
                set(&mut out, text_of_paragraphs(element, &Refs::default()));
            }
        }
    }
    Ok(out)
}

/// An element and its content, as read from the XML.
#[derive(Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    /// Character data directly inside this element (entities resolved).
    text: String,
}

impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// The single element directly under the synthetic document node.
fn root(tree: &Node) -> Option<&Node> {
    tree.children.first()
}

fn parse_tree(xml: &[u8], limits: &Limits) -> Result<Node, DocError> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    let mut stack = vec![Node::default()];
    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) => {
                stack.push(element(&start)?);
                limits.check_xml_depth(stack.len())?;
            }
            Ok(Event::Empty(start)) => {
                let node = element(&start)?;
                limits.check_xml_depth(stack.len() + 1)?;
                top(&mut stack)?.children.push(node);
            }
            Ok(Event::End(_)) => {
                let node = stack
                    .pop()
                    .ok_or_else(|| DocError::invalid_format("unbalanced XML end tag"))?;
                top(&mut stack)?.children.push(node);
            }
            Ok(Event::Text(text)) => {
                let decoded = quick_xml::escape::unescape(&text)
                    .map_err(|e| DocError::invalid_format(format!("XML text: {e}")))?;
                top(&mut stack)?.text.push_str(&decoded);
            }
            Ok(Event::GeneralRef(reference)) => {
                let resolved = resolve_ref(&reference)?;
                top(&mut stack)?.text.push_str(&resolved);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(DocError::invalid_format(format!("XML error: {e}"))),
        }
    }
    if stack.len() != 1 {
        return Err(DocError::invalid_format(
            "XML ends with an unclosed element",
        ));
    }
    stack
        .pop()
        .ok_or_else(|| DocError::invalid_format("empty XML document"))
}

fn top(stack: &mut [Node]) -> Result<&mut Node, DocError> {
    stack
        .last_mut()
        .ok_or_else(|| DocError::invalid_format("XML content outside the document"))
}

fn element(start: &BytesStart<'_>) -> Result<Node, DocError> {
    let qname = start.name();
    let name: &str = qname.as_ref();
    let name = name.to_owned();
    let mut attrs = Vec::new();
    for attribute in start.attributes() {
        let attribute =
            attribute.map_err(|e| DocError::invalid_format(format!("XML attribute: {e}")))?;
        let qkey = attribute.key;
        let key: &str = qkey.as_ref();
        let key = key.to_owned();
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|e| DocError::invalid_format(format!("XML attribute value: {e}")))?;
        attrs.push((key, value.into_owned()));
    }
    Ok(Node {
        name,
        attrs,
        ..Node::default()
    })
}

/// What the document's relationships and numbering say about its paragraphs.
#[derive(Default)]
struct Refs {
    /// External hyperlink targets, by relationship id.
    links: HashMap<String, String>,
    /// The kind of each list, by `numId`.
    lists: HashMap<String, ListKind>,
}

/// The list kinds of the document, by `numId`, from `word/numbering.xml`.
///
/// A `num` points at an `abstractNum`; the kind comes from the number format of
/// that abstract list's first level (`bullet` is a bulleted list, anything else
/// is numbered).
fn read_numbering(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    limits: &Limits,
) -> Result<HashMap<String, ListKind>, DocError> {
    const PART: &str = "word/numbering.xml";
    if archive.index_for_name(PART).is_none() {
        return Ok(HashMap::new());
    }
    let xml = tpt_doc_ooxml::read_part(archive, PART, limits)?;
    let tree = parse_tree(&xml, limits)?;
    let Some(numbering) = root(&tree) else {
        return Ok(HashMap::new());
    };

    let mut kinds: HashMap<String, ListKind> = HashMap::new();
    for abstract_list in numbering.children_named("w:abstractNum") {
        let Some(id) = abstract_list.attr("w:abstractNumId") else {
            continue;
        };
        let format = abstract_list
            .children_named("w:lvl")
            .find(|lvl| lvl.attr("w:ilvl").is_none_or(|level| level == "0"))
            .and_then(|lvl| lvl.child("w:numFmt"))
            .and_then(|fmt| fmt.attr("w:val"));
        let kind = if format == Some("bullet") {
            ListKind::Bulleted
        } else {
            ListKind::Numbered
        };
        kinds.insert(id.to_owned(), kind);
    }

    let mut lists = HashMap::new();
    for num in numbering.children_named("w:num") {
        let (Some(num_id), Some(abstract_id)) = (
            num.attr("w:numId"),
            num.child("w:abstractNumId").and_then(|a| a.attr("w:val")),
        ) else {
            continue;
        };
        if let Some(kind) = kinds.get(abstract_id) {
            lists.insert(num_id.to_owned(), *kind);
        }
    }
    Ok(lists)
}

/// The external hyperlink targets of the document part, by relationship id.
fn read_links(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    limits: &Limits,
) -> Result<HashMap<String, String>, DocError> {
    const RELS: &str = "word/_rels/document.xml.rels";
    if archive.index_for_name(RELS).is_none() {
        return Ok(HashMap::new());
    }
    let xml = tpt_doc_ooxml::read_part(archive, RELS, limits)?;
    let tree = parse_tree(&xml, limits)?;
    let mut links = HashMap::new();
    if let Some(relationships) = root(&tree) {
        for relationship in relationships.children_named("Relationship") {
            if relationship.attr("TargetMode") == Some("External")
                && let (Some(id), Some(target)) =
                    (relationship.attr("Id"), relationship.attr("Target"))
            {
                links.insert(id.to_owned(), target.to_owned());
            }
        }
    }
    Ok(links)
}

fn paragraph_from(node: &Node, refs: &Refs) -> Paragraph {
    let style = node
        .child("w:pPr")
        .and_then(|props| props.child("w:pStyle"))
        .and_then(|style| style.attr("w:val"))
        .map(str::to_owned);
    let mut runs = Vec::new();
    for child in &node.children {
        match child.name.as_str() {
            "w:r" => runs.push(run_from(child)),
            "w:hyperlink" => {
                // A run inside a hyperlink keeps its target, when the target resolves.
                let target = child.attr("r:id").and_then(|id| refs.links.get(id));
                for run in child.children_named("w:r") {
                    let mut run = run_from(run);
                    run.link = target.cloned();
                    runs.push(run);
                }
            }
            _ => {}
        }
    }
    let list = node
        .child("w:pPr")
        .and_then(|props| props.child("w:numPr"))
        .and_then(|num| num.child("w:numId"))
        .and_then(|id| id.attr("w:val"))
        .and_then(|id| refs.lists.get(id))
        .copied();
    Paragraph::from_parts(style, list, runs)
}

fn run_from(node: &Node) -> Run {
    let mut text = String::new();
    for child in &node.children {
        match child.name.as_str() {
            "w:t" => text.push_str(&child.text),
            "w:tab" => text.push('\t'),
            "w:br" | "w:cr" => text.push('\n'),
            _ => {}
        }
    }
    Run::styled(text, run_style(node))
}

/// A toggle such as `<w:b/>` is on unless it says `w:val="0"` or `"false"`.
fn toggle(node: Option<&Node>) -> bool {
    node.is_some_and(|n| !matches!(n.attr("w:val"), Some("0" | "false")))
}

fn run_style(run: &Node) -> RunStyle {
    let props = run.child("w:rPr");
    RunStyle {
        bold: toggle(props.and_then(|p| p.child("w:b"))),
        italic: toggle(props.and_then(|p| p.child("w:i"))),
        underline: props
            .and_then(|p| p.child("w:u"))
            .and_then(|u| u.attr("w:val"))
            .is_some_and(|value| value != "none"),
        font_size_half_points: props
            .and_then(|p| p.child("w:sz"))
            .and_then(|sz| sz.attr("w:val"))
            .and_then(|value| value.parse::<u16>().ok()),
    }
}

fn table_from(node: &Node, refs: &Refs) -> Table {
    let mut table = Table::new();
    let props = node.child("w:tblPr");
    table.borders = table_borders(props);
    table.column_widths_twips = node
        .child("w:tblGrid")
        .map(|grid| {
            grid.children_named("w:gridCol")
                .filter_map(|col| col.attr("w:w").and_then(|w| w.parse::<u16>().ok()))
                .collect()
        })
        .unwrap_or_default();
    for row in node.children_named("w:tr") {
        let cells: Vec<Cell> = row
            .children_named("w:tc")
            .map(|cell| {
                let paragraphs = cell
                    .children_named("w:p")
                    .map(|p| paragraph_from(p, refs))
                    .collect();
                Cell::from_paragraphs(paragraphs)
            })
            .collect();
        table.push_row(Row::new(cells));
    }
    table
}

fn table_borders(props: Option<&Node>) -> BorderStyle {
    if let Some(borders) = props.and_then(|p| p.child("w:tblBorders")) {
        // The writer marks every side with the same value; the first side decides.
        return match borders.children.first().and_then(|side| side.attr("w:val")) {
            Some("double") => BorderStyle::Double,
            Some("single") => BorderStyle::Single,
            _ => BorderStyle::None,
        };
    }
    let grid_style = props
        .and_then(|p| p.child("w:tblStyle"))
        .and_then(|s| s.attr("w:val"));
    if grid_style == Some("TableGrid") {
        BorderStyle::Single
    } else {
        BorderStyle::None
    }
}

/// The text of a header or footer: its paragraphs, joined by newlines.
fn text_of_paragraphs(element: &Node, refs: &Refs) -> String {
    let paragraphs: Vec<String> = element
        .children_named("w:p")
        .map(|p| paragraph_from(p, refs).plain_text())
        .collect();
    paragraphs.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_respect_explicit_off_values() {
        let on = Node {
            name: "w:b".into(),
            ..Node::default()
        };
        let off = Node {
            name: "w:b".into(),
            attrs: vec![("w:val".into(), "0".into())],
            ..Node::default()
        };
        assert!(toggle(Some(&on)));
        assert!(!toggle(Some(&off)));
        assert!(!toggle(None));
    }
}

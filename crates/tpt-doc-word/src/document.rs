use std::fmt::Write as _;
use std::io::{Read as _, Write as _};

use quick_xml::escape::escape;
use quick_xml::events::Event;
use tpt_doc_core::DocError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::styles::BuiltinStyle;
use crate::table::BorderStyle;
use crate::{Cell, Paragraph, Run, Table};

/// A Word document body element — either a paragraph or a table.
#[derive(Debug, Clone)]
pub enum BodyElement {
    /// A paragraph (may contain multiple [`Run`](crate::Run)s).
    Paragraph(Paragraph),
    /// A table.
    Table(Table),
}

/// An in-memory Word document model.
#[derive(Debug, Default)]
pub struct DocxDocument {
    body: Vec<BodyElement>,
    /// Optional running header text (`word/header1.xml`).
    header: Option<String>,
    /// Optional running footer text (`word/footer1.xml`).
    footer: Option<String>,
}

impl DocxDocument {
    /// Create an empty document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a paragraph.
    pub fn push_paragraph(&mut self, p: Paragraph) {
        self.body.push(BodyElement::Paragraph(p));
    }

    /// Append a table.
    pub fn push_table(&mut self, t: Table) {
        self.body.push(BodyElement::Table(t));
    }

    /// Body elements in document order.
    #[must_use]
    pub fn body(&self) -> &[BodyElement] {
        &self.body
    }

    /// Set the running header text (emitted as a single paragraph).
    pub fn set_header(&mut self, text: impl Into<String>) {
        self.header = Some(text.into());
    }

    /// Set the running footer text (emitted as a single paragraph).
    pub fn set_footer(&mut self, text: impl Into<String>) {
        self.footer = Some(text.into());
    }

    /// The running header text, if set.
    #[must_use]
    pub fn header(&self) -> Option<&str> {
        self.header.as_deref()
    }

    /// The running footer text, if set.
    #[must_use]
    pub fn footer(&self) -> Option<&str> {
        self.footer.as_deref()
    }
}

/// Serializes a [`DocxDocument`] to `.docx` bytes.
pub struct DocxWriter;

impl DocxWriter {
    /// Serialize the document to `.docx` bytes (ZIP + OOXML XML parts).
    ///
    /// The package contains `[Content_Types].xml`, `_rels/.rels`,
    /// `word/document.xml`, `word/styles.xml`, `word/settings.xml`, and —
    /// when configured — `word/header1.xml` / `word/footer1.xml`.
    ///
    /// # Errors
    /// Returns [`DocError`] if ZIP assembly or XML serialization fails.
    pub fn write(doc: &DocxDocument) -> Result<Vec<u8>, DocError> {
        let mut content_types = String::from(XML_DECL);
        content_types.push_str(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        );
        content_types.push_str(
            r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        );
        content_types.push_str(r#"<Default Extension="xml" ContentType="application/xml"/>"#);
        for (part, content_type) in [
            (
                "/word/document.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
            ),
            (
                "/word/styles.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
            ),
            (
                "/word/settings.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml",
            ),
        ] {
            let _ = write!(
                content_types,
                r#"<Override PartName="{part}" ContentType="{content_type}"/>"#
            );
        }
        if doc.header().is_some() {
            content_types.push_str(
                r#"<Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/>"#,
            );
        }
        if doc.footer().is_some() {
            content_types.push_str(
                r#"<Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>"#,
            );
        }
        content_types.push_str("</Types>");

        let document = document_xml(doc);
        let styles = styles_xml(doc);

        // Package-level: the main document part.
        let root_rels = concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
            r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>"#,
            r#"</Relationships>"#
        );

        // Document-level relationships: styles (rId1), settings (rId2),
        // header (rId3), footer (rId4) — header/footer emitted only when set.
        let mut rels = String::from(XML_DECL);
        rels.push_str(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        );
        rels.push_str(
            r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
        );
        rels.push_str(
            r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>"#,
        );
        if doc.header().is_some() {
            rels.push_str(
                r#"<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/>"#,
            );
        }
        if doc.footer().is_some() {
            rels.push_str(
                r#"<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>"#,
            );
        }
        rels.push_str("</Relationships>");

        let cursor = std::io::Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let mut parts: Vec<(&str, String)> = vec![
            ("[Content_Types].xml", content_types),
            ("_rels/.rels", root_rels.to_owned()),
            ("word/document.xml", document),
            ("word/_rels/document.xml.rels", rels),
            ("word/styles.xml", styles),
            ("word/settings.xml", SETTINGS_XML.to_owned()),
        ];
        if let Some(header) = doc.header() {
            parts.push(("word/header1.xml", header_or_footer_xml("w:hdr", header)));
        }
        if let Some(footer) = doc.footer() {
            parts.push(("word/footer1.xml", header_or_footer_xml("w:ftr", footer)));
        }

        for (name, content) in &parts {
            zip.start_file(name, options)
                .map_err(|e| DocError::invalid_format(format!("ZIP entry `{name}` failed: {e}")))?;
            zip.write_all(content.as_bytes())?;
        }
        let cursor = zip
            .finish()
            .map_err(|e| DocError::invalid_format(format!("ZIP finalization failed: {e}")))?;
        Ok(cursor.into_inner())
    }
}

const XML_DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

const SETTINGS_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
    r#"<w:compat>"#,
    r#"<w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/>"#,
    r#"</w:compat>"#,
    r#"</w:settings>"#
);

/// Serialize the document body to the `word/document.xml` part.
pub(crate) fn document_xml(doc: &DocxDocument) -> String {
    let mut xml = String::from(XML_DECL);
    xml.push_str(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>"#,
    );
    for element in doc.body() {
        match element {
            BodyElement::Paragraph(p) => paragraph_xml(&mut xml, p),
            BodyElement::Table(t) => table_xml(&mut xml, t),
        }
    }
    xml.push_str("<w:sectPr>");
    if doc.header().is_some() {
        xml.push_str(r#"<w:headerReference w:type="default" r:id="rId3"/>"#);
    }
    if doc.footer().is_some() {
        xml.push_str(r#"<w:footerReference w:type="default" r:id="rId4"/>"#);
    }
    // A4 portrait, 1-inch margins — a valid, deterministic default section.
    xml.push_str(
        r#"<w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1800" w:bottom="1440" w:left="1800" w:header="708" w:footer="708" w:gutter="0"/>"#,
    );
    xml.push_str("</w:sectPr></w:body></w:document>");
    xml
}

fn paragraph_xml(xml: &mut String, p: &Paragraph) {
    xml.push_str("<w:p>");
    if let Some(style_id) = &p.style_id {
        let escaped = escape(style_id);
        let _ = write!(xml, r#"<w:pPr><w:pStyle w:val="{escaped}"/></w:pPr>"#);
    }
    for run in p.runs() {
        run_xml(xml, run);
    }
    xml.push_str("</w:p>");
}

fn run_xml(xml: &mut String, run: &Run) {
    xml.push_str("<w:r>");
    let style = &run.style;
    if style.bold || style.italic || style.underline || style.font_size_half_points.is_some() {
        xml.push_str("<w:rPr>");
        if style.bold {
            xml.push_str("<w:b/>");
        }
        if style.italic {
            xml.push_str("<w:i/>");
        }
        if style.underline {
            xml.push_str(r#"<w:u w:val="single"/>"#);
        }
        if let Some(size) = style.font_size_half_points {
            let _ = write!(xml, r#"<w:sz w:val="{size}"/>"#);
        }
        xml.push_str("</w:rPr>");
    }
    let escaped = escape(&run.text);
    let preserve = run.text.starts_with([' ', '\t', '\n']) || run.text.ends_with([' ', '\t', '\n']);
    let _ = write!(
        xml,
        r"<w:t{}>{escaped}</w:t>",
        if preserve {
            r#" xml:space="preserve""#
        } else {
            ""
        }
    );
    xml.push_str("</w:r>");
}

fn table_xml(xml: &mut String, table: &Table) {
    xml.push_str("<w:tbl><w:tblPr>");
    match table.borders {
        BorderStyle::None => {}
        BorderStyle::Single => {
            xml.push_str(r#"<w:tblStyle w:val="TableGrid"/>"#);
        }
        BorderStyle::Double => {
            let border = r#"w:val="double" w:sz="4" w:space="0" w:color="auto""#;
            let _ = write!(
                xml,
                r"<w:tblBorders><w:top {border}/><w:left {border}/><w:bottom {border}/><w:right {border}/><w:insideH {border}/><w:insideV {border}/></w:tblBorders>"
            );
        }
    }
    xml.push_str("</w:tblPr>");

    // Column grid: explicit widths when provided, otherwise a fixed default
    // column width (2340 twips), which Word stretches to the page width.
    let columns = table
        .rows()
        .iter()
        .map(|row| row.cells().len())
        .max()
        .unwrap_or(0);
    xml.push_str("<w:tblGrid>");
    if table.column_widths_twips.is_empty() {
        for _ in 0..columns {
            xml.push_str(r#"<w:gridCol w:w="2340"/>"#);
        }
    } else {
        for width in &table.column_widths_twips {
            let _ = write!(xml, r#"<w:gridCol w:w="{width}"/>"#);
        }
    }
    xml.push_str("</w:tblGrid>");

    for row in table.rows() {
        xml.push_str("<w:tr>");
        for (index, cell) in row.cells().iter().enumerate() {
            let width = table.column_widths_twips.get(index).copied();
            cell_xml(xml, cell, width);
        }
        xml.push_str("</w:tr>");
    }
    xml.push_str("</w:tbl>");
}

fn cell_xml(xml: &mut String, cell: &Cell, width_twips: Option<u16>) {
    xml.push_str("<w:tc><w:tcPr>");
    match width_twips {
        Some(width) => {
            let _ = write!(xml, r#"<w:tcW w:w="{width}" w:type="dxa"/>"#);
        }
        None => xml.push_str(r#"<w:tcW w:w="0" w:type="auto"/>"#),
    }
    xml.push_str("</w:tcPr>");
    for paragraph in cell.paragraphs() {
        paragraph_xml(xml, paragraph);
    }
    // A table cell must end with a paragraph; empty cells get one.
    if cell.paragraphs().is_empty() {
        xml.push_str("<w:p/>");
    }
    xml.push_str("</w:tc>");
}

fn header_or_footer_xml(root: &str, text: &str) -> String {
    let escaped = escape(text);
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<{root} xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
            r#"<w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#,
            r#"</{root}>"#
        ),
        root = root,
        text = escaped
    )
}

/// Build `word/styles.xml` from the styles the document actually uses, plus
/// `TableGrid` when any table is present.
fn styles_xml(doc: &DocxDocument) -> String {
    let mut set = crate::styles::StyleSet::new();
    let mut has_table = false;
    for element in doc.body() {
        match element {
            BodyElement::Paragraph(p) => {
                if let Some(style) = p.style_id.as_deref().and_then(BuiltinStyle::from_style_id) {
                    set.mark_used(style);
                }
            }
            BodyElement::Table(_) => has_table = true,
        }
    }

    let mut xml = String::from(XML_DECL);
    xml.push_str(
        r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
    );
    for style in set.iter() {
        style_xml(&mut xml, style);
    }
    if has_table {
        table_grid_style_xml(&mut xml);
    }
    xml.push_str("</w:styles>");
    xml
}

fn style_xml(xml: &mut String, style: BuiltinStyle) {
    let id = style.style_id();
    let name = style.name();
    let level = match style {
        BuiltinStyle::Heading1 => Some(0u8),
        BuiltinStyle::Heading2 => Some(1),
        BuiltinStyle::Heading3 => Some(2),
        BuiltinStyle::Heading4 => Some(3),
        BuiltinStyle::Heading5 => Some(4),
        BuiltinStyle::Heading6 => Some(5),
        _ => None,
    };
    xml.push_str(r#"<w:style w:type="paragraph""#);
    if style == BuiltinStyle::Normal {
        xml.push_str(r#" w:default="1""#);
    }
    let _ = write!(xml, r#" w:styleId="{id}"><w:name w:val="{name}"/>"#);
    if style != BuiltinStyle::Normal {
        xml.push_str(r#"<w:basedOn w:val="Normal"/><w:next w:val="Normal"/>"#);
    }
    xml.push_str("<w:qFormat/>");
    if let Some(level) = level {
        let _ = write!(
            xml,
            r#"<w:pPr><w:keepNext/><w:outlineLvl w:val="{level}"/></w:pPr>"#
        );
    }
    xml.push_str("</w:style>");
}

fn table_grid_style_xml(xml: &mut String) {
    let border = r#"w:val="single" w:sz="4" w:space="0" w:color="auto""#;
    xml.push_str(r#"<w:style w:type="table" w:styleId="TableGrid">"#);
    xml.push_str(r#"<w:name w:val="Table Grid"/><w:qFormat/>"#);
    let _ = write!(
        xml,
        r"<w:tblPr><w:tblBorders><w:top {border}/><w:left {border}/><w:bottom {border}/><w:right {border}/><w:insideH {border}/><w:insideV {border}/></w:tblBorders></w:tblPr>"
    );
    xml.push_str("</w:style>");
}

/// Reads basic content out of `.docx` files.
pub struct DocxReader;

impl DocxReader {
    /// Extract plain text from `word/document.xml`.
    ///
    /// Paragraphs are separated by `\n`; `w:tab` and `w:br` become `\t` and
    /// `\n`. Text in headers, footers, and non-body parts is not included.
    ///
    /// # Errors
    /// Returns [`DocError`] if the bytes are not a readable `.docx` file.
    pub fn extract_text(bytes: &[u8]) -> Result<String, DocError> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| DocError::invalid_format(format!("not a valid ZIP archive: {e}")))?;
        let mut document = archive
            .by_name("word/document.xml")
            .map_err(|_| DocError::invalid_format("missing ZIP entry `word/document.xml`"))?;
        let mut xml = Vec::new();
        document.read_to_end(&mut xml)?;

        let mut reader = quick_xml::Reader::from_reader(xml.as_slice());
        let mut text = String::new();
        let mut capture = false;
        loop {
            match reader.read_event() {
                Ok(Event::Start(start)) => {
                    if start.local_name().as_ref() == "t" {
                        capture = true;
                    }
                }
                Ok(Event::Empty(start)) => match start.local_name().as_ref() {
                    "tab" => text.push('\t'),
                    "br" => text.push('\n'),
                    _ => {}
                },
                Ok(Event::Text(t)) if capture => match quick_xml::escape::unescape(&t) {
                    Ok(chunk) => text.push_str(&chunk),
                    Err(e) => {
                        return Err(DocError::invalid_format(format!(
                            "invalid XML entities: {e}"
                        )));
                    }
                },
                Ok(Event::GeneralRef(reference)) if capture => {
                    text.push_str(&resolve_ref(&reference)?);
                }
                Ok(Event::End(end)) => match end.local_name().as_ref() {
                    "t" => capture = false,
                    "p" => text.push('\n'),
                    _ => {}
                },
                Ok(Event::Eof) => {
                    while text.ends_with('\n') {
                        text.pop();
                    }
                    return Ok(text);
                }
                Ok(_) => {}
                Err(e) => return Err(DocError::invalid_format(format!("XML error: {e}"))),
            }
        }
    }
}

/// Resolve an XML general or character reference (`&amp;`, `&#38;`) to text.
///
/// OOXML parts define no custom entities, so only the five predefined
/// references and character references are accepted.
fn resolve_ref(r: &quick_xml::events::BytesRef<'_>) -> Result<String, DocError> {
    if r.is_char_ref() {
        return match r.resolve_char_ref() {
            Ok(Some(ch)) => Ok(ch.to_string()),
            _ => Err(DocError::invalid_format(format!(
                "malformed character reference `&{};`",
                r.as_ref()
            ))),
        };
    }
    match quick_xml::escape::resolve_predefined_entity(r) {
        Some(text) => Ok(text.to_owned()),
        None => Err(DocError::invalid_format(format!(
            "unknown entity reference `&{};`",
            r.as_ref()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cell, Row, RunStyle, Table};

    /// A representative fixture document used by the snapshot test.
    fn fixture_document() -> DocxDocument {
        let mut doc = DocxDocument::new();
        doc.set_header("TPT Solutions — Confidential");
        doc.set_footer("Page footer");
        let mut heading = Paragraph::styled("Heading1", "Quarterly Report");
        heading.push_run(Run::styled(" (draft)", RunStyle::default().italic()));
        doc.push_paragraph(heading);
        doc.push_paragraph(Paragraph::new("Prepared by <Engineering> & Operations."));
        let mut table = Table::new();
        table.push_row(Row::new([Cell::new("Region"), Cell::new("Revenue")]));
        table.push_row(Row::new([Cell::new("Auckland"), Cell::new("12000")]));
        doc.push_table(table);
        doc
    }

    #[test]
    fn document_xml_is_deterministic_for_fixture() {
        insta::assert_snapshot!(document_xml(&fixture_document()));
    }

    #[test]
    fn writer_package_contains_required_parts() {
        let bytes = DocxWriter::write(&fixture_document()).expect("write");
        let cursor = std::io::Cursor::new(&bytes[..]);
        let mut archive = ZipArchive::new(cursor).expect("zip");
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/document.xml",
            "word/_rels/document.xml.rels",
            "word/styles.xml",
            "word/settings.xml",
            "word/header1.xml",
            "word/footer1.xml",
        ] {
            assert!(archive.by_name(name).is_ok(), "missing entry {name}");
        }
    }

    #[test]
    fn content_types_declare_header_footer_only_when_set() {
        let plain = DocxDocument::new();
        let bytes = DocxWriter::write(&plain).expect("write");
        let text = DocxReader::extract_text(&bytes);
        assert!(text.is_ok());
        let cursor = std::io::Cursor::new(&bytes[..]);
        let mut archive = ZipArchive::new(cursor).expect("zip");
        let mut content_types = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_name("[Content_Types].xml").expect("part"),
            &mut content_types,
        )
        .expect("read");
        let content_types = String::from_utf8(content_types).expect("UTF-8");
        assert!(!content_types.contains("header1.xml"));
        assert!(!content_types.contains("footer1.xml"));
    }

    #[test]
    fn styles_xml_lists_only_used_styles() {
        let mut doc = DocxDocument::new();
        doc.push_paragraph(Paragraph::styled("Heading2", "Section"));
        let bytes = DocxWriter::write(&doc).expect("write");
        let cursor = std::io::Cursor::new(&bytes[..]);
        let mut archive = ZipArchive::new(cursor).expect("zip");
        let mut styles = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_name("word/styles.xml").expect("part"),
            &mut styles,
        )
        .expect("read");
        let styles = String::from_utf8(styles).expect("UTF-8");
        assert!(styles.contains(r#"w:styleId="Normal""#));
        assert!(styles.contains(r#"w:styleId="Heading2""#));
        assert!(!styles.contains(r#"w:styleId="Heading1""#));
        assert!(!styles.contains("TableGrid"));
    }

    #[test]
    fn writer_output_round_trips_through_reader() {
        let bytes = DocxWriter::write(&fixture_document()).expect("write");
        let text = DocxReader::extract_text(&bytes).expect("extract");
        // Extraction covers body content only — headers and footers are
        // separate parts and intentionally not included.
        let lines: Vec<&str> = text.split('\n').collect();
        assert_eq!(
            lines,
            [
                "Quarterly Report (draft)",
                "Prepared by <Engineering> & Operations.",
                "Region",
                "Revenue",
                "Auckland",
                "12000",
            ]
        );
    }

    #[test]
    fn extraction_tolerates_missing_document_part() {
        assert!(DocxReader::extract_text(b"not a zip").is_err());
    }
}

use std::fmt::Write as _;
use std::io::{Read, Write};

use quick_xml::XmlVersion;
use tpt_doc_core::escape_xml_text;
use quick_xml::events::Event;
use tpt_doc_core::DocError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// A single spreadsheet cell value.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A string value (from the shared strings table or inline).
    String(String),
    /// A numeric value.
    Number(f64),
    /// A boolean value.
    Boolean(bool),
    /// An error value (e.g. `#REF!`).
    Error(String),
    /// An empty cell.
    Blank,
}

/// A worksheet row: an ordered list of cells with a 1-based row index.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// 1-based row number.
    pub index: u32,
    /// Cells in column order.
    pub cells: Vec<Cell>,
}

impl Row {
    /// Return the cells in this row.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
}

/// Streaming `.xlsx` worksheet reader.
///
/// Reads the ZIP entries of an `.xlsx` file and yields [`Row`] values one at
/// a time via a pull-based XML event loop — peak memory is proportional to a
/// single worksheet plus one row, never the whole workbook as a DOM tree.
///
/// Only the first worksheet (`xl/worksheets/sheet1.xml`) is read; the shared
/// strings table (`xl/sharedStrings.xml`) is parsed when present so that
/// `t="s"` cells written by Excel and `LibreOffice` resolve correctly.
pub struct XlsxReader {
    sheet: Vec<u8>,
    shared_strings: Vec<String>,
}

impl XlsxReader {
    /// Create a new reader over the given `.xlsx` bytes.
    ///
    /// # Errors
    /// Returns [`DocError`] if the bytes are not a valid `.xlsx` (OOXML) file
    /// or the first worksheet entry is missing.
    pub fn new(bytes: &[u8]) -> Result<Self, DocError> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| DocError::invalid_format(format!("not a valid ZIP archive: {e}")))?;
        let shared_strings = read_shared_strings(&mut archive)?;
        let sheet = read_entry(&mut archive, "xl/worksheets/sheet1.xml")?;
        Ok(Self {
            sheet,
            shared_strings,
        })
    }

    /// Iterate over worksheet rows.
    pub fn rows(&mut self) -> impl Iterator<Item = Result<Row, DocError>> + '_ {
        RowsIter {
            reader: quick_xml::Reader::from_reader(self.sheet.as_slice()),
            shared: &self.shared_strings,
            row_index: 1,
            next_row_index: 1,
            row_cells: Vec::new(),
            cell: CellState::default(),
            capture: Capture::None,
            done: false,
        }
    }
}

/// XML version assumed when normalizing values; OOXML parts declare XML 1.0.
const XML_VERSION: XmlVersion = XmlVersion::Implicit1_0;

/// What the iterator is currently accumulating text into.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Capture {
    #[default]
    None,
    /// Inside `<v>` — the raw value of the current cell.
    Value,
    /// Inside `<is><t>` — an inline string.
    InlineText,
}

/// The kind of `<c>` element being parsed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CellKind {
    /// `t` omitted or `n` — numeric.
    #[default]
    Number,
    /// `t="s"` — index into the shared strings table.
    Shared,
    /// `t="inlineStr"` — text inside the cell element.
    Inline,
    /// `t="b"` — boolean.
    Boolean,
    /// `t="e"` — an error value.
    Error,
    /// `t="str"` — cached formula string result.
    FormulaString,
}

/// In-progress state for the current `<c>` element.
#[derive(Debug, Default)]
struct CellState {
    kind: CellKind,
    /// Text accumulated from `<v>`.
    value: String,
    /// Text accumulated from `<is><t>`.
    inline: String,
    /// 0-based column from the cell's `r` attribute, when present.
    column: Option<usize>,
}

impl CellState {
    fn finish(self, shared: &[String]) -> Cell {
        let value = self.value.trim().to_owned();
        match self.kind {
            CellKind::Inline => Cell::String(self.inline),
            CellKind::Shared => shared
                .get(value.parse::<usize>().unwrap_or(usize::MAX))
                .cloned()
                .map_or(Cell::Blank, Cell::String),
            CellKind::Boolean => Cell::Boolean(value == "1" || value == "true"),
            CellKind::Error => Cell::Error(value),
            CellKind::FormulaString => Cell::String(value),
            CellKind::Number => {
                if value.is_empty() {
                    Cell::Blank
                } else {
                    value
                        .parse::<f64>()
                        .map_or_else(|_| Cell::String(value), Cell::Number)
                }
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
            Ok(None) => Err(DocError::invalid_format(format!(
                "malformed character reference `&{};`",
                r.as_ref()
            ))),
            Err(e) => Err(DocError::invalid_format(format!(
                "bad character reference: {e}"
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

/// Pull-based iterator over worksheet rows.
struct RowsIter<'r> {
    reader: quick_xml::Reader<&'r [u8]>,
    shared: &'r [String],
    /// 1-based index of the row currently being parsed.
    row_index: u32,
    /// Default index for the next row lacking an explicit `r` attribute.
    next_row_index: u32,
    row_cells: Vec<Cell>,
    cell: CellState,
    capture: Capture,
    done: bool,
}

impl RowsIter<'_> {
    fn attr(start: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
        start
            .attributes()
            .filter_map(std::result::Result::ok)
            .find(|a| a.key.as_ref() == name)
            .and_then(|a| {
                a.normalized_value(XML_VERSION)
                    .ok()
                    .map(std::borrow::Cow::into_owned)
            })
    }

    /// Place the finished cell at its declared column, padding any skipped
    /// columns with blanks. Cells without an `r` attribute are appended
    /// sequentially, which is what sparse writers expect.
    fn finish_cell(&mut self) {
        let state = std::mem::take(&mut self.cell);
        let column = state.column.unwrap_or(self.row_cells.len());
        let cell = state.finish(self.shared);
        if self.row_cells.len() < column {
            self.row_cells
                .resize_with(column, || Cell::Blank);
        }
        if column < self.row_cells.len() {
            // An out-of-order or duplicate reference: keep the first value
            // rather than shifting the row.
            return;
        }
        self.row_cells.push(cell);
    }
}

impl Iterator for RowsIter<'_> {
    type Item = Result<Row, DocError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let event = match self.reader.read_event() {
                Ok(event) => event,
                Err(e) => {
                    self.done = true;
                    return Some(Err(DocError::invalid_format(format!("XML error: {e}"))));
                }
            };
            match event {
                Event::Start(start) => match start.name().as_ref() {
                    "row" => {
                        let explicit = Self::attr(&start, "r").and_then(|r| r.parse::<u32>().ok());
                        self.row_index = explicit.unwrap_or(self.next_row_index);
                        self.next_row_index = self.row_index.saturating_add(1);
                    }
                    "c" => {
                        self.cell = CellState {
                            kind: match Self::attr(&start, "t").as_deref() {
                                Some("s") => CellKind::Shared,
                                Some("inlineStr") => CellKind::Inline,
                                Some("b") => CellKind::Boolean,
                                Some("e") => CellKind::Error,
                                Some("str") => CellKind::FormulaString,
                                _ => CellKind::Number,
                            },
                            column: Self::attr(&start, "r")
                                .as_deref()
                                .and_then(column_index),
                            ..CellState::default()
                        };
                    }
                    "v" => self.capture = Capture::Value,
                    "is" => self.capture = Capture::InlineText,
                    _ => {}
                },
                Event::Empty(start) => {
                    if start.name().as_ref() == "c" {
                        // A bare <c/> with no children is an empty cell.
                        self.finish_cell();
                    }
                }
                Event::Text(text) => {
                    if self.capture != Capture::None {
                        match quick_xml::escape::unescape(&text) {
                            Ok(chunk) => match self.capture {
                                Capture::Value => self.cell.value.push_str(&chunk),
                                Capture::InlineText => self.cell.inline.push_str(&chunk),
                                Capture::None => {}
                            },
                            Err(e) => {
                                self.done = true;
                                return Some(Err(DocError::invalid_format(format!(
                                    "invalid XML entities in cell text: {e}"
                                ))));
                            }
                        }
                    }
                }
                Event::GeneralRef(reference) => {
                    if self.capture != Capture::None {
                        match resolve_ref(&reference) {
                            Ok(chunk) => match self.capture {
                                Capture::Value => self.cell.value.push_str(&chunk),
                                Capture::InlineText => self.cell.inline.push_str(&chunk),
                                Capture::None => {}
                            },
                            Err(e) => {
                                self.done = true;
                                return Some(Err(e));
                            }
                        }
                    }
                }
                Event::End(end) => match end.name().as_ref() {
                    "v" | "is" => self.capture = Capture::None,
                    "c" => self.finish_cell(),
                    "row" => {
                        let cells = std::mem::take(&mut self.row_cells);
                        return Some(Ok(Row {
                            index: self.row_index,
                            cells,
                        }));
                    }
                    _ => {}
                },
                Event::Eof => {
                    self.done = true;
                    return None;
                }
                _ => {}
            }
        }
    }
}

fn read_entry(
    archive: &mut ZipArchive<std::io::Cursor<&[u8]>>,
    name: &str,
) -> Result<Vec<u8>, DocError> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| DocError::invalid_format(format!("missing ZIP entry `{name}`")))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Parse the shared strings table into an ordered `Vec<String>`.
///
/// Rich-text runs (`<r><t>`) inside a single `<si>` are concatenated, matching
/// how Excel presents them as one string.
fn read_shared_strings(
    archive: &mut ZipArchive<std::io::Cursor<&[u8]>>,
) -> Result<Vec<String>, DocError> {
    if archive.by_name("xl/sharedStrings.xml").is_err() {
        return Ok(Vec::new());
    }
    let bytes = read_entry(archive, "xl/sharedStrings.xml")?;
    let mut reader = quick_xml::Reader::from_reader(bytes.as_slice());
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_si = false;
    let mut in_t = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) => match start.name().as_ref() {
                "si" => {
                    in_si = true;
                    current.clear();
                }
                "t" if in_si => in_t = true,
                _ => {}
            },
            Ok(Event::Text(text)) if in_t => match quick_xml::escape::unescape(&text) {
                Ok(chunk) => current.push_str(&chunk),
                Err(e) => {
                    return Err(DocError::invalid_format(format!(
                        "invalid XML entities: {e}"
                    )));
                }
            },
            Ok(Event::GeneralRef(reference)) if in_t => {
                current.push_str(&resolve_ref(&reference)?);
            }
            Ok(Event::End(end)) => match end.name().as_ref() {
                "t" => in_t = false,
                "si" => strings.push(std::mem::take(&mut current)),
                _ => {}
            },
            Ok(Event::Eof) => return Ok(strings),
            Ok(_) => {}
            Err(e) => return Err(DocError::invalid_format(format!("XML error: {e}"))),
        }
    }
}

/// `.xlsx` workbook writer.
///
/// Emits a minimal, spec-compliant OOXML package: `[Content_Types].xml`,
/// package relationships, a single-sheet workbook, and one worksheet whose
/// cells use inline strings (no shared-strings round trip needed).
pub struct XlsxWriter {
    rows: Vec<Row>,
}

impl XlsxWriter {
    /// Create a new empty workbook.
    #[must_use]
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Append a row to the workbook.
    pub fn push_row(&mut self, row: Row) {
        self.rows.push(row);
    }

    /// Serialize the workbook to `.xlsx` bytes.
    ///
    /// # Errors
    /// Returns [`DocError`] on serialization failure.
    pub fn finish(self) -> Result<Vec<u8>, DocError> {
        let cursor = std::io::Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let sheet = build_sheet_xml(&self.rows)?;
        let parts: [(&str, &str); 5] = [
            ("[Content_Types].xml", CONTENT_TYPES_XML),
            ("_rels/.rels", ROOT_RELS_XML),
            ("xl/workbook.xml", WORKBOOK_XML),
            ("xl/_rels/workbook.xml.rels", WORKBOOK_RELS_XML),
            ("xl/worksheets/sheet1.xml", &sheet),
        ];
        for (name, content) in parts {
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

impl Default for XlsxWriter {
    fn default() -> Self {
        Self::new()
    }
}

const XML_DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

const CONTENT_TYPES_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
    r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
    r#"<Default Extension="xml" ContentType="application/xml"/>"#,
    r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
    r#"<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
    r#"</Types>"#
);

const ROOT_RELS_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
    r#"</Relationships>"#
);

const WORKBOOK_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    r#"<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>"#,
    r#"</workbook>"#
);

const WORKBOOK_RELS_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>"#,
    r#"</Relationships>"#
);

/// Parse the column index (0-based) from a cell reference such as `C5`.
///
/// Returns `None` when the reference has no leading letters, so cells written
/// without an `r` attribute fall back to sequential placement.
#[must_use]
pub(crate) fn column_index(reference: &str) -> Option<usize> {
    let letters: String = reference
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    if letters.is_empty() {
        return None;
    }
    let mut index = 0usize;
    for ch in letters.chars() {
        // Bijective base-26 with an implicit leading A: A=0, Z=25, AA=26.
        let digit = usize::from(ch.to_ascii_uppercase() as u8 - b'A') + 1;
        index = index.checked_mul(26)?.checked_add(digit)?;
    }
    Some(index - 1)
}

/// Column index (0-based) to spreadsheet letters: 0 → `A`, 26 → `AA`.
#[must_use]
pub fn column_name(mut column: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push(b'A' + u8::try_from(column % 26).unwrap_or(0));
        if column < 26 {
            break;
        }
        column = column / 26 - 1;
    }
    letters.reverse();
    String::from_utf8(letters).unwrap_or_default()
}

fn cell_ref(column: usize, row: u32) -> String {
    format!("{}{row}", column_name(column))
}

/// Render the worksheet part for `rows`.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if a string cell contains a character
/// that XML 1.0 forbids; emitting it raw would produce a workbook that Excel
/// refuses to open.
fn build_sheet_xml(rows: &[Row]) -> Result<String, DocError> {
    let mut xml = String::from(XML_DECL);
    xml.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    for row in rows {
        let _ = write!(xml, r#"<row r="{}">"#, row.index);
        for (column, cell) in row.cells.iter().enumerate() {
            let r = cell_ref(column, row.index);
            match cell {
                Cell::String(text) => {
                    let escaped = escape_xml_text(text)?;
                    let preserve = text.starts_with([' ', '\t']) || text.ends_with([' ', '\t']);
                    let _ = write!(
                        xml,
                        r#"<c r="{r}" t="inlineStr"><is><t{}>{escaped}</t></is></c>"#,
                        if preserve {
                            r#" xml:space="preserve""#
                        } else {
                            ""
                        }
                    );
                }
                Cell::Number(value) => {
                    let _ = write!(xml, r#"<c r="{r}"><v>{value}</v></c>"#);
                }
                Cell::Boolean(value) => {
                    let _ = write!(xml, r#"<c r="{r}" t="b"><v>{}</v></c>"#, u8::from(*value));
                }
                Cell::Error(value) => {
                    let escaped = escape_xml_text(value)?;
                    let _ = write!(xml, r#"<c r="{r}" t="e"><v>{escaped}</v></c>"#);
                }
                Cell::Blank => {
                    let _ = write!(xml, r#"<c r="{r}"/>"#);
                }
            }
        }
        xml.push_str("</row>");
    }
    xml.push_str("</sheetData></worksheet>");
    Ok(xml)
}

#[cfg(test)]
mod tests {
    use super::*;

        /// A sparse worksheet: row 1 skips column C, and row 3 has one cell in
    /// column C. Cells carry explicit `r` references naming their column.
    const SPARSE_SHEET: &str = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
        r#"<sheetData>"#,
        r#"<row r="1" spans="1:4">"#,
        r#"<c r="A1" t="inlineStr"><is><t>alpha</t></is></c>"#,
        r#"<c r="B1" t="inlineStr"><is><t>beta</t></is></c>"#,
        r#"<c r="D1" t="inlineStr"><is><t>delta</t></is></c>"#,
        r#"</row>"#,
        r#"<row>"#,
        r#"<c r="C3" t="inlineStr"><is><t>gamma</t></is></c>"#,
        r#"</row>"#,
        r#"</sheetData></worksheet>"#,
    );

    fn parse_sheet(xml: &str) -> Vec<Row> {
        let mut reader = XlsxReader {
            sheet: xml.as_bytes().to_vec(),
            shared_strings: Vec::new(),
        };
        reader.rows().collect::<Result<Vec<_>, _>>().expect("parse sheet")
    }

    #[test]
    fn sparse_row_fills_the_skipped_column_with_blank_cells() {
        let rows = parse_sheet(SPARSE_SHEET);

        assert_eq!(rows[0].cells.len(), 4, "A..D with the skipped C column");
        assert_eq!(rows[0].cells[0], Cell::String("alpha".to_owned()));
        assert_eq!(rows[0].cells[1], Cell::String("beta".to_owned()));
        assert_eq!(rows[0].cells[2], Cell::Blank, "skipped column C");
        assert_eq!(rows[0].cells[3], Cell::String("delta".to_owned()));
    }

    #[test]
    fn cell_reference_column_places_a_lone_cell_in_column_c() {
        let rows = parse_sheet(SPARSE_SHEET);

        // Without honouring `r="C3"`, gamma would be read as column A.
        assert_eq!(rows[1].cells.len(), 3);
        assert_eq!(rows[1].cells[0], Cell::Blank);
        assert_eq!(rows[1].cells[1], Cell::Blank);
        assert_eq!(rows[1].cells[2], Cell::String("gamma".to_owned()));
    }

    #[test]
    fn cells_without_references_keep_their_sequential_position() {
        let xml = concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<sheetData><row r="1">"#,
            r#"<c t="inlineStr"><is><t>one</t></is></c>"#,
            r#"<c t="inlineStr"><is><t>two</t></is></c>"#,
            r#"</row></sheetData></worksheet>"#,
        );
        let rows = parse_sheet(xml);
        assert_eq!(rows[0].cells.len(), 2);
        assert_eq!(rows[0].cells[0], Cell::String("one".to_owned()));
        assert_eq!(rows[0].cells[1], Cell::String("two".to_owned()));
    }

    #[test]
    fn column_index_parses_letter_references() {
        assert_eq!(column_index("A1"), Some(0));
        assert_eq!(column_index("B7"), Some(1));
        assert_eq!(column_index("C3"), Some(2));
        assert_eq!(column_index("Z9"), Some(25));
        assert_eq!(column_index("AA1"), Some(26));
        assert_eq!(column_index("AB2"), Some(27));
        assert_eq!(column_index("1"), None, "a reference needs letters");
        assert_eq!(column_index(""), None);
    }

    /// A string cell containing a control character cannot be represented in XML.
/// Writing it raw produces a workbook that conforming parsers reject, so the
/// writer must fail instead.
#[test]
fn illegal_control_characters_are_rejected_not_written_raw() {
    let rows = vec![Row {
        index: 1,
        cells: vec![Cell::String("bad\u{1}value".to_owned())],
    }];

    let err = build_sheet_xml(&rows).expect_err("U+0001 must be rejected");
    assert!(
        err.to_string().contains("U+0001"),
        "error should name the code point: {err}"
    );

    // The same rejection must surface from the public writer.
    let mut writer = XlsxWriter::new();
    writer.push_row(rows[0].clone());
    let err = writer.finish().expect_err("writer must refuse illegal XML");
    assert!(err.to_string().contains("U+0001"), "{err}");
}

#[test]
fn tab_newline_and_return_are_accepted_in_cells() {
    let rows = vec![Row {
        index: 1,
        cells: vec![Cell::String("a\tb\nc\rd".to_owned())],
    }];
    let xml = build_sheet_xml(&rows).expect("these three controls are legal XML");
    assert!(xml.contains("a\tb\nc\rd"));
}

#[test]
fn writer_round_trips_its_own_blank_cells() {
        let rows = vec![Row {
            index: 1,
            cells: vec![Cell::String("a".to_owned()), Cell::Blank, Cell::String("c".to_owned())],
        }];
        let xml = build_sheet_xml(&rows).expect("legal XML");
        let parsed = parse_sheet(&xml);
        assert_eq!(parsed[0].cells, rows[0].cells);
    }

    #[test]
fn column_names_progress_like_excel() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(27), "AB");
        assert_eq!(column_name(701), "ZZ");
        assert_eq!(column_name(702), "AAA");
    }

    #[test]
    fn sheet_xml_escapes_text_and_preserves_spacing() {
        let rows = vec![Row {
            index: 1,
            cells: vec![Cell::String(" <b> & ".to_owned()), Cell::Number(2.5)],
        }];
        let xml = build_sheet_xml(&rows).expect("legal XML");
        assert!(xml.contains(r#"<t xml:space="preserve"> &lt;b&gt; &amp; </t>"#));
        assert!(xml.contains(r#"<c r="B1"><v>2.5</v></c>"#));
    }

    #[test]
    fn writer_produces_expected_zip_entries() {
        let bytes = XlsxWriter::new().finish().expect("write");
        let cursor = std::io::Cursor::new(&bytes[..]);
        let mut archive = ZipArchive::new(cursor).expect("zip");
        for name in [
            "[Content_Types].xml",
            "_rels/.rels",
            "xl/workbook.xml",
            "xl/_rels/workbook.xml.rels",
            "xl/worksheets/sheet1.xml",
        ] {
            assert!(archive.by_name(name).is_ok(), "missing entry {name}");
        }
    }

    #[test]
    fn empty_cell_elements_read_back_as_blank() {
        let xml = concat!(
            "<?xml version=\"1.0\"?>",
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
            "<sheetData><row r=\"1\"><c r=\"A1\"/><c r=\"B1\"/></row></sheetData></worksheet>"
        );
        let mut reader = XlsxReader {
            sheet: xml.as_bytes().to_vec(),
            shared_strings: Vec::new(),
        };
        let rows: Vec<_> = reader.rows().collect::<Result<Vec<_>, _>>().expect("parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells, vec![Cell::Blank, Cell::Blank]);
    }

    #[test]
    fn cell_kinds_map_to_variants() {
        let xml = concat!(
            "<?xml version=\"1.0\"?>",
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
            "<sheetData>",
            "<row r=\"1\">",
            "<c r=\"A1\" t=\"inlineStr\"><is><t>text &amp; more</t></is></c>",
            "<c r=\"B1\" t=\"s\"><v>0</v></c>",
            "<c r=\"C1\" t=\"b\"><v>1</v></c>",
            "<c r=\"D1\" t=\"e\"><v>#REF!</v></c>",
            "<c r=\"E1\"><v>-12.5</v></c>",
            "<c r=\"F1\" t=\"str\"><v>formula result</v></c>",
            "</row>",
            "</sheetData></worksheet>"
        );
        let mut reader = XlsxReader {
            sheet: xml.as_bytes().to_vec(),
            shared_strings: vec!["from shared".to_owned()],
        };
        let rows: Vec<_> = reader.rows().collect::<Result<Vec<_>, _>>().expect("parse");
        assert_eq!(
            rows[0].cells,
            vec![
                Cell::String("text & more".to_owned()),
                Cell::String("from shared".to_owned()),
                Cell::Boolean(true),
                Cell::Error("#REF!".to_owned()),
                Cell::Number(-12.5),
                Cell::String("formula result".to_owned()),
            ]
        );
    }

    #[test]
    fn rows_without_r_attribute_are_numbered_sequentially() {
        let xml = concat!(
            "<?xml version=\"1.0\"?>",
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
            "<sheetData><row><c r=\"A1\"><v>1</v></c></row><row><c r=\"A2\"><v>2</v></c></row>",
            "</sheetData></worksheet>"
        );
        let mut reader = XlsxReader {
            sheet: xml.as_bytes().to_vec(),
            shared_strings: Vec::new(),
        };
        let rows: Vec<_> = reader.rows().collect::<Result<Vec<_>, _>>().expect("parse");
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[1].index, 2);
    }

    #[test]
    fn malformed_sheet_xml_is_an_error_not_a_panic() {
        let mut reader = XlsxReader {
            sheet: b"<sheetData><row".to_vec(),
            shared_strings: Vec::new(),
        };
        assert!(reader.rows().any(|r| r.is_err()));
    }
}

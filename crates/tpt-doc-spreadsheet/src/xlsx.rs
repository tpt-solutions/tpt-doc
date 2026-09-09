use tpt_doc_core::DocError;

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
#[derive(Debug, Clone)]
pub struct Row {
    /// 1-based row number.
    pub index: u32,
    /// Cells in column order.
    pub cells: Vec<Cell>,
}

impl Row {
    /// Return the cells in this row.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
}

/// Streaming `.xlsx` worksheet reader.
///
/// Parses the ZIP/XML structure of an `.xlsx` file and yields [`Row`] values
/// one at a time without loading the full workbook into memory.
pub struct XlsxReader<'a> {
    _data: &'a [u8],
}

impl<'a> XlsxReader<'a> {
    /// Create a new reader over the given `.xlsx` bytes.
    ///
    /// # Errors
    /// Returns [`DocError`] if the bytes are not a valid `.xlsx` (OOXML) file.
    pub fn new(_bytes: &'a [u8]) -> Result<Self, DocError> {
        // TODO(phase-1): implement ZIP entry streaming + quick-xml parsing
        Err(DocError::invalid_format("XlsxReader not yet implemented"))
    }

    /// Iterate over worksheet rows.
    pub fn rows(&mut self) -> impl Iterator<Item = Result<Row, DocError>> + '_ {
        core::iter::empty()
    }
}

/// `.xlsx` workbook writer.
pub struct XlsxWriter {
    rows: Vec<Row>,
}

impl XlsxWriter {
    /// Create a new empty workbook.
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
        // TODO(phase-1): implement OOXML ZIP assembly
        Err(DocError::invalid_format("XlsxWriter not yet implemented"))
    }
}

impl Default for XlsxWriter {
    fn default() -> Self {
        Self::new()
    }
}

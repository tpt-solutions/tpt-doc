use crate::paragraph::Paragraph;

/// Border style for table outlines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BorderStyle {
    /// No borders rendered.
    #[default]
    None,
    /// Single-line borders (emitted via the `TableGrid` style).
    Single,
    /// Double-line borders.
    Double,
}

/// A table cell: one or more paragraphs.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    paragraphs: Vec<Paragraph>,
}

impl Cell {
    /// Create a cell containing a single Normal paragraph.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            paragraphs: vec![Paragraph::new(text)],
        }
    }

    /// Append a paragraph to the cell.
    pub fn push_paragraph(&mut self, p: Paragraph) {
        self.paragraphs.push(p);
    }

    /// The cell's paragraphs, in order.
    #[must_use]
    pub fn paragraphs(&self) -> &[Paragraph] {
        &self.paragraphs
    }
}

/// A table row: an ordered sequence of cells.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    cells: Vec<Cell>,
}

impl Row {
    /// Create a row from the given cells.
    pub fn new(cells: impl IntoIterator<Item = Cell>) -> Self {
        Self {
            cells: cells.into_iter().collect(),
        }
    }

    /// Append a cell to the row.
    pub fn push_cell(&mut self, cell: Cell) {
        self.cells.push(cell);
    }

    /// The row's cells, in order.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
}

/// A table: rows of cells with a uniform border style and optional
/// fixed column widths.
#[derive(Debug, Clone, Default)]
pub struct Table {
    rows: Vec<Row>,
    /// Border style applied to the whole table.
    pub borders: BorderStyle,
    /// Fixed column widths in twentieths of a point (twips).
    pub column_widths_twips: Vec<u16>,
}

impl Table {
    /// Create an empty bordered table.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            borders: BorderStyle::Single,
            column_widths_twips: Vec::new(),
        }
    }

    /// Append a row to the table.
    pub fn push_row(&mut self, row: Row) {
        self.rows.push(row);
    }

    /// The table's rows, in order.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_holds_paragraphs() {
        let mut cell = Cell::new("first");
        cell.push_paragraph(Paragraph::new("second"));
        let texts: Vec<_> = cell
            .paragraphs()
            .iter()
            .map(Paragraph::plain_text)
            .collect();
        assert_eq!(texts, ["first", "second"]);
    }

    #[test]
    fn row_preserves_cell_order() {
        let row = Row::new([Cell::new("a"), Cell::new("b"), Cell::new("c")]);
        assert_eq!(row.cells().len(), 3);
        assert_eq!(row.cells()[1].paragraphs()[0].plain_text(), "b");
    }

    #[test]
    fn new_table_defaults_to_single_borders() {
        let mut table = Table::new();
        table.push_row(Row::new([Cell::new("a")]));
        assert_eq!(table.borders, BorderStyle::Single);
        assert_eq!(table.rows().len(), 1);
    }
}

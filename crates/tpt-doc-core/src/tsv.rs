//! A strict reader for the tab-separated tables behind the schema validators:
//! HL7 segment definitions, EDIFACT and X12 element tables, X12 transaction
//! sets.
//!
//! Blank lines and lines starting with `#` are skipped. Every other line is a
//! data row. A row that does not match its table is an error that names the
//! line, rather than being dropped silently, so a typo in a table you loaded
//! at runtime cannot quietly weaken validation.
//!
//! Structural checks belong here (column count, numbers, known codes).
//! Semantic checks, such as whether an element position is in range, stay in
//! the validators, where they are reported against the document.

use alloc::{format, string::String, vec::Vec};

use crate::error::DocError;
use crate::traits::Location;

/// One data row of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row<'a> {
    /// 1-based line number in the table text.
    pub line: usize,
    /// The tab-separated columns.
    pub columns: Vec<&'a str>,
}

impl Row<'_> {
    /// Fail unless the row has at least `count` columns.
    ///
    /// # Errors
    /// Returns an error naming the line when the row is too short.
    pub fn expect_columns(&self, count: usize) -> Result<(), DocError> {
        if self.columns.len() < count {
            return Err(self.error(format!(
                "expected {count} columns, found {}",
                self.columns.len()
            )));
        }
        Ok(())
    }

    /// Column `index` (0-based) parsed as an unsigned 16-bit number.
    ///
    /// # Errors
    /// Returns an error naming the line and column when it is not a number in
    /// range, or when the column is missing.
    pub fn parse_u16(&self, index: usize) -> Result<u16, DocError> {
        let text = self.columns.get(index).copied().unwrap_or_default();
        text.parse::<u16>().map_err(|_| {
            self.error(format!(
                "column {} must be a number, found `{text}`",
                index + 1
            ))
        })
    }

    /// An error that points at this row.
    #[must_use]
    pub fn error(&self, message: impl Into<String>) -> DocError {
        DocError::InvalidFormat(message.into()).with_location(Location::Index(self.line))
    }
}

/// The data rows of a table, in order.
pub fn rows(text: &str) -> impl Iterator<Item = Row<'_>> {
    text.lines().enumerate().filter_map(|(index, raw)| {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        Some(Row {
            line: index + 1,
            columns: trimmed.split('\t').collect(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_blank_lines_and_comments_and_numbers_lines_from_one() {
        let text = "# header\n\nA\t1\n# note\nB\t2\n";
        let lines: Vec<usize> = rows(text).map(|r| r.line).collect();
        assert_eq!(lines, [3, 5]);
    }

    #[test]
    fn short_row_names_its_line() {
        let row = rows("A\t1\n").next().expect("row");
        let err = row.expect_columns(3).expect_err("too short");
        assert!(err.to_string().contains("line") || err.to_string().contains("columns"));
        assert_eq!(err.location(), Some(&Location::Index(1)));
    }

    #[test]
    fn non_numeric_column_is_an_error_at_its_line() {
        let row = rows("A\tx\n").next().expect("row");
        assert!(row.parse_u16(1).is_err());
    }
}

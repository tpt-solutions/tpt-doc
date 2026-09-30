/// A PDF cross-reference table entry.
#[derive(Debug, Clone, Copy)]
pub struct XrefEntry {
    /// Byte offset of the object from the start of the file.
    pub offset: u64,
    /// Generation number (almost always 0 for new documents).
    pub generation: u16,
    /// Whether the entry is in-use (`true`) or free (`false`).
    pub in_use: bool,
}

/// A cross-reference table for a PDF document.
///
/// Tracks the byte offset of every indirect object so PDF readers can
/// seek directly to any object without scanning the whole file.
#[derive(Debug, Default)]
pub struct XrefTable {
    entries: Vec<XrefEntry>,
}

impl XrefTable {
    /// Create a new xref table whose entry 0 is the free-list head
    /// (`0000000000 65535 f`), as the PDF specification requires.
    #[must_use]
    pub fn new() -> Self {
        let mut table = Self::default();
        table.entries.push(XrefEntry {
            offset: 0,
            generation: 65535,
            in_use: false,
        });
        table
    }

    /// Register the byte offset of the next object and return its object
    /// number (the first real object is number 1).
    pub fn push(&mut self, offset: u64) -> u32 {
        let id = u32::try_from(self.entries.len()).unwrap_or(u32::MAX);
        self.entries.push(XrefEntry {
            offset,
            generation: 0,
            in_use: true,
        });
        id
    }

    /// Serialize the xref section to PDF bytes.
    ///
    /// Each entry is exactly 20 bytes: `nnnnnnnnnn ggggg n \r\n`
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.entries.len() * 20 + 32);
        out.extend_from_slice(b"xref\n");
        out.extend_from_slice(format!("0 {}\n", self.entries.len()).as_bytes());
        for entry in &self.entries {
            let line = format!(
                "{:010} {:05} {}\r\n",
                entry.offset,
                entry.generation,
                if entry.in_use { 'n' } else { 'f' }
            );
            out.extend_from_slice(line.as_bytes());
        }
        out
    }

    /// Number of entries in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_byte_exact_20_bytes() {
        let mut table = XrefTable::new();
        table.push(0);
        table.push(100);
        let serialized = table.serialize();
        // "xref\n" + "0 3\n" + 3 * 20 bytes (including the free head).
        let entries_start = serialized.len() - 60;
        assert_eq!(&serialized[..5], b"xref\n");
        assert_eq!(&serialized[5..8], b"0 3");
        let entry = |index: usize| {
            std::str::from_utf8(
                &serialized[entries_start + index * 20..entries_start + (index + 1) * 20],
            )
            .expect("ASCII entry")
        };
        assert_eq!(entry(0), "0000000000 65535 f\r\n");
        assert_eq!(entry(1), "0000000000 00000 n\r\n");
        assert_eq!(entry(2), "0000000100 00000 n\r\n");
    }

    #[test]
    fn object_numbers_start_at_one() {
        let mut table = XrefTable::new();
        let first = table.push(42);
        assert_eq!(first, 1);
    }
}

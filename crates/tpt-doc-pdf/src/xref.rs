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
    /// Create a new, empty xref table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the byte offset of the next object and return its object number.
    pub fn push(&mut self, offset: u64) -> u32 {
        let id = self.entries.len() as u32;
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
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.entries.len() * 20 + 32);
        out.extend_from_slice(b"xref\n");
        out.extend_from_slice(
            format!("0 {}\n", self.entries.len()).as_bytes(),
        );
        for entry in &self.entries {
            let line = format!(
                "{:010} {:05} {} \r\n",
                entry.offset,
                entry.generation,
                if entry.in_use { 'n' } else { 'f' }
            );
            out.extend_from_slice(line.as_bytes());
        }
        out
    }

    /// Number of entries in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_is_20_bytes() {
        let mut table = XrefTable::new();
        table.push(0);
        table.push(100);
        let serialized = table.serialize();
        // "xref\n" + "0 2\n" + 2 * 20 bytes
        let header = b"xref\n0 2\n";
        let entries_start = serialized.len() - 40;
        let entry0 = &serialized[entries_start..entries_start + 20];
        assert_eq!(entry0.len(), 20);
    }
}

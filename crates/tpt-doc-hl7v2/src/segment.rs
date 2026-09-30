/// The five HL7 v2.x encoding characters (MSH-1/MSH-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delimiters {
    /// Field separator (MSH-1, default `|`).
    pub field: u8,
    /// Component separator (default `^`).
    pub component: u8,
    /// Repetition separator (default `~`).
    pub repeat: u8,
    /// Escape character (default `\`).
    pub escape: u8,
    /// Subcomponent separator (default `&`).
    pub subcomponent: u8,
}

impl Default for Delimiters {
    fn default() -> Self {
        Self {
            field: b'|',
            component: b'^',
            repeat: b'~',
            escape: b'\\',
            subcomponent: b'&',
        }
    }
}

/// A single HL7 v2.x segment, borrowing field data from the original input.
///
/// Fields are addressed 1-based per the HL7 standard: for non-MSH segments
/// `field(1)` is the first field after the segment tag. For `MSH` segments
/// the standard numbering is preserved — `field(1)` is the field separator
/// itself and `field(2)` holds the encoding characters — so `field(3)` on a
/// message read with default delimiters returns the sending application.
#[derive(Debug, Clone)]
pub struct Segment<'a> {
    tag: &'a str,
    /// Raw field values (components and repeats still joined).
    /// Index `i` holds field `i + 1`.
    fields: Vec<&'a str>,
    delims: Delimiters,
}

impl<'a> Segment<'a> {
    /// Construct a segment from its tag, raw field values, and delimiters.
    ///
    /// `fields[i]` is field `i + 1`; for `MSH` segments pass the field
    /// separator as the first field to preserve standard numbering.
    #[must_use]
    pub fn new(tag: &'a str, fields: Vec<&'a str>, delims: Delimiters) -> Self {
        Self {
            tag,
            fields,
            delims,
        }
    }

    /// The three-character segment tag (e.g. `"MSH"`, `"PID"`, `"OBX"`).
    #[must_use]
    pub fn tag(&self) -> &str {
        self.tag
    }

    /// The raw field values, field `n` at index `n - 1`.
    #[must_use]
    pub fn raw_fields(&self) -> &[&'a str] {
        &self.fields
    }

    /// The delimiters this segment was parsed with.
    #[must_use]
    pub fn delimiters(&self) -> &Delimiters {
        &self.delims
    }

    /// Field `n` (1-based, HL7 numbering including MSH-1/MSH-2).
    #[must_use]
    pub fn field(&self, n: usize) -> Option<&'a str> {
        if n == 0 {
            return None;
        }
        self.fields.get(n - 1).copied()
    }

    /// Component `m` of field `n` (both 1-based).
    #[must_use]
    pub fn component(&self, n: usize, m: usize) -> Option<&'a str> {
        let comp_sep = char::from(self.delims.component);
        self.field(n)?.split(comp_sep).nth(m - 1)
    }

    /// Repetition `r` of field `n` (both 1-based).
    #[must_use]
    pub fn repeat(&self, n: usize, r: usize) -> Option<&'a str> {
        let repeat_sep = char::from(self.delims.repeat);
        self.field(n)?.split(repeat_sep).nth(r - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid_segment() -> Segment<'static> {
        static RAW: &str = "PID|1||12345^^^NZHPI^MR||Smith^John";
        let fields: Vec<&str> = RAW.split('|').skip(1).collect();
        Segment::new("PID", fields, Delimiters::default())
    }

    #[test]
    fn field_access_is_one_based() {
        let seg = pid_segment();
        assert_eq!(seg.field(1), Some("1"));
        assert_eq!(seg.field(3), Some("12345^^^NZHPI^MR"));
        assert_eq!(seg.field(0), None);
        assert_eq!(seg.field(99), None);
    }

    #[test]
    fn component_access_splits_on_caret() {
        let seg = pid_segment();
        assert_eq!(seg.component(3, 1), Some("12345"));
        assert_eq!(seg.component(3, 4), Some("NZHPI"));
        assert_eq!(seg.component(5, 2), Some("John"));
        assert_eq!(seg.component(5, 3), None);
    }

    #[test]
    fn repeat_access_splits_on_tilde() {
        static RAW: &str = "OBX|1|ST|GPL^Glucose||90~110";
        let fields: Vec<&str> = RAW.split('|').skip(1).collect();
        let seg = Segment::new("OBX", fields, Delimiters::default());
        assert_eq!(seg.repeat(5, 1), Some("90"));
        assert_eq!(seg.repeat(5, 2), Some("110"));
    }
}

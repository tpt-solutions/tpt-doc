use tpt_doc_core::DocError;

/// A single EDIFACT segment, borrowing data from the original input.
#[derive(Debug, Clone)]
pub struct Segment<'a> {
    tag: &'a str,
    elements: Vec<&'a str>,
}

impl<'a> Segment<'a> {
    /// The three-character segment tag (e.g. `"UNB"`, `"LIN"`).
    #[must_use]
    pub fn tag(&self) -> &str {
        self.tag
    }

    /// The data elements of the segment, in order.
    #[must_use]
    pub fn elements(&self) -> &[&'a str] {
        &self.elements
    }
}

/// Streaming EDIFACT message parser.
///
/// Yields [`Segment`] values borrowed from the input byte slice.
pub struct EdifactParser<'a> {
    input: &'a [u8],
    pos: usize,
    /// Segment terminator (default `'`).
    seg_term: u8,
    /// Element separator (default `+`).
    elem_sep: u8,
    /// Component data element separator (default `:`).
    comp_sep: u8,
    /// Release character (default `?`).
    release: u8,
}

impl<'a> EdifactParser<'a> {
    /// Create a new parser with EDIFACT default delimiters.
    ///
    /// If the message starts with a `UNA` service string advice, delimiters
    /// are read from it automatically.
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        let mut parser = Self {
            input,
            pos: 0,
            seg_term: b'\'',
            elem_sep: b'+',
            comp_sep: b':',
            release: b'?',
        };
        parser.try_parse_una();
        parser
    }

    /// Attempt to read a UNA service string advice at the start of the input.
    fn try_parse_una(&mut self) {
        if self.input.len() >= 9 && &self.input[..3] == b"UNA" {
            // UNA:+.? ' — positions 3-8 are the six service characters
            self.comp_sep = self.input[3];
            self.elem_sep = self.input[4];
            // pos 5 is decimal mark (ignored for delimiter purposes)
            self.release = self.input[6];
            // pos 7 is reserved
            self.seg_term = self.input[8];
            self.pos = 9;
            // Skip optional whitespace after UNA
            while self.pos < self.input.len() && self.input[self.pos] == b'\n' {
                self.pos += 1;
            }
        }
    }

    /// Parse the next segment from the input.
    fn next_segment(&mut self) -> Option<Result<Segment<'a>, DocError>> {
        if self.pos >= self.input.len() {
            return None;
        }

        // Find the segment terminator
        let start = self.pos;
        let end = memchr::memchr(self.seg_term, &self.input[start..])?;
        let seg_bytes = &self.input[start..start + end];
        self.pos = start + end + 1;

        // Skip leading whitespace / newlines between segments
        while self.pos < self.input.len()
            && (self.input[self.pos] == b'\n' || self.input[self.pos] == b'\r')
        {
            self.pos += 1;
        }

        // Convert to str (EDIFACT is typically ASCII/ISO 8859)
        let Ok(seg_str) = std::str::from_utf8(seg_bytes) else {
            return Some(Err(DocError::invalid_format(
                "segment contains non-UTF-8 bytes",
            )));
        };

        if seg_str.is_empty() {
            return self.next_segment();
        }

        // Split on element separator
        let elem_sep = self.elem_sep as char;
        let mut parts = seg_str.splitn(64, elem_sep);
        let tag = match parts.next() {
            Some(t) if !t.is_empty() => t,
            _ => return Some(Err(DocError::invalid_format("empty segment tag"))),
        };

        let elements: Vec<&str> = parts.collect();
        Some(Ok(Segment { tag, elements }))
    }
}

impl<'a> Iterator for EdifactParser<'a> {
    type Item = Result<Segment<'a>, DocError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_segment()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_segments() {
        let msg =
            b"UNB+UNOA:1+SENDER+RECEIVER+200101:0900+1'UNH+1+ORDERS:D:96A:UN'UNT+2+1'UNZ+1+1'";
        let parser = EdifactParser::new(msg);
        let segs: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].tag(), "UNB");
        assert_eq!(segs[1].tag(), "UNH");
    }

    #[test]
    fn parse_with_una() {
        let msg = b"UNA:+.? 'UNB+UNOA:1+A+B+200101:0900+1'UNZ+1+1'";
        let parser = EdifactParser::new(msg);
        let segs: Vec<_> = parser.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(segs[0].tag(), "UNB");
    }
}

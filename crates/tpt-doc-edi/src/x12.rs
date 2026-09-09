use tpt_doc_core::DocError;

/// A single X12 segment.
#[derive(Debug, Clone)]
pub struct X12Segment<'a> {
    tag: &'a str,
    elements: Vec<&'a str>,
}

impl<'a> X12Segment<'a> {
    /// The segment identifier (e.g. `"ISA"`, `"GS"`, `"ST"`).
    pub fn tag(&self) -> &str {
        self.tag
    }

    /// The data elements of the segment (after the tag), in order.
    pub fn elements(&self) -> &[&'a str] {
        &self.elements
    }
}

/// Streaming X12 interchange parser.
///
/// The element separator and segment terminator are read from the ISA envelope.
pub struct X12Parser<'a> {
    input: &'a [u8],
    pos: usize,
    elem_sep: u8,
    seg_term: u8,
}

impl<'a> X12Parser<'a> {
    /// Create a parser, inferring delimiters from the ISA header.
    ///
    /// # Errors
    /// Returns [`DocError`] if the input does not begin with a valid ISA segment.
    pub fn new(input: &'a [u8]) -> Result<Self, DocError> {
        // ISA is always 106 bytes with fixed positions
        if input.len() < 106 {
            return Err(DocError::invalid_format("X12 input too short for ISA header"));
        }
        if &input[..3] != b"ISA" {
            return Err(DocError::invalid_format("X12 input must start with ISA"));
        }
        let elem_sep = input[3];
        let seg_term = input[105];

        Ok(Self {
            input,
            pos: 0,
            elem_sep,
            seg_term,
        })
    }
}

impl<'a> Iterator for X12Parser<'a> {
    type Item = Result<X12Segment<'a>, DocError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }

        let start = self.pos;
        let end = memchr::memchr(self.seg_term, &self.input[start..])?;
        let seg_bytes = &self.input[start..start + end];
        self.pos = start + end + 1;

        // Skip newlines between segments
        while self.pos < self.input.len()
            && (self.input[self.pos] == b'\n' || self.input[self.pos] == b'\r')
        {
            self.pos += 1;
        }

        let seg_str = match std::str::from_utf8(seg_bytes) {
            Ok(s) => s.trim(),
            Err(_) => {
                return Some(Err(DocError::invalid_format(
                    "segment contains non-UTF-8 bytes",
                )))
            }
        };

        if seg_str.is_empty() {
            return self.next();
        }

        let elem_sep = self.elem_sep as char;
        let mut parts = seg_str.splitn(64, elem_sep);
        let tag = match parts.next() {
            Some(t) if !t.is_empty() => t,
            _ => return Some(Err(DocError::invalid_format("empty segment tag"))),
        };

        Some(Ok(X12Segment {
            tag,
            elements: parts.collect(),
        }))
    }
}

use tpt_doc_core::DocError;

/// A single EDIFACT segment, borrowing data from the original input.
#[derive(Debug, Clone)]
pub struct Segment<'a> {
    tag: &'a str,
    elements: Vec<&'a str>,
    /// The release character in force for this segment (`?` by default).
    release: char,
}

impl<'a> Segment<'a> {
    /// The three-character segment tag (e.g. `"UNB"`, `"LIN"`).
    #[must_use]
    pub fn tag(&self) -> &str {
        self.tag
    }

    /// The data elements of the segment, in order.
    ///
    /// These are the raw slices, with any release sequences still present.
    /// Use [`Segment::elements_released`] to resolve them.
    #[must_use]
    pub fn elements(&self) -> &[&'a str] {
        &self.elements
    }

    /// The data elements with EDIFACT release sequences resolved.
    ///
    /// A release character followed by a separator (`+`, `:`, `?`, `'`, or a
    /// newline) yields that separator as literal data, so `A?+B` reads as
    /// `A+B`. A doubled release character (`??`) yields a literal `?`. Per
    /// UN/EDIFACT ISO 9735, a release character followed by anything else is
    /// dropped and the following character is kept.
    ///
    /// This allocates, unlike [`Segment::elements`], because resolving a
    /// release sequence generally produces a new string.
    #[must_use]
    pub fn elements_released(&self) -> Vec<std::borrow::Cow<'a, str>> {
        self.elements
            .iter()
            .map(|e| unescape(e, self.release))
            .collect()
    }
}

/// Resolve EDIFACT release sequences in `value`.
///
/// Returns a borrowed `Cow` when no release character is present, which is the
/// overwhelmingly common case.
fn unescape(value: &str, release: char) -> std::borrow::Cow<'_, str> {
    if !value.contains(release) {
        return std::borrow::Cow::Borrowed(value);
    }
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != release {
            out.push(ch);
            continue;
        }
        // A release character always consumes the next character, whatever it
        // is; an unrecognised one is simply dropped.
        if let Some(next) = chars.next() {
            out.push(next);
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Split one segment into its tag and data elements.
///
/// A `release` character immediately before the element separator makes it
/// literal data, so `A?+B` yields the single element `A+B`. The returned
/// slices are still views into `segment`; release sequences remain in place
/// and are resolved by [`Segment::elements_released`].
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] for an empty tag, or when the segment
/// carries more than `MAX_ELEMENTS` data elements.
fn split_elements(
    segment: &str,
    elem_sep: char,
    release: char,
) -> Result<(&str, Vec<&str>), DocError> {
    const MAX_ELEMENTS: usize = 128;

    let mut parts: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let chars = segment.char_indices().peekable();

    for (index, ch) in chars {
        if ch != elem_sep {
            continue;
        }
        // A separator released by the preceding character is literal data, so
        // this is not a boundary; leave the pair in the slice.
        let released = segment[..index].ends_with(release)
            && segment[..index]
                .chars()
                .rev()
                .take_while(|c| *c == release)
                .count()
                % 2
                == 1;
        if released {
            continue;
        }
        parts.push(&segment[start..index]);
        start = index + ch.len_utf8();
    }
    parts.push(&segment[start..]);

    let mut parts = parts.into_iter();
    let tag = match parts.next() {
        Some(t) if !t.is_empty() => t,
        _ => return Err(DocError::invalid_format("empty segment tag")),
    };
    let elements: Vec<&str> = parts.collect();
    if elements.len() > MAX_ELEMENTS {
        return Err(DocError::invalid_format(format!(
            "segment {tag} has {} data elements, exceeding the {MAX_ELEMENTS} limit",
            elements.len()
        )));
    }
    Ok((tag, elements))
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
        // Loop rather than recurse: a message containing a long run of empty
        // segments would otherwise overflow the stack.
        loop {
            if self.pos >= self.input.len() {
                return None;
            }

            let start = self.pos;
            // Find the segment terminator. A trailing segment with no terminator
            // is a truncated message, which must be reported rather than
            // silently discarding the data after the last `'`.
            let Some(end) = memchr::memchr(self.seg_term, &self.input[start..]) else {
                let remainder = self.input.len() - start;
                // Exhaust the parser: without advancing, a caller that keeps
                // iterating would receive this same error forever.
                self.pos = self.input.len();
                return Some(Err(DocError::invalid_format(format!(
                    "unterminated segment ({remainder} trailing bytes, no terminator)"
                ))));
            };
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
                continue;
            }

            // Split on the element separator, but only where the separator is not
            // released. Splitting first and unescaping afterwards would cut
            // `A?+B` into two elements, so the split walks the string and
            // treats `?<sep>` as literal data.
            let elem_sep = self.elem_sep as char;
            let release = self.release as char;

            let (tag, elements) = match split_elements(seg_str, elem_sep, release) {
                Ok(parts) => parts,
                Err(e) => return Some(Err(e)),
            };
            return Some(Ok(Segment {
                tag,
                elements,
                release,
            }));
        }
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

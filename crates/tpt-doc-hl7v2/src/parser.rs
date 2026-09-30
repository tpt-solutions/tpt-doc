use tpt_doc_core::DocError;

use crate::segment::{Delimiters, Segment};

/// Streaming HL7 v2.x message parser.
///
/// Yields [`Segment`] values borrowed from the input `&[u8]` — no heap
/// allocation beyond the field-index vector per segment. Segments are
/// separated by `\r`. If the message starts with an `MSH` segment, the
/// encoding characters in MSH-2 override the default delimiters for the
/// whole message.
pub struct Hl7Parser<'a> {
    input: &'a [u8],
    pos: usize,
    delims: Delimiters,
    /// Whether `delims` was set explicitly or read from an MSH already.
    delims_locked: bool,
}

impl<'a> Hl7Parser<'a> {
    /// Create a parser with HL7 default delimiters (`|^~\&`).
    ///
    /// If the first segment is `MSH`, delimiters are re-read from
    /// MSH-1/MSH-2 automatically.
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            delims: Delimiters::default(),
            delims_locked: false,
        }
    }

    /// Create a parser with explicit delimiters, ignoring MSH-2.
    #[must_use]
    pub fn with_delimiters(input: &'a [u8], delims: Delimiters) -> Self {
        Self {
            input,
            pos: 0,
            delims,
            delims_locked: true,
        }
    }

    /// Read encoding characters from an `MSH` segment: MSH-1 is the field
    /// separator, MSH-2 holds the other four in the order
    /// component, repeat, escape, subcomponent.
    fn read_delims_from_msh(seg_bytes: &[u8]) -> Result<Delimiters, DocError> {
        if seg_bytes.len() < 4 {
            return Err(DocError::invalid_format("MSH segment too short"));
        }
        let field = seg_bytes[3];
        let enc = seg_bytes
            .get(4..)
            .and_then(|rest| {
                let end = rest.iter().position(|&b| b == field)?;
                rest.get(..end)
            })
            .ok_or_else(|| DocError::invalid_format("MSH-2 not terminated by field separator"))?;
        if enc.len() < 4 {
            return Err(DocError::invalid_format(
                "MSH-2 must contain four encoding characters",
            ));
        }
        Ok(Delimiters {
            field,
            component: enc[0],
            repeat: enc[1],
            escape: enc[2],
            subcomponent: enc[3],
        })
    }

    /// Parse the next segment from the input.
    fn next_segment(&mut self) -> Option<Result<Segment<'a>, DocError>> {
        if self.pos >= self.input.len() {
            return None;
        }

        let start = self.pos;
        let input = self.input;
        let end = match memchr::memchr(b'\r', &input[start..]) {
            Some(rel) => start + rel,
            None => input.len(),
        };
        let seg_bytes = &input[start..end];
        self.pos = end + 1;

        let Ok(seg_str) = core::str::from_utf8(seg_bytes) else {
            return Some(Err(DocError::invalid_format(
                "segment contains non-UTF-8 bytes",
            )));
        };

        if seg_str.is_empty() {
            return self.next_segment();
        }

        // Read delimiters from the message header unless set explicitly.
        if !self.delims_locked && seg_str.as_bytes().starts_with(b"MSH") {
            match Self::read_delims_from_msh(seg_bytes) {
                Ok(d) => self.delims = d,
                Err(e) => return Some(Err(e)),
            }
            self.delims_locked = true;
        }

        let field_sep = char::from(self.delims.field);
        let mut parts = seg_str.splitn(256, field_sep);
        let tag = match parts.next() {
            Some(t) if !t.is_empty() => t,
            _ => return Some(Err(DocError::invalid_format("empty segment tag"))),
        };

        let fields: Vec<&str> = if tag == "MSH" {
            // Preserve standard MSH numbering: MSH-1 is the field separator.
            let Ok(sep_field) = core::str::from_utf8(&seg_bytes[3..4]) else {
                return Some(Err(DocError::invalid_format(
                    "MSH-1 is not a valid UTF-8 character",
                )));
            };
            let mut fields = Vec::with_capacity(1 + seg_str.matches(field_sep).count());
            fields.push(sep_field);
            fields.extend(parts);
            fields
        } else {
            parts.collect()
        };

        Some(Ok(Segment::new(tag, fields, self.delims)))
    }
}

impl<'a> Iterator for Hl7Parser<'a> {
    type Item = Result<Segment<'a>, DocError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_segment()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(input: &[u8]) -> Vec<Segment<'_>> {
        Hl7Parser::new(input)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse error")
    }

    const ADT: &str = "MSH|^~\\&|HIS|ACME|RIS|ACME|20261001120000||ADT^A01|MSG00001|P|2.5.1\rEVN|A01|20261001120000\rPID|1||12345^^^NZHPI^MR||Smith^John||19700101";

    #[test]
    fn parse_message_yields_all_segments() {
        let segs = collect(ADT.as_bytes());
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].tag(), "MSH");
        assert_eq!(segs[1].tag(), "EVN");
        assert_eq!(segs[2].tag(), "PID");
    }

    #[test]
    fn msh_fields_follow_standard_numbering() {
        let segs = collect(ADT.as_bytes());
        let msh = &segs[0];
        assert_eq!(msh.field(1), Some("|"));
        assert_eq!(msh.field(2), Some("^~\\&"));
        assert_eq!(msh.field(3), Some("HIS"));
        assert_eq!(msh.component(9, 1), Some("ADT"));
        assert_eq!(msh.component(9, 2), Some("A01"));
        assert_eq!(msh.field(12), Some("2.5.1"));
    }

    #[test]
    fn non_msh_fields_start_after_tag() {
        let segs = collect(ADT.as_bytes());
        let pid = &segs[2];
        assert_eq!(pid.field(3), Some("12345^^^NZHPI^MR"));
        assert_eq!(pid.component(5, 2), Some("John"));
    }

    #[test]
    fn reads_custom_delimiters_from_msh2() {
        // Non-default: field `!`, component `^`, repeat `*`, escape `?`, subcomponent `,`
        let msg = "MSH!^*?,&!HIS!ACME\rPID!1!!A^B!C*D";
        let segs = collect(msg.as_bytes());
        assert_eq!(segs.len(), 2);
        let pid = &segs[1];
        assert_eq!(pid.field(3), Some("A^B"));
        assert_eq!(pid.component(3, 2), Some("B"));
        assert_eq!(pid.repeat(4, 2), Some("D"));
        assert_eq!(segs[0].field(3), Some("HIS"));
    }

    #[test]
    fn with_delimiters_overrides_msh2() {
        let msg = "MSH|^~\\&|HIS\rPID|1||X";
        let segs: Vec<_> = Hl7Parser::with_delimiters(msg.as_bytes(), Delimiters::default())
            .collect::<Result<Vec<_>, _>>()
            .expect("parse error");
        assert_eq!(segs[1].field(3), Some("X"));
    }

    #[test]
    fn skips_empty_segments_and_tolerates_missing_trailing_cr() {
        let msg = "\rMSH|^~\\&|A\r\rPID|1";
        let segs = collect(msg.as_bytes());
        assert_eq!(segs.len(), 2);
    }

    #[test]
    fn non_utf8_input_is_an_error_not_a_panic() {
        let msg = b"MSH|^~\x80&|A\rPID|1";
        let mut iter = Hl7Parser::new(msg);
        assert!(iter.next().is_some_and(|r| r.is_err()));
    }
}

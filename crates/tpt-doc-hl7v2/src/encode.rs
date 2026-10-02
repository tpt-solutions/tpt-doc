use crate::segment::Delimiters;

/// An owned segment queued in a [`Message`].
///
/// `fields[i]` holds field `i + 1`. For the `MSH` segment, `fields[0]` is
/// MSH-2 (the encoding characters); MSH-1 (the field separator) is implicit
/// and re-emitted during serialization.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageSegment {
    /// Three-character segment tag.
    pub tag: String,
    /// Field values, field `n` at index `n - 1`.
    pub fields: Vec<String>,
}

/// HL7 v2.x message builder and encoder.
///
/// Construct with [`Message::msh`] to create the required message header,
/// append segments with [`push_segment`](Self::push_segment), then serialize
/// with [`to_bytes`](Self::to_bytes) into `\r`-delimited bytes.
#[derive(Debug, Clone, Default)]
pub struct Message {
    segments: Vec<MessageSegment>,
    delims: Delimiters,
}

impl Message {
    /// Create a message with an `MSH` header (v2.5.1, processing ID `P`).
    ///
    /// `message_type` is MSH-9 (e.g. `"ADT^A01"`), `control_id` is MSH-10,
    /// and `datetime` is MSH-7 in HL7 TS format (e.g. `"20261001120000"`).
    #[must_use]
    pub fn msh(
        sending_app: &str,
        sending_facility: &str,
        receiving_app: &str,
        receiving_facility: &str,
        datetime: &str,
        message_type: &str,
        control_id: &str,
    ) -> Self {
        Self::msh_with_version(
            sending_app,
            sending_facility,
            receiving_app,
            receiving_facility,
            datetime,
            message_type,
            control_id,
            "2.5.1",
        )
    }

    /// Like [`Message::msh`] but with an explicit version ID (MSH-12).
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn msh_with_version(
        sending_app: &str,
        sending_facility: &str,
        receiving_app: &str,
        receiving_facility: &str,
        datetime: &str,
        message_type: &str,
        control_id: &str,
        version: &str,
    ) -> Self {
        let fields = vec![
            "^~\\&".to_owned(),            // MSH-2 encoding characters
            sending_app.to_owned(),        // MSH-3
            sending_facility.to_owned(),   // MSH-4
            receiving_app.to_owned(),      // MSH-5
            receiving_facility.to_owned(), // MSH-6
            datetime.to_owned(),           // MSH-7
            String::new(),                 // MSH-8 security
            message_type.to_owned(),       // MSH-9
            control_id.to_owned(),         // MSH-10
            "P".to_owned(),                // MSH-11 processing ID
            version.to_owned(),            // MSH-12
        ];
        Self {
            segments: vec![MessageSegment {
                tag: "MSH".to_owned(),
                fields,
            }],
            delims: Delimiters::default(),
        }
    }

    /// Append a segment with its fields (field 1 first, tag excluded).
    pub fn push_segment(&mut self, tag: impl Into<String>, fields: &[&str]) {
        self.segments.push(MessageSegment {
            tag: tag.into(),
            fields: fields.iter().map(|f| (*f).to_owned()).collect(),
        });
    }

    /// The queued segments, in order.
    #[must_use]
    pub fn segments(&self) -> &[MessageSegment] {
        &self.segments
    }

/// Serialize the message to `\r`-terminated, `\r`-delimited bytes.
    ///
    /// Field values are escaped via [`escape_value`], so a delimiter inside
    /// data cannot corrupt the message structure. MSH-2 is left verbatim
    /// because it *is* the encoding-characters field.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let sep = self.delims.field as char;
        let mut out = Vec::new();
        for seg in &self.segments {
            out.extend_from_slice(seg.tag.as_bytes());
            out.push(self.delims.field);
            let is_msh = seg.tag == "MSH";
            let fields: Vec<String> = seg
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    // MSH-2 is the encoding-characters field itself and must
                    // be written verbatim.
                    if is_msh && index == 0 {
                        field.clone()
                    } else {
                        escape_value(field, self.delims)
                    }
                })
                .collect();
            out.extend_from_slice(fields.join(&sep.to_string()).as_bytes());
            out.push(b'\r');
        }
        out
    }
}

/// Escape a field value for output, applying the HL7 escape sequences.
///
/// Only characters that would break the *envelope* are escaped: the segment
/// terminator `\r`, the field separator, and the escape character itself.
/// Component (`^`), repetition (`~`), and subcomponent (`&`) separators are
/// left alone, because a field value is expected to carry them — escaping
/// `12345^^^NZHPI^MR` would destroy the identifier it encodes.
///
/// `\r` and `\n` become `\X0D\` and `\X0A\`; a raw `\r` would otherwise split
/// the segment, and a raw `\n` is not a legal HL7 byte.
fn escape_value(value: &str, delims: Delimiters) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        let sequence = if ch == delims.escape as char {
            Some("\\E\\")
        } else if ch == delims.field as char {
            Some("\\F\\")
        } else {
            match ch {
                '\r' => Some("\\X0D\\"),
                '\n' => Some("\\X0A\\"),
                _ => None,
            }
        };
        match sequence {
            Some(escape) => out.push_str(escape),
            None => out.push(ch),
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Hl7Parser;

    fn sample_message() -> Message {
        let mut msg = Message::msh(
            "HIS",
            "ACME",
            "RIS",
            "ACME",
            "20261001120000",
            "ADT^A01",
            "MSG00001",
        );
        msg.push_segment("EVN", &["A01", "20261001120000"]);
        msg.push_segment(
            "PID",
            &[
                "1",
                "",
                "12345^^^NZHPI^MR",
                "",
                "Smith^John",
                "",
                "19700101",
            ],
        );
        msg
    }

    #[test]
    fn encoded_bytes_are_cr_delimited() {
        let bytes = sample_message().to_bytes();
        let text = core::str::from_utf8(&bytes).expect("UTF-8");
        assert!(text.ends_with('\r'));
        let lines: Vec<_> = text.trim_end_matches('\r').split('\r').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("MSH|^~\\&|HIS|ACME|RIS|ACME|"));
        assert_eq!(lines[1], "EVN|A01|20261001120000");
        assert!(lines[2].starts_with("PID|1||12345"));
    }

    /// A field value containing a delimiter must not be written verbatim, or
    /// the message structure is destroyed on the wire.
    #[test]
    fn delimiters_inside_field_values_are_escaped() {
        let mut msg = Message::msh(
            "HIS", "ACME", "RIS", "ACME", "20261001120000", "ADT^A01", "MSG1",
        );
        msg.push_segment("NTE", &["1", r"Surname: O'Fahey | given: A*B"]);
        let bytes = msg.to_bytes();
        let text = core::str::from_utf8(&bytes).expect("UTF-8");
        let nte = text.trim_end_matches('\r').split('\r').nth(1).expect("NTE");
        assert_eq!(nte, "NTE|1|Surname: O'Fahey \\F\\ given: A*B");
    }

    #[test]
    fn every_encoding_character_is_escaped() {
        let mut msg = Message::msh(
            "HIS", "ACME", "RIS", "ACME", "20261001120000", "ADT^A01", "MSG1",
        );
        msg.push_segment("NTE", &["1", "a|b^c~d\\e&f"]);
        let bytes = msg.to_bytes();
        let text = core::str::from_utf8(&bytes).expect("UTF-8");
        let nte = text.trim_end_matches('\r').split('\r').nth(1).expect("NTE");
        // field and escape separators are escaped; ^ ~ & are legitimate component data
        assert_eq!(nte, "NTE|1|a\\F\\b^c~d\\E\\e&f");
    }

    /// Carriage returns and newlines inside a value would split the segment.
    #[test]
    fn carriage_returns_and_newlines_in_values_are_escaped() {
        let mut msg = Message::msh(
            "HIS", "ACME", "RIS", "ACME", "20261001120000", "ADT^A01", "MSG1",
        );
        msg.push_segment("NTE", &["1", "line one\rline two\nline three"]);
        let bytes = msg.to_bytes();
        let text = core::str::from_utf8(&bytes).expect("UTF-8");
        // Exactly two segments: the value did not create extra ones.
        assert_eq!(text.trim_end_matches('\r').split('\r').count(), 2);
        assert!(text.contains("line one\\X0D\\line two\\X0A\\line three"));
    }

    /// The escape character itself must survive a round trip.
    #[test]
    fn escape_sequences_are_reversed_on_read() {
        let mut msg = Message::msh(
            "HIS", "ACME", "RIS", "ACME", "20261001120000", "ADT^A01", "MSG1",
        );
        msg.push_segment("NTE", &["1", "a|b^c"]);
        let bytes = msg.to_bytes();
        let segs: Vec<_> = Hl7Parser::new(&bytes).collect::<Result<Vec<_>, _>>().expect("parse");
        assert_eq!(segs[1].field(2), Some("a\\F\\b^c"));
    }

    #[test]
    fn encode_parse_round_trip() {
        let bytes = sample_message().to_bytes();
        let segs: Vec<_> = Hl7Parser::new(&bytes)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse error");
        assert_eq!(segs.len(), 3);

        let msh = &segs[0];
        assert_eq!(msh.field(3), Some("HIS"));
        assert_eq!(msh.component(9, 2), Some("A01"));
        assert_eq!(msh.field(10), Some("MSG00001"));
        assert_eq!(msh.field(12), Some("2.5.1"));

        let pid = &segs[2];
        assert_eq!(pid.component(3, 4), Some("NZHPI"));
        assert_eq!(pid.component(5, 2), Some("John"));
    }

    #[test]
    fn empty_fields_are_preserved() {
        let mut msg = Message::msh("A", "B", "C", "D", "", "ORU^R01", "CTRL1");
        msg.push_segment("OBX", &["1", "", "ST", "", "90"]);
        let bytes = msg.to_bytes();
        let text = core::str::from_utf8(&bytes).expect("UTF-8");
        let msh_line = text.split('\r').next().expect("first line");
        assert_eq!(msh_line, "MSH|^~\\&|A|B|C|D|||ORU^R01|CTRL1|P|2.5.1");
    }
}

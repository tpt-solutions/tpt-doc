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
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let sep = self.delims.field;
        let mut out = Vec::new();
        for seg in &self.segments {
            out.extend_from_slice(seg.tag.as_bytes());
            out.push(sep);
            out.extend_from_slice(seg.fields.join(&(sep as char).to_string()).as_bytes());
            out.push(b'\r');
        }
        out
    }
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

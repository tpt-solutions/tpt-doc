//! HL7 v2 acknowledgments (`ACK`).
//!
//! An acknowledgment answers one message. It swaps sender and receiver, names
//! the trigger event of the original (`ADT^A01` is acknowledged by
//! `ACK^A01`), and carries the original message's control ID in `MSA-2` so the
//! sender can match it up.

use tpt_doc_core::DocError;

use crate::encode::escape_text;
use crate::{Hl7Parser, Message, Segment};

/// The acknowledgment code, `MSA-1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckCode {
    /// `AA`: the message was accepted and processed.
    Accept,
    /// `AE`: the message was understood but could not be processed.
    Error,
    /// `AR`: the message was refused, for example an unsupported type.
    Reject,
}

impl AckCode {
    /// The three-letter code as it appears in `MSA-1`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Accept => "AA",
            Self::Error => "AE",
            Self::Reject => "AR",
        }
    }
}

/// Build the acknowledgment for an HL7 v2 message.
///
/// `control_id` and `datetime` belong to the acknowledgment itself. `text`
/// becomes `MSA-3`, a human-readable reason, and is escaped by the encoder.
///
/// The version is taken from the original's `MSH-12`, falling back to `2.5.1`.
///
/// # Errors
/// Returns [`DocError`] if `original` does not parse, or its first segment is
/// not `MSH`.
pub fn ack_for(
    original: &[u8],
    code: AckCode,
    text: Option<&str>,
    control_id: &str,
    datetime: &str,
) -> Result<Vec<u8>, DocError> {
    let segments: Vec<Segment<'_>> = Hl7Parser::new(original).collect::<Result<_, _>>()?;
    let msh = segments
        .first()
        .filter(|segment| segment.tag() == "MSH")
        .ok_or_else(|| DocError::invalid_format("an acknowledgment needs an MSH first"))?;

    // MSH-3/4 are the sender; in the acknowledgment they become the receiver.
    let receiving_app = msh.field(3).unwrap_or_default();
    let receiving_facility = msh.field(4).unwrap_or_default();
    let sending_app = msh.field(5).unwrap_or_default();
    let sending_facility = msh.field(6).unwrap_or_default();
    let original_control_id = msh.field(10).unwrap_or_default();
    let trigger = msh.component(9, 2).unwrap_or_default();
    let version = msh
        .field(12)
        .and_then(|v| v.split(char::from(msh.delimiters().component)).next())
        .filter(|v| !v.is_empty())
        .unwrap_or("2.5.1");

    let message_type = if trigger.is_empty() {
        "ACK".to_owned()
    } else {
        format!("ACK^{trigger}")
    };
    let mut ack = Message::msh_with_version(
        sending_app,
        sending_facility,
        receiving_app,
        receiving_facility,
        datetime,
        &message_type,
        control_id,
        version,
    );
    match text {
        Some(text) => {
            let reason = escape_text(text);
            ack.push_escaped_segment("MSA", &[code.code(), original_control_id, reason.as_str()]);
        }
        None => ack.push_segment("MSA", &[code.code(), original_control_id]),
    }
    Ok(ack.to_bytes())
}

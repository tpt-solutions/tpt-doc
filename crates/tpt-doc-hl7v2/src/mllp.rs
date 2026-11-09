//! MLLP framing: how HL7 v2 messages travel over a TCP connection.
//!
//! Each message is wrapped as `<VT> message <FS><CR>`, where VT is `0x0B`,
//! FS is `0x1C` and CR is `0x0D`. The framing is what lets a receiver find
//! message boundaries in a byte stream.

use tpt_doc_core::{DocError, Location};

/// Start of a frame (`VT`, vertical tab).
pub const START_BLOCK: u8 = 0x0B;
/// End of a frame (`FS`, file separator); followed by [`CARRIAGE_RETURN`].
pub const END_BLOCK: u8 = 0x1C;
/// The byte that closes every frame.
pub const CARRIAGE_RETURN: u8 = 0x0D;

/// Wrap one message in an MLLP frame.
#[must_use]
pub fn frame(message: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(message.len() + 3);
    out.push(START_BLOCK);
    out.extend_from_slice(message);
    out.push(END_BLOCK);
    out.push(CARRIAGE_RETURN);
    out
}

/// Split a byte stream into the messages it carries, borrowing from `stream`.
///
/// # Errors
/// Returns [`DocError`] at the byte offset where the stream stops being valid:
/// a byte that is not a frame start where one is expected, or a frame that
/// never ends. The error is raised rather than returning a partial list, so a
/// caller cannot mistake a truncated message for a complete one.
pub fn unframe(stream: &[u8]) -> Result<Vec<&[u8]>, DocError> {
    let mut messages = Vec::new();
    let mut rest = stream;
    while !rest.is_empty() {
        let offset = (stream.len() - rest.len()) as u64;
        if rest[0] != START_BLOCK {
            return Err(
                DocError::invalid_format("expected an MLLP frame start (0x0B)")
                    .with_location(Location::Offset(offset)),
            );
        }
        let body = &rest[1..];
        let Some(end) = body
            .windows(2)
            .position(|pair| pair == [END_BLOCK, CARRIAGE_RETURN])
        else {
            return Err(DocError::invalid_format(
                "MLLP frame is not terminated (expected 0x1C 0x0D)",
            )
            .with_location(Location::Offset(offset)));
        };
        messages.push(&body[..end]);
        rest = &body[end + 2..];
    }
    Ok(messages)
}

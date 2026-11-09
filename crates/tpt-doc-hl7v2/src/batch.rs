//! HL7 v2 batches: several messages in one stream, framed by a batch header
//! (`BHS`) and trailer (`BTS`).
//!
//! A batch looks like `BHS … MSH … MSH … BTS|<count>`. A file header (`FHS`)
//! and file trailer (`FTS`) may wrap one or more batches; this module builds
//! and reads the single-batch form, and reads a file wrapper around it.
//! Batches are usually carried in MLLP frames, see [`crate::mllp`].

use tpt_doc_core::{DocError, Location};

use crate::encode::{Message, escape_text};

/// The header fields of a batch, written into `BHS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchHeader {
    /// `BHS-3`: sending application.
    pub sending_app: String,
    /// `BHS-4`: sending facility.
    pub sending_facility: String,
    /// `BHS-5`: receiving application.
    pub receiving_app: String,
    /// `BHS-6`: receiving facility.
    pub receiving_facility: String,
    /// `BHS-7`: creation date and time.
    pub datetime: String,
    /// `BHS-10`: batch name or ID.
    pub batch_id: String,
}

/// Build a batch: `BHS`, then each message, then `BTS` with the message count.
///
/// Each message must start with an `MSH` segment. A missing final line break is
/// added, so messages can be concatenated safely.
///
/// # Errors
/// Returns [`DocError`] if a message does not start with `MSH`.
pub fn build_batch(header: &BatchHeader, messages: &[&[u8]]) -> Result<Vec<u8>, DocError> {
    let mut out = Vec::new();
    let mut bhs = Message::default();
    // BHS-2 is the encoding characters, which are written verbatim.
    bhs.push_escaped_segment(
        "BHS",
        &[
            "^~\\&",
            &escape_text(&header.sending_app),
            &escape_text(&header.sending_facility),
            &escape_text(&header.receiving_app),
            &escape_text(&header.receiving_facility),
            &escape_text(&header.datetime),
            "",
            "",
            "",
            &escape_text(&header.batch_id),
        ],
    );
    out.extend(bhs.to_bytes());

    for message in messages {
        if !message.starts_with(b"MSH") {
            return Err(DocError::invalid_format(
                "a batch can only contain messages that start with MSH",
            ));
        }
        out.extend_from_slice(message);
        if !message.ends_with(b"\r") {
            out.push(b'\r');
        }
    }

    let mut bts = Message::default();
    bts.push_escaped_segment("BTS", &[&messages.len().to_string()]);
    out.extend(bts.to_bytes());
    Ok(out)
}

/// Read the messages from a batch, borrowing from `bytes`.
///
/// The batch may be wrapped in `FHS`/`FTS`. The `BTS` count must match the
/// number of messages found. A batch with no `BTS`, or one that counts a
/// different number of messages, is refused, so a truncated batch cannot pass
/// for a complete one.
///
/// # Errors
/// Returns [`DocError`] if the stream does not start with `BHS` or `FHS`, if a
/// message is not preceded by `MSH`, if `BTS` is missing or its count is wrong,
/// or if the stream ends inside a message without a trailer.
pub fn read_batch(bytes: &[u8]) -> Result<Vec<&[u8]>, DocError> {
    // Walk the segments, recording where each message starts and ends.
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut open: Option<usize> = None;
    let mut trailer_count: Option<usize> = None;
    let mut first = true;
    let mut offset = 0usize;
    for raw in bytes.split(|b| *b == b'\r') {
        let start = offset;
        offset += raw.len() + 1;
        if raw.is_empty() {
            continue;
        }
        let tag = raw.get(..3).unwrap_or(raw);
        if first {
            first = false;
            if tag != b"BHS" && tag != b"FHS" {
                return Err(
                    DocError::invalid_format("a batch must start with BHS or FHS")
                        .with_location(Location::Offset(start as u64)),
                );
            }
        }
        match tag {
            b"MSH" => {
                if let Some(begin) = open.replace(start) {
                    spans.push((begin, start));
                }
            }
            b"BTS" => {
                if let Some(begin) = open.take() {
                    spans.push((begin, start));
                }
                trailer_count = Some(parse_count(raw, start)?);
            }
            b"BHS" | b"FHS" | b"FTS" => {
                if let Some(begin) = open.take() {
                    spans.push((begin, start));
                }
            }
            _ => {
                if open.is_none() && trailer_count.is_none() {
                    return Err(
                        DocError::invalid_format("a segment appears outside any message")
                            .with_location(Location::Offset(start as u64)),
                    );
                }
            }
        }
    }
    if let Some(begin) = open {
        // The stream ended inside a message: it was never closed.
        return Err(
            DocError::invalid_format("batch ends inside a message: no BTS trailer")
                .with_location(Location::Offset(begin as u64)),
        );
    }
    let Some(count) = trailer_count else {
        return Err(DocError::invalid_format("batch has no BTS trailer"));
    };
    if count != spans.len() {
        return Err(DocError::invalid_format(format!(
            "BTS counts {count} messages but the batch holds {}",
            spans.len()
        )));
    }
    Ok(spans.into_iter().map(|(a, b)| &bytes[a..b]).collect())
}

/// Parse the message count from `BTS-1`.
fn parse_count(segment: &[u8], offset: usize) -> Result<usize, DocError> {
    let text = segment
        .split(|b| *b == b'|')
        .nth(1)
        .and_then(|field| std::str::from_utf8(field).ok())
        .unwrap_or("");
    text.trim().parse::<usize>().map_err(|_| {
        DocError::invalid_format(format!("BTS-1 must be a message count, found `{text}`"))
            .with_location(Location::Offset(offset as u64))
    })
}

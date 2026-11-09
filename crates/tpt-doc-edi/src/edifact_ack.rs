//! EDIFACT `CONTRL` acknowledgments: the EDIFACT counterpart of the X12 `997`.
//!
//! A `CONTRL` message answers an interchange. Its `UCI` segment names the
//! acknowledged interchange and carries an action code: `7` for an accepted
//! interchange and `4` for a rejected one. The envelope is swapped, as with
//! the `997`.
//!
//! This is the interchange-level subset: one `UCI` for the whole interchange,
//! no message-level `UCM` detail. The action codes are the ones used here; check
//! them against the UN/EDIFACT CONTRL directory before relying on them with a
//! trading partner.

use std::fmt::Write as _;

use tpt_doc_core::DocError;

use crate::edifact::EdifactParser;

/// The acknowledgment action codes written to `UCI`.
const ACTION_ACCEPTED: &str = "7";
const ACTION_REJECTED: &str = "4";

/// Build a `CONTRL` acknowledgment for an EDIFACT interchange.
///
/// `control` is the new interchange control reference (written to `UNB`,
/// `UNH` and `UNZ`) and `date` and `time` are the creation date (`YYMMDD`) and
/// time (`HHMM`).
///
/// # Errors
/// Returns [`DocError`] if the input has no `UNB`, or `UNB` lacks the sender,
/// recipient or control reference.
pub fn build_contrl(
    interchange: &[u8],
    accepted: bool,
    control: &str,
    date: &str,
    time: &str,
) -> Result<Vec<u8>, DocError> {
    let segments: Vec<_> = EdifactParser::new(interchange).collect::<Result<_, _>>()?;
    let unb = segments
        .iter()
        .find(|s| s.tag() == "UNB")
        .ok_or_else(|| DocError::invalid_format("interchange has no UNB"))?;
    let element = |index: usize| -> Result<String, DocError> {
        unb.elements()
            .get(index)
            .and_then(|e| e.split(':').next())
            .filter(|e| !e.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| {
                DocError::invalid_format(format!("UNB element {} is missing", index + 1))
            })
    };
    let sender = element(1)?;
    let recipient = element(2)?;
    let reference = element(4)?;
    let action = if accepted {
        ACTION_ACCEPTED
    } else {
        ACTION_REJECTED
    };

    let mut out = String::new();
    // The UNB the answer travels in: the original recipient answers.
    let _ = write!(
        out,
        "UNB+UNOA:1+{recipient}+{sender}+{date}:{time}+{control}'"
    );
    let _ = write!(out, "UNH+1+CONTRL:0:3:UN'");
    let _ = write!(out, "UCI+{reference}+{sender}+{recipient}+{action}'");
    let _ = write!(out, "UNT+3+1'");
    let _ = write!(out, "UNZ+1+{control}'");
    Ok(out.into_bytes())
}

/// Read the interchange reference and action code from a `CONTRL`.
///
/// Returns `(acknowledged reference, accepted)`.
///
/// # Errors
/// Returns [`DocError`] if there is no `UCI`, or its action code is neither
/// the accepted nor the rejected code.
pub fn read_contrl(contrl: &[u8]) -> Result<(String, bool), DocError> {
    let segments: Vec<_> = EdifactParser::new(contrl).collect::<Result<_, _>>()?;
    let uci = segments
        .iter()
        .find(|s| s.tag() == "UCI")
        .ok_or_else(|| DocError::invalid_format("CONTRL has no UCI"))?;
    let reference = uci
        .elements()
        .first()
        .copied()
        .unwrap_or_default()
        .to_owned();
    let action = uci
        .elements()
        .get(3)
        .and_then(|e| e.split(':').next())
        .unwrap_or_default();
    match action {
        ACTION_ACCEPTED => Ok((reference, true)),
        ACTION_REJECTED => Ok((reference, false)),
        other => Err(DocError::invalid_format(format!(
            "UCI action code `{other}` is not one this subset writes"
        ))),
    }
}

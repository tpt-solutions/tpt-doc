//! X12 `997` functional acknowledgments.
//!
//! A `997` tells the sender of an interchange whether each functional group
//! and transaction set arrived and was accepted. The envelope is swapped: the
//! receiver of the original becomes the sender of the acknowledgment.
//!
//! This builder writes one functional group containing one `AK2` per
//! transaction set in the original. It accepts or rejects the whole
//! interchange (`AK5`/`AK9` `A` or `R`). It does not write element-level
//! `AK3`/`AK4` error detail, so a rejection says that a set was refused but
//! not which element was wrong.

use std::fmt::Write as _;

use tpt_doc_core::DocError;

use crate::x12::{X12Parser, X12Segment};

/// The envelope values a `997` needs from the interchange it answers.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Envelope {
    /// ISA05 and ISA06 of the original: the sender of the original.
    sender_qualifier: String,
    sender_id: String,
    /// ISA07 and ISA08: the receiver of the original.
    receiver_qualifier: String,
    receiver_id: String,
    /// ISA13: the interchange control number.
    interchange_control: String,
    /// GS01: the functional identifier (for example `HP` for a 835).
    functional_id: String,
    /// GS06: the group control number.
    group_control: String,
    /// The `ST` of each transaction set: (set id, control number).
    sets: Vec<(String, String)>,
}

/// Build a `997` acknowledging an X12 interchange.
///
/// `accepted` decides the `AK5` and `AK9` codes for every set: `true` writes
/// `A` (accepted), `false` writes `R` (rejected). `control` is the new
/// interchange control number (nine digits, written into ISA13 and IEA02) and
/// `date` and `time` are the creation date (`YYMMDD`) and time (`HHMM`).
///
/// # Errors
/// Returns [`DocError`] if the input is not an X12 interchange with an ISA,
/// GS and at least one ST, or if `control` is not a number of at most nine
/// digits.
pub fn build_997(
    interchange: &[u8],
    accepted: bool,
    control: &str,
    date: &str,
    time: &str,
) -> Result<Vec<u8>, DocError> {
    if control.is_empty() || control.len() > 9 || !control.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DocError::invalid_format(
            "997/999 interchange control number must be up to nine digits",
        ));
    }
    let segments: Vec<X12Segment<'_>> = X12Parser::new(interchange)?.collect::<Result<_, _>>()?;
    let envelope = read_envelope(&segments)?;
    Ok(write_acknowledgment(&envelope, "997", "AK5", accepted, control, date, time).into_bytes())
}

/// Build a `999` implementation acknowledgment for an X12 interchange.
///
/// The arguments and envelope are those of [`build_997`]. The set-level response
/// is `IK5` rather than `AK5`. This writes no `IK3`/`IK4` segment-error detail,
/// so a rejection says a set was refused but not which segment was wrong.
///
/// # Errors
/// As [`build_997`].
pub fn build_999(
    interchange: &[u8],
    accepted: bool,
    control: &str,
    date: &str,
    time: &str,
) -> Result<Vec<u8>, DocError> {
    if control.is_empty() || control.len() > 9 || !control.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DocError::invalid_format(
            "997 interchange control number must be up to nine digits",
        ));
    }
    let segments: Vec<X12Segment<'_>> = X12Parser::new(interchange)?.collect::<Result<_, _>>()?;
    let envelope = read_envelope(&segments)?;
    Ok(write_acknowledgment(&envelope, "999", "IK5", accepted, control, date, time).into_bytes())
}

fn read_envelope(segments: &[X12Segment<'_>]) -> Result<Envelope, DocError> {
    let isa = segments
        .iter()
        .find(|s| s.tag() == "ISA")
        .ok_or_else(|| DocError::invalid_format("interchange has no ISA"))?;
    let gs = segments
        .iter()
        .find(|s| s.tag() == "GS")
        .ok_or_else(|| DocError::invalid_format("interchange has no GS"))?;
    let at = |segment: &X12Segment<'_>, index: usize| -> String {
        segment
            .elements()
            .get(index)
            .copied()
            .unwrap_or("")
            .to_owned()
    };

    let mut sets = Vec::new();
    let mut open = false;
    for segment in segments {
        match segment.tag() {
            "ST" => {
                open = true;
                sets.push((at(segment, 0), at(segment, 1)));
            }
            "SE" => open = false,
            _ => {}
        }
    }
    if sets.is_empty() || open {
        return Err(DocError::invalid_format(
            "interchange has no complete transaction set to acknowledge",
        ));
    }

    Ok(Envelope {
        sender_qualifier: at(isa, 4),
        sender_id: at(isa, 5),
        receiver_qualifier: at(isa, 6),
        receiver_id: at(isa, 7),
        interchange_control: at(isa, 12),
        functional_id: at(gs, 0),
        group_control: at(gs, 5),
        sets,
    })
}

fn write_acknowledgment(
    envelope: &Envelope,
    version: &str,
    response_tag: &str,
    accepted: bool,
    control: &str,
    date: &str,
    time: &str,
) -> String {
    let code = if accepted { "A" } else { "R" };
    let set_count = envelope.sets.len();

    // The functional group body. SE01 counts its segments from ST to SE.
    let mut body: Vec<String> = vec![
        format!("ST*{version}*0001"),
        format!("AK1*{}*{}", envelope.functional_id, envelope.group_control),
    ];
    for (set_id, set_control) in &envelope.sets {
        body.push(format!("AK2*{set_id}*{set_control}"));
        body.push(format!("{response_tag}*{code}"));
    }
    let accepted_count = if accepted { set_count } else { 0 };
    body.push(format!(
        "AK9*{code}*{set_count}*{set_count}*{accepted_count}"
    ));
    body.push(format!("SE*{}*0001", body.len() + 1));

    let mut out = String::new();
    // ISA: the receiver of the original answers, so the parties are swapped.
    let _ = write!(
        out,
        "ISA*00*{:10}*00*{:10}*{}*{:<15}*{}*{:<15}*{date}*{time}*^*00501*{control:0>9}*0*P*:~",
        "",
        "",
        envelope.receiver_qualifier,
        envelope.receiver_id,
        envelope.sender_qualifier,
        envelope.sender_id,
    );
    let _ = write!(
        out,
        "GS*FA*{}*{}*{}*{time}*{}*X*005010X231A1~",
        envelope.receiver_id, envelope.sender_id, date, envelope.group_control,
    );
    for segment in &body {
        out.push_str(segment);
        out.push('~');
    }
    let _ = write!(out, "GE*1*{}~", envelope.group_control);
    let _ = write!(out, "IEA*1*{control:0>9}~");
    out
}

/// One transaction-set acknowledgment (`AK2` with its `AK5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetAck {
    /// `AK2-1`: the acknowledged set id, for example `835`.
    pub set_id: String,
    /// `AK2-2`: the acknowledged set's control number.
    pub set_control: String,
    /// `AK5-1`: `A` accepted, `E` accepted with errors, `R` rejected.
    pub code: String,
}

/// One functional-group acknowledgment (`AK1` through `AK9`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupAck {
    /// `AK1-1`: the acknowledged group's functional identifier.
    pub functional_id: String,
    /// `AK1-2`: the acknowledged group's control number.
    pub group_control: String,
    /// The acknowledged transaction sets, in order.
    pub sets: Vec<SetAck>,
    /// `AK9-1`: the group code.
    pub group_code: String,
    /// `AK9-2`: transaction sets included.
    pub sets_included: usize,
    /// `AK9-3`: transaction sets received.
    pub sets_received: usize,
    /// `AK9-4`: transaction sets accepted.
    pub sets_accepted: usize,
}

impl GroupAck {
    /// Whether the group was accepted, with or without errors.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        matches!(self.group_code.as_str(), "A" | "E")
    }
}

/// Read a `997` functional acknowledgment.
///
/// The counts in each `AK9` are checked against the `AK2` lines that precede
/// it, so an acknowledgment that claims more sets than it lists is refused.
///
/// # Errors
/// Returns [`DocError`] if the interchange does not parse, an `AK2` or `AK5`
/// appears outside a group, a group has no `AK9`, or an `AK9` count does not
/// match the sets listed.
pub fn parse_997(interchange: &[u8]) -> Result<Vec<GroupAck>, DocError> {
    parse_acknowledgment(interchange, "AK5")
}

/// Read a `999` implementation acknowledgment. Its per-set response is `IK5`
/// rather than `AK5`; otherwise it reads like a `997`.
///
/// # Errors
/// As [`parse_997`].
pub fn parse_999(interchange: &[u8]) -> Result<Vec<GroupAck>, DocError> {
    parse_acknowledgment(interchange, "IK5")
}

fn parse_acknowledgment(interchange: &[u8], response_tag: &str) -> Result<Vec<GroupAck>, DocError> {
    let segments: Vec<X12Segment<'_>> = X12Parser::new(interchange)?.collect::<Result<_, _>>()?;
    let mut groups: Vec<GroupAck> = Vec::new();
    let mut open: Option<GroupAck> = None;
    let mut current_set = false;

    let at = |segment: &X12Segment<'_>, index: usize| -> String {
        segment
            .elements()
            .get(index)
            .copied()
            .unwrap_or("")
            .to_owned()
    };
    let count = |segment: &X12Segment<'_>, index: usize| -> Result<usize, DocError> {
        at(segment, index)
            .parse::<usize>()
            .map_err(|_| DocError::invalid_format(format!("AK9-{} must be a count", index + 1)))
    };

    for segment in &segments {
        match segment.tag() {
            "AK1" => {
                if open.is_some() {
                    return Err(DocError::invalid_format(
                        "AK1 before the previous group's AK9",
                    ));
                }
                open = Some(GroupAck {
                    functional_id: at(segment, 0),
                    group_control: at(segment, 1),
                    sets: Vec::new(),
                    group_code: String::new(),
                    sets_included: 0,
                    sets_received: 0,
                    sets_accepted: 0,
                });
                current_set = false;
            }
            "AK2" => {
                let group = open
                    .as_mut()
                    .ok_or_else(|| DocError::invalid_format("AK2 outside a group"))?;
                group.sets.push(SetAck {
                    set_id: at(segment, 0),
                    set_control: at(segment, 1),
                    code: String::new(),
                });
                current_set = true;
            }
            tag if tag == response_tag => {
                let group = open
                    .as_mut()
                    .ok_or_else(|| DocError::invalid_format("AK5 outside a group"))?;
                if !current_set {
                    return Err(DocError::invalid_format("AK5 without an AK2"));
                }
                if let Some(set) = group.sets.last_mut() {
                    set.code = at(segment, 0);
                }
            }
            "AK9" => {
                let mut group = open
                    .take()
                    .ok_or_else(|| DocError::invalid_format("AK9 without an AK1"))?;
                group.group_code = at(segment, 0);
                group.sets_included = count(segment, 1)?;
                group.sets_received = count(segment, 2)?;
                group.sets_accepted = count(segment, 3)?;
                if let Some(set) = group.sets.iter().find(|set| set.code.is_empty()) {
                    return Err(DocError::invalid_format(format!(
                        "transaction set {} has no response code",
                        set.set_control
                    )));
                }
                if group.sets_included != group.sets.len()
                    || group.sets_received != group.sets.len()
                {
                    return Err(DocError::invalid_format(format!(
                        "AK9 counts {} sets but the group lists {}",
                        group.sets_included,
                        group.sets.len()
                    )));
                }
                groups.push(group);
            }
            _ => {}
        }
    }
    if open.is_some() {
        return Err(DocError::invalid_format("group has no AK9 trailer"));
    }
    Ok(groups)
}

//! EDIFACT `ORDERS` (purchase orders) and `INVOIC` (invoices): read-only.
//!
//! The three messages share one shape here (for DESADV the number is the
//! despatch note and the quantities are the quantities despatched): `BGM` for the document number, `DTM+137`
//! for its date, `NAD+BY` for the buyer, a `LIN` for each line with its `QTY`,
//! and for an invoice a `MOA+9` amount due. The `UNT` segment count is checked
//! against the segments of the message, as the standard requires.
//!
//! This reads the message the way the standard lays it out. It does not handle
//! the release character inside element values; values are taken as they
//! appear, so a value containing `+`, `:` or `'` written with the release
//! character `?` will come out with the `?` in it.

use tpt_doc_core::DocError;

use crate::edifact::{EdifactParser, Segment};

/// The kind of document, from `UNH-02`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// A purchase order.
    Orders,
    /// An invoice.
    Invoic,
    /// A despatch advice (delivery note).
    Desadv,
}

/// An `ORDERS` or `INVOIC` message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Which message this is.
    pub kind: MessageKind,
    /// `BGM-02`: the order or invoice number.
    pub number: String,
    /// `DTM+137`: the document date, as sent (`CCYYMMDD` for format `102`).
    pub date: String,
    /// `NAD+BY`: the buyer's party identifier, when present.
    pub buyer: Option<String>,
    /// The lines, in order.
    pub lines: Vec<DocLine>,
    /// `MOA+9`: the amount due on an invoice, as sent.
    pub amount_due: Option<String>,
}

/// One `LIN` with its quantity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocLine {
    /// `LIN-01`: the line number.
    pub line_number: String,
    /// `LIN-03`: the item number (the component before its qualifier).
    pub item: String,
    /// `QTY-01` value (`QTY+21:10` gives `10`), when a `QTY` follows the line.
    pub quantity: Option<String>,
}

/// Read an `ORDERS`, `INVOIC` or `DESADV` message from an EDIFACT interchange.
///
/// # Errors
/// Returns [`DocError`] if the input does not parse, there is no `UNH` of a
/// supported kind (`ORDERS`, `INVOIC` or `DESADV`), the `UNT` segment count does not match the message, or a
/// `LIN` has no item number.
pub fn parse_document(interchange: &[u8]) -> Result<Document, DocError> {
    let segments: Vec<Segment<'_>> = EdifactParser::new(interchange).collect::<Result<_, _>>()?;
    let start = segments
        .iter()
        .position(|s| s.tag() == "UNH")
        .ok_or_else(|| DocError::invalid_format("interchange has no UNH"))?;
    let kind = match component(&segments[start], 1, 0) {
        "ORDERS" => MessageKind::Orders,
        "INVOIC" => MessageKind::Invoic,
        "DESADV" => MessageKind::Desadv,
        other => {
            return Err(DocError::invalid_format(format!(
                "message type `{other}` is not ORDERS or INVOIC"
            )));
        }
    };
    let end = segments[start..]
        .iter()
        .position(|s| s.tag() == "UNT")
        .map(|offset| start + offset)
        .ok_or_else(|| DocError::invalid_format("message has no UNT trailer"))?;

    let message = &segments[start..=end];
    let declared = element(&segments[end], 0);
    if declared.parse::<usize>().ok() != Some(message.len()) {
        return Err(DocError::invalid_format(format!(
            "UNT counts {declared} segments but the message has {}",
            message.len()
        )));
    }

    let mut number = String::new();
    let mut date = String::new();
    let mut buyer = None;
    let mut amount_due = None;
    let mut lines: Vec<DocLine> = Vec::new();
    for segment in message {
        match segment.tag() {
            "BGM" => element(segment, 1).clone_into(&mut number),
            "DTM" if component(segment, 0, 0) == "137" => {
                component(segment, 0, 1).clone_into(&mut date);
            }
            "NAD" if element(segment, 0) == "BY" => {
                buyer = Some(component(segment, 1, 0).to_owned()).filter(|b| !b.is_empty());
            }
            "MOA" if component(segment, 0, 0) == "9" => {
                amount_due = Some(component(segment, 0, 1).to_owned());
            }
            "LIN" => {
                let item = component(segment, 2, 0);
                if item.is_empty() {
                    return Err(DocError::invalid_format(format!(
                        "LIN {} has no item number",
                        element(segment, 0)
                    )));
                }
                lines.push(DocLine {
                    line_number: element(segment, 0).to_owned(),
                    item: item.to_owned(),
                    quantity: None,
                });
            }
            "QTY" => {
                if let Some(line) = lines.last_mut() {
                    let value = component(segment, 0, 1);
                    if !value.is_empty() {
                        line.quantity = Some(value.to_owned());
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Document {
        kind,
        number,
        date,
        buyer,
        lines,
        amount_due,
    })
}

/// Data element `index` (0-based, after the tag) of a segment, or `""`.
fn element<'a>(segment: &Segment<'a>, index: usize) -> &'a str {
    segment.elements().get(index).copied().unwrap_or("")
}

/// Component `sub` of data element `index`, or `""`.
fn component<'a>(segment: &Segment<'a>, index: usize, sub: usize) -> &'a str {
    element(segment, index).split(':').nth(sub).unwrap_or("")
}

//! X12 `850` purchase orders: the header, the buyer, and the line items.
//!
//! This reads the stand-alone order that a trading partner sends: `BEG` for the
//! order number and date, `N1*BY` for the buyer, `PO1` for each line, and
//! `CTT` for the line count. Numbers are kept as the text they were sent in,
//! so no precision is lost to a float conversion; convert them with the
//! arithmetic your system requires.
//!
//! The envelope (`ISA`/`GS`/`ST`/`SE`/`GE`/`IEA`) is checked by
//! [`validate_x12_envelope`](crate::validate_x12_envelope), which callers
//! should run as well. This module checks the transaction itself.

use tpt_doc_core::DocError;

use crate::x12::{X12Parser, X12Segment};

/// A purchase order read from an X12 `850`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseOrder {
    /// `BEG-03`: the purchase order number.
    pub number: String,
    /// `BEG-05`: the order date, `CCYYMMDD`.
    pub date: String,
    /// `N1` with qualifier `BY`: the buyer's name, when present.
    pub buyer: Option<String>,
    /// The `PO1` lines, in order.
    pub lines: Vec<OrderLine>,
}

/// One `PO1` line of a purchase order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderLine {
    /// `PO1-01`: the line's identifier.
    pub line_id: String,
    /// `PO1-02`: the quantity ordered, as sent.
    pub quantity: String,
    /// `PO1-03`: the unit of measure, for example `EA`.
    pub unit: String,
    /// `PO1-04`: the unit price, as sent. Empty when not given.
    pub unit_price: String,
    /// The product identifiers as `(qualifier, value)` pairs, from `PO1-06`
    /// onwards, for example `("VP", "WIDGET-1")`.
    pub products: Vec<(String, String)>,
}

/// Read a purchase order from an X12 interchange holding one `850`.
///
/// # Errors
/// Returns [`DocError`] if the interchange does not parse, there is no `ST`
/// with set id `850`, there is no `BEG`, a `PO1` has no line id or quantity,
/// or the `CTT` line count does not match the lines present.
pub fn parse_850(interchange: &[u8]) -> Result<PurchaseOrder, DocError> {
    let segments: Vec<X12Segment<'_>> = X12Parser::new(interchange)?.collect::<Result<_, _>>()?;
    let st = segments
        .iter()
        .position(|s| s.tag() == "ST")
        .ok_or_else(|| DocError::invalid_format("interchange has no ST"))?;
    if segments[st].elements().first().copied() != Some("850") {
        return Err(DocError::invalid_format(
            "the transaction set is not an 850",
        ));
    }
    let se = segments[st..]
        .iter()
        .position(|s| s.tag() == "SE")
        .map_or(segments.len(), |offset| st + offset);
    let body = &segments[st + 1..se];

    let at = |segment: &X12Segment<'_>, index: usize| -> String {
        segment
            .elements()
            .get(index)
            .copied()
            .unwrap_or("")
            .to_owned()
    };

    let beg = body
        .iter()
        .find(|s| s.tag() == "BEG")
        .ok_or_else(|| DocError::invalid_format("850 has no BEG segment"))?;
    let number = at(beg, 2);
    if number.is_empty() {
        return Err(DocError::invalid_format(
            "BEG-03 (purchase order number) is empty",
        ));
    }
    let date = at(beg, 4);

    let buyer = body
        .iter()
        .find(|s| s.tag() == "N1" && s.elements().first() == Some(&"BY"))
        .map(|n1| at(n1, 1))
        .filter(|name| !name.is_empty());

    let mut lines = Vec::new();
    for po1 in body.iter().filter(|s| s.tag() == "PO1") {
        let line_id = at(po1, 0);
        let quantity = at(po1, 1);
        if line_id.is_empty() || quantity.is_empty() {
            return Err(DocError::invalid_format(format!(
                "PO1 line {} is missing its id or quantity",
                lines.len() + 1
            )));
        }
        // PO1-06 onwards are qualifier/value pairs.
        let rest: Vec<&str> = po1.elements().iter().skip(5).copied().collect();
        let products = rest
            .chunks(2)
            .filter_map(|pair| match pair {
                [qualifier, value] if !value.is_empty() => {
                    Some(((*qualifier).to_owned(), (*value).to_owned()))
                }
                _ => None,
            })
            .collect();
        lines.push(OrderLine {
            line_id,
            quantity,
            unit: at(po1, 2),
            unit_price: at(po1, 3),
            products,
        });
    }

    let ctt = body
        .iter()
        .find(|s| s.tag() == "CTT")
        .map(|ctt| at(ctt, 0))
        .ok_or_else(|| DocError::invalid_format("850 has no CTT segment"))?;
    let declared: usize = ctt
        .parse()
        .map_err(|_| DocError::invalid_format(format!("CTT-01 is not a count: `{ctt}`")))?;
    if declared != lines.len() {
        return Err(DocError::invalid_format(format!(
            "CTT counts {declared} lines but the order has {}",
            lines.len()
        )));
    }

    Ok(PurchaseOrder {
        number,
        date,
        buyer,
        lines,
    })
}

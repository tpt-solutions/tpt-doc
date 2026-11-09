//! X12 `856` advance ship notices: the shipment header and the shipped items.
//!
//! The notice is hierarchical. `BSN` names the shipment, each `HL` segment
//! opens a level (`S` shipment, `O` order, `T` tare, `P` pack, `I` item), and
//! an item level carries a `LIN` (the product) and an `SN1` (the quantity
//! shipped). This reader keeps the shipment header and the item levels. The
//! `CTT` count, when present, is checked against the number of `HL` segments,
//! as the standard requires.
//!
//! Numbers are kept as sent, as in the other transaction readers.

use tpt_doc_core::DocError;

use crate::x12::{X12Parser, X12Segment};

/// An advance ship notice read from an X12 `856`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipNotice {
    /// `BSN-02`: the shipment identifier.
    pub shipment_id: String,
    /// `BSN-03`: the shipment date, `CCYYMMDD`.
    pub date: String,
    /// `BSN-04`: the shipment time, `HHMM`, when given.
    pub time: String,
    /// The shipped item levels, in order.
    pub items: Vec<ShippedItem>,
}

/// One item level (`HL*…*I`) of a ship notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShippedItem {
    /// The `HL-01` number of the item level.
    pub level_id: String,
    /// The product identifiers from `LIN`, as `(qualifier, value)` pairs.
    pub products: Vec<(String, String)>,
    /// `SN1-02`: the quantity shipped, as sent.
    pub quantity: String,
    /// `SN1-03`: the unit of measure.
    pub unit: String,
}

/// Read a ship notice from an X12 interchange holding one `856`.
///
/// # Errors
/// Returns [`DocError`] if the interchange does not parse, there is no `ST`
/// with set id `856`, there is no `BSN`, an item level has no `SN1`, or the
/// `CTT` count does not match the number of `HL` segments.
pub fn parse_856(interchange: &[u8]) -> Result<ShipNotice, DocError> {
    let segments: Vec<X12Segment<'_>> = X12Parser::new(interchange)?.collect::<Result<_, _>>()?;
    let st = segments
        .iter()
        .position(|s| s.tag() == "ST")
        .ok_or_else(|| DocError::invalid_format("interchange has no ST"))?;
    if segments[st].elements().first().copied() != Some("856") {
        return Err(DocError::invalid_format(
            "the transaction set is not an 856",
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

    let bsn = body
        .iter()
        .find(|s| s.tag() == "BSN")
        .ok_or_else(|| DocError::invalid_format("856 has no BSN segment"))?;
    let shipment_id = at(bsn, 1);
    if shipment_id.is_empty() {
        return Err(DocError::invalid_format("BSN-02 (shipment id) is empty"));
    }

    let mut items = Vec::new();
    let mut open: Option<ShippedItem> = None;
    let mut hl_count = 0usize;
    for segment in body {
        match segment.tag() {
            "HL" => {
                hl_count += 1;
                if let Some(item) = open.take() {
                    items.push(finish_item(item)?);
                }
                if at(segment, 2) == "I" {
                    open = Some(ShippedItem {
                        level_id: at(segment, 0),
                        products: Vec::new(),
                        quantity: String::new(),
                        unit: String::new(),
                    });
                }
            }
            "LIN" => {
                if let Some(item) = open.as_mut() {
                    // LIN-01 is the line id; the rest are qualifier/value pairs.
                    let rest: Vec<&str> = segment.elements().iter().skip(1).copied().collect();
                    item.products = rest
                        .chunks(2)
                        .filter_map(|pair| match pair {
                            [qualifier, value] if !value.is_empty() => {
                                Some(((*qualifier).to_owned(), (*value).to_owned()))
                            }
                            _ => None,
                        })
                        .collect();
                }
            }
            "SN1" => {
                if let Some(item) = open.as_mut() {
                    item.quantity = at(segment, 1);
                    item.unit = at(segment, 2);
                }
            }
            _ => {}
        }
    }
    if let Some(item) = open.take() {
        items.push(finish_item(item)?);
    }

    if let Some(ctt) = body.iter().find(|s| s.tag() == "CTT") {
        let text = at(ctt, 0);
        let declared: usize = text
            .parse()
            .map_err(|_| DocError::invalid_format(format!("CTT-01 is not a count: `{text}`")))?;
        if declared != hl_count {
            return Err(DocError::invalid_format(format!(
                "CTT counts {declared} HL segments but the notice has {hl_count}"
            )));
        }
    }

    Ok(ShipNotice {
        shipment_id,
        date: at(bsn, 2),
        time: at(bsn, 3),
        items,
    })
}

/// An item level must say what was shipped.
fn finish_item(item: ShippedItem) -> Result<ShippedItem, DocError> {
    if item.quantity.is_empty() {
        return Err(DocError::invalid_format(format!(
            "item level {} has no SN1 quantity",
            item.level_id
        )));
    }
    Ok(item)
}

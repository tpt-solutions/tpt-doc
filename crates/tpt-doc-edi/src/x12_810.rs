//! X12 `810` invoices: the header, the buyer, the line items, and the total.
//!
//! `BIG` carries the invoice number and date, `N1*ST` the buyer, `IT1` each
//! line, `TDS` the invoice total, and `CTT` the line count. As with the `850`
//! module, values are kept as the text they were sent in. The total is kept in
//! cents, because `TDS-01` has an implied two decimal places.
//!
//! The envelope is checked by [`validate_x12_envelope`](crate::validate_x12_envelope);
//! run it as well.

use tpt_doc_core::DocError;

use crate::x12::{X12Parser, X12Segment};

/// An invoice read from an X12 `810`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    /// `BIG-02`: the invoice number.
    pub number: String,
    /// `BIG-01`: the invoice date, `CCYYMMDD`.
    pub date: String,
    /// `BIG-04`: the purchase order the invoice bills against, when given.
    pub purchase_order: Option<String>,
    /// `N1` with qualifier `ST`: the ship-to name, when present.
    pub buyer: Option<String>,
    /// The `IT1` lines, in order.
    pub lines: Vec<InvoiceLine>,
    /// `TDS-01`: the invoice total in cents, when given.
    pub total_cents: Option<u64>,
}

/// One `IT1` line of an invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceLine {
    /// `IT1-01`: the line's identifier.
    pub line_id: String,
    /// `IT1-02`: the quantity invoiced, as sent.
    pub quantity: String,
    /// `IT1-03`: the unit of measure, for example `EA`.
    pub unit: String,
    /// `IT1-04`: the unit price, as sent.
    pub unit_price: String,
    /// The product identifiers as `(qualifier, value)` pairs, from `IT1-06`.
    pub products: Vec<(String, String)>,
}

/// Read an invoice from an X12 interchange holding one `810`.
///
/// # Errors
/// Returns [`DocError`] if the interchange does not parse, there is no `ST`
/// with set id `810`, there is no `BIG`, an `IT1` lacks its id or quantity, the
/// `TDS` total is not a number, or the `CTT` count does not match the lines.
pub fn parse_810(interchange: &[u8]) -> Result<Invoice, DocError> {
    let segments: Vec<X12Segment<'_>> = X12Parser::new(interchange)?.collect::<Result<_, _>>()?;
    let st = segments
        .iter()
        .position(|s| s.tag() == "ST")
        .ok_or_else(|| DocError::invalid_format("interchange has no ST"))?;
    if segments[st].elements().first().copied() != Some("810") {
        return Err(DocError::invalid_format(
            "the transaction set is not an 810",
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

    let big = body
        .iter()
        .find(|s| s.tag() == "BIG")
        .ok_or_else(|| DocError::invalid_format("810 has no BIG segment"))?;
    let number = at(big, 1);
    if number.is_empty() {
        return Err(DocError::invalid_format("BIG-02 (invoice number) is empty"));
    }
    let date = at(big, 0);
    let purchase_order = Some(at(big, 3)).filter(|po| !po.is_empty());

    let buyer = body
        .iter()
        .find(|s| s.tag() == "N1" && s.elements().first() == Some(&"ST"))
        .map(|n1| at(n1, 1))
        .filter(|name| !name.is_empty());

    let mut lines = Vec::new();
    for it1 in body.iter().filter(|s| s.tag() == "IT1") {
        let line_id = at(it1, 0);
        let quantity = at(it1, 1);
        if line_id.is_empty() || quantity.is_empty() {
            return Err(DocError::invalid_format(format!(
                "IT1 line {} is missing its id or quantity",
                lines.len() + 1
            )));
        }
        let rest: Vec<&str> = it1.elements().iter().skip(5).copied().collect();
        let products = rest
            .chunks(2)
            .filter_map(|pair| match pair {
                [qualifier, value] if !value.is_empty() => {
                    Some(((*qualifier).to_owned(), (*value).to_owned()))
                }
                _ => None,
            })
            .collect();
        lines.push(InvoiceLine {
            line_id,
            quantity,
            unit: at(it1, 2),
            unit_price: at(it1, 3),
            products,
        });
    }

    let total_cents =
        match body.iter().find(|s| s.tag() == "TDS") {
            Some(tds) => {
                let text = at(tds, 0);
                Some(text.parse::<u64>().map_err(|_| {
                    DocError::invalid_format(format!("TDS-01 is not cents: `{text}`"))
                })?)
            }
            None => None,
        };

    let ctt = body
        .iter()
        .find(|s| s.tag() == "CTT")
        .map(|ctt| at(ctt, 0))
        .ok_or_else(|| DocError::invalid_format("810 has no CTT segment"))?;
    let declared: usize = ctt
        .parse()
        .map_err(|_| DocError::invalid_format(format!("CTT-01 is not a count: `{ctt}`")))?;
    if declared != lines.len() {
        return Err(DocError::invalid_format(format!(
            "CTT counts {declared} lines but the invoice has {}",
            lines.len()
        )));
    }

    Ok(Invoice {
        number,
        date,
        purchase_order,
        buyer,
        lines,
        total_cents,
    })
}

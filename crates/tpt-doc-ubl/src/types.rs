use serde::{Deserialize, Serialize};
use time::Date;

/// Postal address of a trading party.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Address {
    /// Street name and number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    /// City or locality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// Postal / ZIP code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// ISO 3166-1 alpha-2 country code (e.g. `"NZ"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
}

/// A trading party: the supplier or customer of an invoice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Party {
    /// Legal name of the party.
    pub name: String,
    /// Postal address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
    /// VAT / tax registration number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<String>,
    /// PEPPOL endpoint identifier (e.g. `NZBN` value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
}

impl Party {
    /// Create a party with the given legal name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            address: None,
            tax_id: None,
            endpoint_id: None,
        }
    }
}

/// An invoice line: one item with quantity, unit price, and net amount.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceLine {
    /// 1-based line number within the invoice.
    pub id: u32,
    /// Human-readable item description.
    pub item_name: String,
    /// Invoiced quantity.
    pub quantity: f64,
    /// Price per unit, excluding tax.
    pub unit_price: f64,
    /// Line net amount (`quantity * unit_price`, excluding tax).
    pub line_extension_amount: f64,
}

impl InvoiceLine {
    /// Create a line, computing `line_extension_amount` from quantity and
    /// unit price rounded to 2 decimal places.
    pub fn new(id: u32, item_name: impl Into<String>, quantity: f64, unit_price: f64) -> Self {
        Self {
            id,
            item_name: item_name.into(),
            quantity,
            unit_price,
            line_extension_amount: round2(quantity * unit_price),
        }
    }
}

/// UN/EDIFACT 5305 tax category code subset used by PEPPOL BIS Billing 3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaxCategory {
    /// `S` — standard rate.
    Standard,
    /// `Z` — zero-rated.
    ZeroRated,
    /// `E` — exempt from tax.
    Exempt,
    /// `AE` — reverse charge.
    ReverseCharge,
}

impl TaxCategory {
    /// The UN/EDIFACT 5305 code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Standard => "S",
            Self::ZeroRated => "Z",
            Self::Exempt => "E",
            Self::ReverseCharge => "AE",
        }
    }

    /// Parse a UN/EDIFACT 5305 code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "S" => Some(Self::Standard),
            "Z" => Some(Self::ZeroRated),
            "E" => Some(Self::Exempt),
            "AE" => Some(Self::ReverseCharge),
            _ => None,
        }
    }
}

/// A per-category tax subtotal: taxable amount, rate, and tax amount.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaxSubtotal {
    /// The tax category this subtotal applies to.
    pub category: TaxCategory,
    /// Applicable tax rate percentage (e.g. `15.0` for 15%).
    pub rate_percent: f64,
    /// Net amount the tax rate applies to.
    pub taxable_amount: f64,
    /// Tax amount due for this category.
    pub tax_amount: f64,
}

/// The `LegalMonetaryTotal` block: document-level totals.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MonetaryTotal {
    /// Total net amount excluding tax (`TaxExclusiveAmount`).
    pub tax_exclusive: f64,
    /// Total gross amount including tax (`TaxInclusiveAmount`).
    pub tax_inclusive: f64,
    /// Amount payable (`PayableAmount`).
    pub payable: f64,
}

/// Round to 2 decimal places, avoiding negative-zero output.
pub(crate) fn round2(value: f64) -> f64 {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}

/// Issue date type alias: an ISO 8601 calendar date.
pub type IssueDate = Date;

#[cfg(test)]
mod tests {
    // Monetary arithmetic is rounded to 2dp; exact f64 comparison is intentional.
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn line_computes_extension_amount() {
        let line = InvoiceLine::new(1, "Widget", 3.0, 9.99);
        assert_eq!(line.line_extension_amount, 29.97);
    }

    #[test]
    fn line_rounds_to_two_decimals() {
        let line = InvoiceLine::new(1, "Loose", 3.0, 0.335);
        assert_eq!(line.line_extension_amount, 1.01);
    }

    #[test]
    fn tax_category_codes_round_trip() {
        for (code, category) in [
            ("S", TaxCategory::Standard),
            ("Z", TaxCategory::ZeroRated),
            ("E", TaxCategory::Exempt),
            ("AE", TaxCategory::ReverseCharge),
        ] {
            assert_eq!(TaxCategory::from_code(code), Some(category));
            assert_eq!(category.code(), code);
        }
        assert_eq!(TaxCategory::from_code("X"), None);
    }

    #[test]
    fn round2_normalises_negative_zero() {
        assert_eq!(round2(-0.001), 0.0);
        assert_eq!(round2(10.005), 10.01);
    }
}

use serde::{Deserialize, Serialize};
use time::Date;
use tpt_doc_core::DocError;

pub use crate::money::{Decimal, Money};

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
    /// Electronic address scheme (`cbc:EndpointID/@schemeID`, e.g. `"0088"` for GLN,
    /// `"0192"` for the Norwegian register). Required by BR-62 / BR-63.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_scheme: Option<String>,
    /// Registered legal name (`cac:PartyLegalEntity/cbc:RegistrationName`).
    /// Defaults to [`Party::name`] when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legal_name: Option<String>,
    /// Legal registration identifier (`cac:PartyLegalEntity/cbc:CompanyID`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legal_id: Option<String>,
}

impl Party {
    /// Create a party with the given legal name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            address: None,
            tax_id: None,
            endpoint_id: None,
            endpoint_scheme: None,
            legal_name: None,
            legal_id: None,
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
    pub quantity: Decimal,
    /// Price per unit, excluding tax.
    pub unit_price: Decimal,
    /// Line net amount (`quantity * unit_price`, excluding tax).
    pub line_extension_amount: Money,
    /// UN/ECE Recommendation 20 unit of measure (`cbc:InvoicedQuantity/@unitCode`),
    /// e.g. `"C62"` (one), `"HUR"` (hour), `"KGM"`. Required by BR-23.
    #[serde(default = "default_unit_code")]
    pub unit_code: String,
    /// How this line is taxed (`cac:ClassifiedTaxCategory`). Required by BR-CO-04.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax: Option<LineTax>,
}

fn default_unit_code() -> String {
    "C62".to_owned()
}

/// The tax category and rate applied to one invoice line.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LineTax {
    /// The tax category.
    pub category: TaxCategory,
    /// The rate as a percentage (e.g. `15` for 15%).
    pub rate_percent: Decimal,
}

/// How the invoice is paid (`cac:PaymentMeans`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentMeans {
    /// UN/EDIFACT 4461 payment means code (`"30"` credit transfer, `"58"` SEPA
    /// credit transfer, `"48"` card, …).
    pub code: String,
    /// Remittance information (`cbc:PaymentID`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<String>,
    /// Payee account identifier, e.g. an IBAN or NZ bank account
    /// (`cac:PayeeFinancialAccount/cbc:ID`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

impl InvoiceLine {
    /// Create a line, computing `line_extension_amount` from quantity and
    /// unit price, rounded half away from zero to cents.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if the product overflows.
    pub fn new(
        id: u32,
        item_name: impl Into<String>,
        quantity: Decimal,
        unit_price: Decimal,
    ) -> Result<Self, DocError> {
        let line_extension_amount = quantity
            .checked_mul(unit_price)
            .and_then(Decimal::round_to_money)
            .ok_or_else(|| DocError::invalid_format("line amount overflows"))?;
        Ok(Self {
            id,
            item_name: item_name.into(),
            quantity,
            unit_price,
            line_extension_amount,
            unit_code: default_unit_code(),
            tax: None,
        })
    }

    /// Set the unit of measure (UN/ECE Recommendation 20 code).
    #[must_use]
    pub fn with_unit_code(mut self, unit_code: impl Into<String>) -> Self {
        self.unit_code = unit_code.into();
        self
    }

    /// Set the tax category and rate applied to this line.
    #[must_use]
    pub fn with_tax(mut self, category: TaxCategory, rate_percent: Decimal) -> Self {
        self.tax = Some(LineTax {
            category,
            rate_percent,
        });
        self
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
    /// `K` — VAT exempt for EEA intra-community supply (IGIC/IPSI).
    EeaSupply,
    /// `G` — free export, outside the scope of VAT.
    FreeExport,
    /// `O` — services outside the scope of tax.
    OutsideScope,
    /// `L` — Canary Islands general indirect tax.
    CanaryIslands,
    /// `M` — tax for production, manufacturing and fishing in Ceuta and Melilla.
    CeutaMelilla,
}

impl TaxCategory {
    /// Whether the category charges tax at a positive rate (`S`, `L`, `M`).
    /// The others (`Z`, `E`, `AE`, `K`, `G`, `O`) carry a zero rate and zero tax.
    #[must_use]
    pub fn is_rate_based(&self) -> bool {
        matches!(
            self,
            Self::Standard | Self::CanaryIslands | Self::CeutaMelilla
        )
    }

    /// Whether a VAT exemption reason must accompany the breakdown.
    #[must_use]
    pub fn requires_exemption_reason(&self) -> bool {
        matches!(
            self,
            Self::Exempt
                | Self::ReverseCharge
                | Self::EeaSupply
                | Self::FreeExport
                | Self::OutsideScope
        )
    }

    /// The EN 16931 rule-id prefix for this category's breakdown rules
    /// (`BR-S`, `BR-Z`, `BR-E`, `BR-AE`, `BR-IC`, `BR-G`, `BR-O`, `BR-AF`, `BR-AG`).
    #[must_use]
    pub fn rule_prefix(&self) -> &'static str {
        match self {
            Self::Standard => "BR-S",
            Self::ZeroRated => "BR-Z",
            Self::Exempt => "BR-E",
            Self::ReverseCharge => "BR-AE",
            Self::EeaSupply => "BR-IC",
            Self::FreeExport => "BR-G",
            Self::OutsideScope => "BR-O",
            Self::CanaryIslands => "BR-AF",
            Self::CeutaMelilla => "BR-AG",
        }
    }

    /// The UN/EDIFACT 5305 code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Standard => "S",
            Self::ZeroRated => "Z",
            Self::Exempt => "E",
            Self::ReverseCharge => "AE",
            Self::EeaSupply => "K",
            Self::FreeExport => "G",
            Self::OutsideScope => "O",
            Self::CanaryIslands => "L",
            Self::CeutaMelilla => "M",
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
            "K" => Some(Self::EeaSupply),
            "G" => Some(Self::FreeExport),
            "O" => Some(Self::OutsideScope),
            "L" => Some(Self::CanaryIslands),
            "M" => Some(Self::CeutaMelilla),
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
    pub rate_percent: Decimal,
    /// Net amount the tax rate applies to.
    pub taxable_amount: Money,
    /// Tax amount due for this category.
    pub tax_amount: Money,
    /// Why no tax is charged (`cbc:TaxExemptionReason`). Required for the
    /// exempt-style categories by BR-E-10, BR-AE-10, BR-IC-10, BR-G-10, BR-O-10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exemption_reason: Option<String>,
}

impl TaxSubtotal {
    /// A subtotal with no exemption reason.
    #[must_use]
    pub fn new(
        category: TaxCategory,
        rate_percent: Decimal,
        taxable_amount: Money,
        tax_amount: Money,
    ) -> Self {
        Self {
            category,
            rate_percent,
            taxable_amount,
            tax_amount,
            exemption_reason: None,
        }
    }

    /// Set the exemption reason text.
    #[must_use]
    pub fn with_exemption_reason(mut self, reason: impl Into<String>) -> Self {
        self.exemption_reason = Some(reason.into());
        self
    }
}

/// The `LegalMonetaryTotal` block: document-level totals.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MonetaryTotal {
    /// Sum of invoice line net amounts (`LineExtensionAmount`, BT-106).
    pub line_extension: Money,
    /// Total net amount excluding tax (`TaxExclusiveAmount`).
    pub tax_exclusive: Money,
    /// Total gross amount including tax (`TaxInclusiveAmount`).
    pub tax_inclusive: Money,
    /// Amount payable (`PayableAmount`).
    pub payable: Money,
}

/// Issue date type alias: an ISO 8601 calendar date.
pub type IssueDate = Date;

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    #[test]
    fn line_computes_extension_amount() {
        let line = InvoiceLine::new(1, "Widget", d("3"), d("9.99")).unwrap();
        assert_eq!(line.line_extension_amount, Money::from_cents(2997));
    }

    #[test]
    fn line_rounds_to_two_decimals() {
        let line = InvoiceLine::new(1, "Loose", d("3"), d("0.335")).unwrap();
        assert_eq!(line.line_extension_amount, Money::from_cents(101));
    }

    #[test]
    fn line_amount_is_exact_where_floats_drift() {
        // 3 x 0.10 is 0.30000000000000004 in binary floating point.
        let line = InvoiceLine::new(1, "Pens", d("3"), d("0.10")).unwrap();
        assert_eq!(line.line_extension_amount, Money::from_cents(30));
    }

    #[test]
    fn line_overflow_is_an_error() {
        let huge = d("999999999999");
        assert!(InvoiceLine::new(1, "x", huge, huge).is_err());
    }

    #[test]
    fn tax_category_codes_round_trip() {
        for (code, category) in [
            ("S", TaxCategory::Standard),
            ("Z", TaxCategory::ZeroRated),
            ("E", TaxCategory::Exempt),
            ("AE", TaxCategory::ReverseCharge),
            ("K", TaxCategory::EeaSupply),
            ("G", TaxCategory::FreeExport),
            ("O", TaxCategory::OutsideScope),
            ("L", TaxCategory::CanaryIslands),
            ("M", TaxCategory::CeutaMelilla),
        ] {
            assert_eq!(TaxCategory::from_code(code), Some(category));
            assert_eq!(category.code(), code);
        }
        assert_eq!(TaxCategory::from_code("X"), None);
    }
}

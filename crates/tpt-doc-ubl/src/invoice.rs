use serde::{Deserialize, Serialize};
use tpt_doc_core::DocError;

use crate::types::{InvoiceLine, IssueDate, MonetaryTotal, Party, TaxSubtotal, round2};

/// UBL document type code (`cbc:InvoiceTypeCode` / `cbc:CreditNoteTypeCode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceTypeCode {
    /// `380` — commercial invoice.
    Invoice,
    /// `381` — credit note.
    CreditNote,
}

impl InvoiceTypeCode {
    /// The UBL type code value.
    #[must_use]
    pub fn value(&self) -> u16 {
        match self {
            Self::Invoice => 380,
            Self::CreditNote => 381,
        }
    }
}

/// A UBL 2.1 invoice covering the PEPPOL BIS Billing 3.0 mandatory elements:
/// document ID, issue date, supplier, customer, invoice lines, tax totals,
/// and legal monetary totals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invoice {
    /// `cbc:ID` — the invoice document number.
    pub id: String,
    /// `cbc:InvoiceTypeCode` — invoice or credit note.
    pub type_code: InvoiceTypeCode,
    /// `cbc:IssueDate` — the invoice issue date.
    pub issue_date: IssueDate,
    /// `cac:AccountingSupplierParty`.
    pub supplier: Party,
    /// `cac:AccountingCustomerParty`.
    pub customer: Party,
    /// `cac:InvoiceLine` items, in order.
    pub lines: Vec<InvoiceLine>,
    /// `cac:TaxSubtotal` entries, one per tax category.
    pub tax_subtotals: Vec<TaxSubtotal>,
    /// `cac:LegalMonetaryTotal`.
    pub monetary_total: MonetaryTotal,
    /// `cbc:DocumentCurrencyCode` (ISO 4217, e.g. `"NZD"`).
    pub currency: String,
}

impl Invoice {
    /// Create an invoice with no lines, zero totals, and `NZD` currency.
    ///
    /// Totals are recomputed automatically as lines and tax subtotals are
    /// added via [`push_line`](Self::push_line) and
    /// [`push_tax_subtotal`](Self::push_tax_subtotal).
    pub fn new(
        id: impl Into<String>,
        type_code: InvoiceTypeCode,
        issue_date: IssueDate,
        supplier: Party,
        customer: Party,
    ) -> Self {
        Self {
            id: id.into(),
            type_code,
            issue_date,
            supplier,
            customer,
            lines: Vec::new(),
            tax_subtotals: Vec::new(),
            monetary_total: MonetaryTotal::default(),
            currency: "NZD".to_owned(),
        }
    }

    /// Append an invoice line and recompute document totals.
    pub fn push_line(&mut self, line: InvoiceLine) {
        self.lines.push(line);
        self.recalculate_totals();
    }

    /// Append a tax subtotal and recompute document totals.
    pub fn push_tax_subtotal(&mut self, subtotal: TaxSubtotal) {
        self.tax_subtotals.push(subtotal);
        self.recalculate_totals();
    }

    /// The amount payable (`PayableAmount`).
    #[must_use]
    pub fn total(&self) -> f64 {
        self.monetary_total.payable
    }

    /// Recompute `LegalMonetaryTotal` from lines and tax subtotals:
    ///
    /// - `tax_exclusive` = Σ line net amounts (BR-CO-10)
    /// - `tax_inclusive` = `tax_exclusive` + Σ tax amounts (BR-CO-13)
    /// - `payable` = `tax_inclusive` (BR-CO-15)
    fn recalculate_totals(&mut self) {
        let tax_exclusive = round2(self.lines.iter().map(|l| l.line_extension_amount).sum());
        let tax_amount = round2(self.tax_subtotals.iter().map(|t| t.tax_amount).sum());
        let tax_inclusive = round2(tax_exclusive + tax_amount);
        self.monetary_total = MonetaryTotal {
            tax_exclusive,
            tax_inclusive,
            payable: tax_inclusive,
        };
    }

    /// Serialize to a UBL 2.1 `Invoice` XML document.
    ///
    /// # Errors
    /// Returns [`DocError`] if XML serialization fails.
    pub fn to_xml(&self) -> Result<Vec<u8>, DocError> {
        // TODO(phase-4): emit full UBL 2.1 document with correct namespaces
        // (urn:oasis:names:specification:ubl:schema:xsd:Invoice-2) and
        // PEPPOL BIS 3.0 profile/customization IDs.
        Err(DocError::invalid_format(
            "Invoice XML serialization not yet implemented",
        ))
    }

    /// Parse a UBL 2.1 `Invoice` XML document.
    ///
    /// # Errors
    /// Returns [`DocError`] if the XML is not a valid UBL invoice.
    pub fn from_xml(_bytes: &[u8]) -> Result<Self, DocError> {
        // TODO(phase-4): streaming quick-xml parser mapping cac:/cbc: elements
        // onto this struct.
        Err(DocError::invalid_format(
            "Invoice XML parsing not yet implemented",
        ))
    }
}

#[cfg(test)]
mod tests {
    // Monetary arithmetic is rounded to 2dp; exact f64 comparison is intentional.
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::types::{Address, TaxCategory};

    fn party(name: &str) -> Party {
        Party {
            address: Some(Address {
                city: Some("Wellington".to_owned()),
                country_code: Some("NZ".to_owned()),
                ..Address::default()
            }),
            tax_id: Some("123456789".to_owned()),
            endpoint_id: None,
            name: name.to_owned(),
        }
    }

    fn sample_invoice() -> Invoice {
        let date =
            time::Date::from_calendar_date(2026, time::Month::January, 15).expect("valid date");
        let mut inv = Invoice::new(
            "INV-001",
            InvoiceTypeCode::Invoice,
            date,
            party("Supplier Ltd"),
            party("Customer Ltd"),
        );
        inv.push_line(InvoiceLine::new(1, "Consulting", 2.0, 150.0));
        inv.push_line(InvoiceLine::new(2, "Hosting", 12.0, 10.0));
        inv.push_tax_subtotal(TaxSubtotal {
            category: TaxCategory::Standard,
            rate_percent: 15.0,
            taxable_amount: 420.0,
            tax_amount: 63.0,
        });
        inv
    }

    #[test]
    fn totals_recompute_from_lines_and_tax() {
        let inv = sample_invoice();
        assert_eq!(inv.monetary_total.tax_exclusive, 420.0);
        assert_eq!(inv.monetary_total.tax_inclusive, 483.0);
        assert_eq!(inv.total(), 483.0);
    }

    #[test]
    fn empty_invoice_has_zero_totals() {
        let date =
            time::Date::from_calendar_date(2026, time::Month::January, 15).expect("valid date");
        let inv = Invoice::new(
            "INV-002",
            InvoiceTypeCode::CreditNote,
            date,
            party("A"),
            party("B"),
        );
        assert_eq!(inv.total(), 0.0);
        assert_eq!(inv.type_code.value(), 381);
    }

    #[test]
    fn valid_invoice_passes_validation() {
        let inv = sample_invoice();
        assert!(crate::validate::validate(&inv).is_empty());
    }

    #[test]
    fn xml_round_trip_is_phase_4() {
        let inv = sample_invoice();
        assert!(Invoice::to_xml(&inv).is_err());
        assert!(Invoice::from_xml(b"<Invoice/>").is_err());
    }
}

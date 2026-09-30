use std::fmt;

use crate::invoice::Invoice;
use crate::types::round2;

/// A PEPPOL BIS Billing 3.0 / EN 16931 rule violation with the rule ID,
/// the field path that failed, and a human-readable message.
#[derive(Debug, Clone)]
pub struct UblValidationError {
    /// Business rule identifier (e.g. `"BR-CO-10"`).
    pub rule: &'static str,
    /// Path to the offending field (UBL `cbc:`/`cac:` element names).
    pub field_path: String,
    /// Human-readable description of the violation.
    pub message: String,
}

impl UblValidationError {
    /// Construct a new validation error.
    pub fn new(
        rule: &'static str,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            field_path: field_path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for UblValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.rule, self.field_path, self.message)
    }
}

impl std::error::Error for UblValidationError {}

/// Validate an invoice against the PEPPOL BIS Billing 3.0 arithmetic and
/// structural rules that can be checked without external schematron files:
///
/// - **BR-16** — an invoice must have at least one invoice line.
/// - **BR-CO-10** — sum of line net amounts equals `TaxExclusiveAmount`.
/// - **BR-CO-13** — `TaxInclusiveAmount` = `TaxExclusiveAmount` + Σ tax amounts.
/// - **BR-CO-15** — `PayableAmount` = `TaxInclusiveAmount`.
/// - **BR-CO-25** — each subtotal's tax amount equals its taxable amount
///   multiplied by its rate (where the category is rate-based).
///
/// Returns all violations found; an empty vector means the invoice is valid.
#[must_use]
// Monetary comparisons operate on values already rounded to 2 decimal places,
// so exact equality is the intended semantics.
#[allow(clippy::float_cmp)]
pub fn validate(invoice: &Invoice) -> Vec<UblValidationError> {
    let mut errors = Vec::new();

    if invoice.lines.is_empty() {
        errors.push(UblValidationError::new(
            "BR-16",
            "cac:InvoiceLine",
            "an invoice shall have at least one invoice line",
        ));
    }

    let line_sum = round2(invoice.lines.iter().map(|l| l.line_extension_amount).sum());
    if line_sum != round2(invoice.monetary_total.tax_exclusive) {
        errors.push(UblValidationError::new(
            "BR-CO-10",
            "cac:LegalMonetaryTotal/cbc:TaxExclusiveAmount",
            format!(
                "sum of line net amounts ({line_sum}) does not equal tax exclusive amount ({})",
                round2(invoice.monetary_total.tax_exclusive)
            ),
        ));
    }

    let tax_sum = round2(invoice.tax_subtotals.iter().map(|t| t.tax_amount).sum());
    let expected_inclusive = round2(invoice.monetary_total.tax_exclusive + tax_sum);
    if expected_inclusive != round2(invoice.monetary_total.tax_inclusive) {
        errors.push(UblValidationError::new(
            "BR-CO-13",
            "cac:LegalMonetaryTotal/cbc:TaxInclusiveAmount",
            format!(
                "tax inclusive amount ({}) does not equal tax exclusive + tax ({expected_inclusive})",
                round2(invoice.monetary_total.tax_inclusive)
            ),
        ));
    }

    if round2(invoice.monetary_total.payable) != round2(invoice.monetary_total.tax_inclusive) {
        errors.push(UblValidationError::new(
            "BR-CO-15",
            "cac:LegalMonetaryTotal/cbc:PayableAmount",
            format!(
                "payable amount ({}) does not equal tax inclusive amount ({})",
                round2(invoice.monetary_total.payable),
                round2(invoice.monetary_total.tax_inclusive)
            ),
        ));
    }

    for subtotal in &invoice.tax_subtotals {
        let expected_tax = round2(subtotal.taxable_amount * subtotal.rate_percent / 100.0);
        if expected_tax != round2(subtotal.tax_amount) {
            errors.push(UblValidationError::new(
                "BR-CO-25",
                "cac:TaxSubtotal/cbc:TaxAmount",
                format!(
                    "tax amount ({}) does not equal taxable amount x rate ({expected_tax})",
                    round2(subtotal.tax_amount)
                ),
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    // Monetary arithmetic is rounded to 2dp; exact f64 comparison is intentional.
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::invoice::InvoiceTypeCode;
    use crate::types::{Address, Party, TaxCategory, TaxSubtotal};

    fn party(name: &str) -> Party {
        Party {
            address: Some(Address {
                country_code: Some("NZ".to_owned()),
                ..Address::default()
            }),
            tax_id: None,
            endpoint_id: None,
            name: name.to_owned(),
        }
    }

    fn valid_invoice() -> Invoice {
        let date = time::Date::from_calendar_date(2026, time::Month::June, 1).expect("valid date");
        let mut inv = Invoice::new(
            "INV-100",
            InvoiceTypeCode::Invoice,
            date,
            party("Supplier"),
            party("Customer"),
        );
        inv.push_line(crate::types::InvoiceLine::new(1, "Item", 1.0, 100.0));
        inv.push_tax_subtotal(TaxSubtotal {
            category: TaxCategory::Standard,
            rate_percent: 15.0,
            taxable_amount: 100.0,
            tax_amount: 15.0,
        });
        inv
    }

    #[test]
    fn valid_invoice_has_no_errors() {
        assert!(validate(&valid_invoice()).is_empty());
    }

    #[test]
    fn empty_invoice_violates_br16() {
        let date = time::Date::from_calendar_date(2026, time::Month::June, 1).expect("valid date");
        let inv = Invoice::new(
            "INV-101",
            InvoiceTypeCode::Invoice,
            date,
            party("S"),
            party("C"),
        );
        let errors = validate(&inv);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "BR-16");
    }

    #[test]
    fn corrupted_payable_violates_br15() {
        let mut inv = valid_invoice();
        inv.monetary_total.payable = 999.0;
        let errors = validate(&inv);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "BR-CO-15");
    }

    #[test]
    fn wrong_subtotal_tax_violates_br25() {
        let mut inv = valid_invoice();
        inv.monetary_total.payable = 0.0;
        inv.monetary_total.tax_inclusive = 100.0;
        inv.monetary_total.tax_exclusive = 100.0;
        inv.tax_subtotals[0].tax_amount = 20.0;
        let errors = validate(&inv);
        let rules: Vec<_> = errors.iter().map(|e| e.rule).collect();
        assert!(rules.contains(&"BR-CO-25"));
        assert!(rules.contains(&"BR-CO-13"));
    }
}

use std::fmt;

use tpt_doc_core::{Location, Severity, ValidationIssue, ValidationReport};

use crate::invoice::Invoice;
use crate::money::Money;

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

impl From<&UblValidationError> for ValidationIssue {
    fn from(error: &UblValidationError) -> Self {
        ValidationIssue::new(error.field_path.clone(), error.message.clone())
            .with_rule(error.rule)
            // A UBL rule always rejects the document, so the severity is an
            // error; advisory PEPPOL warnings would be a separate rule set.
            .with_severity(Severity::Error)
            .with_location(Location::Path(error.field_path.clone()))
    }
}

impl UblValidationError {
    /// Convert a slice of violations into a structured [`ValidationReport`].
    ///
    /// This is the migration path towards the shared report type: the
    /// `Vec<UblValidationError>` API keeps working, while callers that want
    /// severity, machine-readable rule IDs, and serialization can move over
    /// without waiting for every format to drop its own error type.
    #[must_use]
    pub fn to_report(errors: &[Self]) -> ValidationReport {
        let mut report = ValidationReport::default();
        for error in errors {
            report.push_issue(ValidationIssue::from(error));
        }
        report
    }
}

/// Validate an invoice against the PEPPOL BIS Billing 3.0 arithmetic and
/// structural rules that can be checked without external schematron files:
///
/// - **BR-01, BR-02, BR-05, BR-16** — specification id, invoice number, currency, at least one line.
/// - **BR-06..BR-11** — seller / buyer name, postal address and country code.
/// - **BR-23, BR-CO-04** — every line has a unit of measure and a tax category.
/// - **BR-49** — payment instructions, when present, name a payment means code.
/// - **BR-62, BR-63, PEPPOL-EN16931-R010 / R020** — electronic addresses and their scheme.
/// - **BR-CO-26** — the seller carries some identifier.
/// - **PEPPOL-EN16931-R001 / R003** — business process and buyer reference.
/// - **BR-CO-10** — Σ line net amounts equals the declared `LineExtensionAmount`.
/// - **BR-CO-13** — `TaxExclusiveAmount` equals the line total (no allowances or charges yet).
/// - **BR-CO-15** — `TaxInclusiveAmount` = `TaxExclusiveAmount` + Σ tax amounts.
/// - **BR-CO-16** — `PayableAmount` = `TaxInclusiveAmount`.
/// - **BR-CO-17** — each breakdown's tax amount equals taxable amount × rate.
/// - **BR-CO-18** and the per-category families **BR-S / Z / E / AE / IC / G / O / AF / AG**
///   `-01`, `-05`, `-08`, `-09`, `-10` — the VAT breakdown (see the `vat` module docs).
///
/// Returns all violations found; an empty vector means the invoice is valid.
#[must_use]
pub fn validate(invoice: &Invoice) -> Vec<UblValidationError> {
    let mut errors = Vec::new();
    check_document(invoice, &mut errors);
    check_parties(invoice, &mut errors);
    check_lines(invoice, &mut errors);
    check_totals(invoice, &mut errors);
    errors
}

/// Document-level identification and payment instructions.
fn check_document(invoice: &Invoice, errors: &mut Vec<UblValidationError>) {
    if invoice.lines.is_empty() {
        errors.push(UblValidationError::new(
            "BR-16",
            "cac:InvoiceLine",
            "an invoice shall have at least one invoice line",
        ));
    }

    // EN 16931 BR-01 / PEPPOL BIS: the specification identifier (BT-24).
    if invoice.customization_id.trim().is_empty() {
        errors.push(UblValidationError::new(
            "BR-01",
            "cbc:CustomizationID",
            "an invoice shall have a specification identifier (customization ID)",
        ));
    }
    // PEPPOL-EN16931-R001: business process (profile, BT-23) must be provided.
    if invoice.profile_id.trim().is_empty() {
        errors.push(UblValidationError::new(
            "PEPPOL-EN16931-R001",
            "cbc:ProfileID",
            "a PEPPOL invoice shall have a business process (profile) identifier",
        ));
    }
    // BR-02 / BR-05: the document must be identifiable and have a currency.
    if invoice.id.trim().is_empty() {
        errors.push(UblValidationError::new(
            "BR-02",
            "cbc:ID",
            "an invoice shall have an invoice number",
        ));
    }
    if invoice.currency.trim().is_empty() {
        errors.push(UblValidationError::new(
            "BR-05",
            "cbc:DocumentCurrencyCode",
            "an invoice shall have a document currency code",
        ));
    }
    // PEPPOL-EN16931-R003: a buyer reference (BT-10) or order reference must
    // be present so the buyer can route and match the invoice.
    if invoice
        .buyer_reference
        .as_deref()
        .is_none_or(|r| r.trim().is_empty())
    {
        errors.push(UblValidationError::new(
            "PEPPOL-EN16931-R003",
            "cbc:BuyerReference",
            "an invoice shall have a buyer reference",
        ));
    }

    // BR-49: payment instructions, when given, must say how the payment is made.
    if let Some(means) = &invoice.payment_means
        && means.code.trim().is_empty()
    {
        errors.push(UblValidationError::new(
            "BR-49",
            "cac:PaymentMeans/cbc:PaymentMeansCode",
            "payment instructions shall specify a payment means type code",
        ));
    }
}

/// Seller and buyer: name, address, electronic address, seller identification.
fn check_parties(invoice: &Invoice, errors: &mut Vec<UblValidationError>) {
    // Seller and buyer: name (BR-06/07), postal address with a country
    // (BR-08..11), and an electronic address with its scheme (PEPPOL R020/R010,
    // BR-62/63). BR-CO-26: the seller must carry some kind of identifier.
    for (role, party, name_rule, address_rule, country_rule, endpoint_rule, scheme_rule) in [
        (
            "seller",
            &invoice.supplier,
            "BR-06",
            "BR-08",
            "BR-09",
            "PEPPOL-EN16931-R020",
            "BR-62",
        ),
        (
            "buyer",
            &invoice.customer,
            "BR-07",
            "BR-10",
            "BR-11",
            "PEPPOL-EN16931-R010",
            "BR-63",
        ),
    ] {
        let blank = |v: &Option<String>| v.as_deref().is_none_or(|s| s.trim().is_empty());
        if party.name.trim().is_empty() {
            errors.push(UblValidationError::new(
                name_rule,
                "cac:PartyName/cbc:Name",
                format!("the {role} shall have a name"),
            ));
        }
        match &party.address {
            None => errors.push(UblValidationError::new(
                address_rule,
                "cac:PostalAddress",
                format!("the {role} shall have a postal address"),
            )),
            Some(address) if blank(&address.country_code) => {
                errors.push(UblValidationError::new(
                    country_rule,
                    "cac:PostalAddress/cac:Country/cbc:IdentificationCode",
                    format!("the {role} postal address shall have a country code"),
                ));
            }
            Some(_) => {}
        }
        if blank(&party.endpoint_id) {
            errors.push(UblValidationError::new(
                endpoint_rule,
                "cbc:EndpointID",
                format!("the {role} shall have an electronic address"),
            ));
        } else if blank(&party.endpoint_scheme) {
            errors.push(UblValidationError::new(
                scheme_rule,
                "cbc:EndpointID/@schemeID",
                format!("the {role} electronic address shall have a scheme identifier"),
            ));
        }
    }
    if [
        &invoice.supplier.tax_id,
        &invoice.supplier.legal_id,
        &invoice.supplier.endpoint_id,
    ]
    .iter()
    .all(|v| v.as_deref().is_none_or(|s| s.trim().is_empty()))
    {
        errors.push(UblValidationError::new(
            "BR-CO-26",
            "cac:AccountingSupplierParty",
            "the seller shall have a tax identifier, a legal registration identifier or an endpoint",
        ));
    }
}

/// Per-line unit of measure and tax categorisation.
fn check_lines(invoice: &Invoice, errors: &mut Vec<UblValidationError>) {
    // Per line: BR-23 unit of measure, BR-CO-04 tax category.
    for line in &invoice.lines {
        if line.unit_code.trim().is_empty() {
            errors.push(UblValidationError::new(
                "BR-23",
                "cbc:InvoicedQuantity/@unitCode",
                format!("line {} shall have a unit of measure code", line.id),
            ));
        }
        if line.tax.is_none() {
            errors.push(UblValidationError::new(
                "BR-CO-04",
                "cac:Item/cac:ClassifiedTaxCategory",
                format!("line {} shall be categorised for tax", line.id),
            ));
        }
    }
}

/// Arithmetic rules over the monetary totals and tax subtotals.
fn check_totals(invoice: &Invoice, errors: &mut Vec<UblValidationError>) {
    let totals = &invoice.monetary_total;
    let mut check = |rule: &'static str, path: &'static str, ok: bool, message: String| {
        if !ok {
            errors.push(UblValidationError::new(rule, path, message));
        }
    };

    let line_sum: Money = invoice.lines.iter().map(|l| l.line_extension_amount).sum();
    check(
        "BR-CO-10",
        "cac:LegalMonetaryTotal/cbc:LineExtensionAmount",
        line_sum == totals.line_extension,
        format!(
            "sum of line net amounts ({line_sum}) does not equal the declared line extension amount ({})",
            totals.line_extension
        ),
    );
    // No document-level allowances or charges are modelled, so the net total
    // is exactly the line total.
    check(
        "BR-CO-13",
        "cac:LegalMonetaryTotal/cbc:TaxExclusiveAmount",
        totals.tax_exclusive == totals.line_extension,
        format!(
            "tax exclusive amount ({}) does not equal the line total ({})",
            totals.tax_exclusive, totals.line_extension
        ),
    );
    let tax_sum: Money = invoice.tax_subtotals.iter().map(|t| t.tax_amount).sum();
    let expected_inclusive = totals.tax_exclusive + tax_sum;
    check(
        "BR-CO-15",
        "cac:LegalMonetaryTotal/cbc:TaxInclusiveAmount",
        expected_inclusive == totals.tax_inclusive,
        format!(
            "tax inclusive amount ({}) does not equal tax exclusive + tax ({expected_inclusive})",
            totals.tax_inclusive
        ),
    );
    check(
        "BR-CO-16",
        "cac:LegalMonetaryTotal/cbc:PayableAmount",
        totals.payable == totals.tax_inclusive,
        format!(
            "payable amount ({}) does not equal tax inclusive amount ({})",
            totals.payable, totals.tax_inclusive
        ),
    );

    for subtotal in &invoice.tax_subtotals {
        let expected_tax = subtotal.rate_percent.percent_of(subtotal.taxable_amount);
        let shown = expected_tax.map_or_else(|| "overflow".to_owned(), |t| t.to_string());
        check(
            "BR-CO-17",
            "cac:TaxSubtotal/cbc:TaxAmount",
            expected_tax == Some(subtotal.tax_amount),
            format!(
                "tax amount ({}) does not equal taxable amount x rate ({shown})",
                subtotal.tax_amount
            ),
        );
    }

    crate::vat::check(invoice, errors);
}

#[cfg(test)]
mod tests {

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
            endpoint_id: Some("9429041234567".to_owned()),
            endpoint_scheme: Some("0088".to_owned()),
            legal_name: None,
            legal_id: Some("9429041234567".to_owned()),
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
        inv.buyer_reference = Some("PO-1".to_owned());
        inv.push_line(
            crate::types::InvoiceLine::new(1, "Item", 1.into(), 100.into())
                .unwrap()
                .with_tax(TaxCategory::Standard, 15.into()),
        );
        inv.push_tax_subtotal(TaxSubtotal {
            category: TaxCategory::Standard,
            rate_percent: 15.into(),
            taxable_amount: Money::from_cents(10_000),
            tax_amount: Money::from_cents(1_500),
            exemption_reason: None,
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
        let mut inv = Invoice::new(
            "INV-101",
            InvoiceTypeCode::Invoice,
            date,
            party("S"),
            party("C"),
        );
        inv.buyer_reference = Some("PO-1".to_owned());
        let errors = validate(&inv);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "BR-16");
    }

    #[test]
    fn corrupted_payable_violates_br_co_16() {
        let mut inv = valid_invoice();
        inv.monetary_total.payable = Money::from_cents(99_900);
        let errors = validate(&inv);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "BR-CO-16");
    }

    #[test]
    fn wrong_subtotal_tax_violates_br_co_17() {
        let mut inv = valid_invoice();
        inv.monetary_total.payable = Money::ZERO;
        inv.monetary_total.tax_inclusive = Money::from_cents(10_000);
        inv.monetary_total.tax_exclusive = Money::from_cents(10_000);
        inv.tax_subtotals[0].tax_amount = Money::from_cents(2_000);
        let errors = validate(&inv);
        let rules: Vec<_> = errors.iter().map(|e| e.rule).collect();
        assert!(rules.contains(&"BR-CO-17"), "{rules:?}");
        assert!(rules.contains(&"BR-CO-15"), "{rules:?}");
    }
}

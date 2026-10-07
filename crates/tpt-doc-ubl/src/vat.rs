//! EN 16931 VAT breakdown rules (BG-23) and the per-category rule families.
//!
//! Each tax category has a family of rules sharing a suffix; the prefix comes
//! from [`TaxCategory::rule_prefix`] (`BR-S`, `BR-Z`, `BR-E`, `BR-AE`, `BR-IC`,
//! `BR-G`, `BR-O`, `BR-AF`, `BR-AG`):
//!
//! | Rule | Meaning |
//! |---|---|
//! | `…-01` | a line in category X requires a breakdown in category X |
//! | `…-05` | line rate: `> 0` for `S`, `>= 0` for `L`/`M`, `= 0` otherwise |
//! | `…-08` | breakdown taxable amount = Σ net amounts of its lines |
//! | `…-09` | zero-rate categories carry zero tax (rate-based ones are BR-CO-17) |
//! | `…-10` | exempt-style categories state an exemption reason |
//!
//! Not modelled yet, so not checked: VAT identifiers of the parties
//! (`BR-AE-02..04`, `BR-IC-02..04`), delivery date / country (`BR-IC-11/12`),
//! and document-level allowances and charges.

use crate::Invoice;
use crate::money::{Decimal, Money};
use crate::types::TaxCategory;
use crate::validate::UblValidationError;

/// Run every VAT breakdown rule, appending violations to `errors`.
pub(crate) fn check(invoice: &Invoice, errors: &mut Vec<UblValidationError>) {
    if !invoice.lines.is_empty() && invoice.tax_subtotals.is_empty() {
        errors.push(UblValidationError::new(
            "BR-CO-18",
            "cac:TaxTotal/cac:TaxSubtotal",
            "an invoice shall have at least one VAT breakdown",
        ));
    }
    check_lines(invoice, errors);
    check_breakdowns(invoice, errors);
}

/// `…-01` (category present in the breakdown) and `…-05` (line rate).
fn check_lines(invoice: &Invoice, errors: &mut Vec<UblValidationError>) {
    let mut reported: Vec<TaxCategory> = Vec::new();
    for line in &invoice.lines {
        let Some(tax) = &line.tax else { continue };
        let prefix = tax.category.rule_prefix();

        let has_breakdown = invoice
            .tax_subtotals
            .iter()
            .any(|s| s.category == tax.category);
        if !has_breakdown && !reported.contains(&tax.category) {
            reported.push(tax.category);
            errors.push(UblValidationError::new(
                rule(prefix, "01"),
                "cac:TaxTotal/cac:TaxSubtotal/cac:TaxCategory/cbc:ID",
                format!(
                    "line {} uses tax category {} but the VAT breakdown has none",
                    line.id,
                    tax.category.code()
                ),
            ));
        }

        let rate_ok = match tax.category {
            TaxCategory::Standard => tax.rate_percent > Decimal::ZERO,
            TaxCategory::CanaryIslands | TaxCategory::CeutaMelilla => {
                tax.rate_percent >= Decimal::ZERO
            }
            _ => tax.rate_percent == Decimal::ZERO,
        };
        if !rate_ok {
            errors.push(UblValidationError::new(
                rule(prefix, "05"),
                "cac:ClassifiedTaxCategory/cbc:Percent",
                format!(
                    "line {} has rate {} which is not allowed for category {}",
                    line.id,
                    tax.rate_percent,
                    tax.category.code()
                ),
            ));
        }
    }
}

/// `…-08`, `…-09` and `…-10` for each breakdown.
fn check_breakdowns(invoice: &Invoice, errors: &mut Vec<UblValidationError>) {
    for subtotal in &invoice.tax_subtotals {
        let category = subtotal.category;
        let prefix = category.rule_prefix();

        // The taxable amount is the net of the lines in the same category (and,
        // for rate-based categories, at the same rate).
        let expected: Money = invoice
            .lines
            .iter()
            .filter_map(|line| line.tax.as_ref().map(|tax| (line, tax)))
            .filter(|(_, tax)| {
                tax.category == category
                    && (!category.is_rate_based() || tax.rate_percent == subtotal.rate_percent)
            })
            .map(|(line, _)| line.line_extension_amount)
            .sum();
        if expected != subtotal.taxable_amount {
            errors.push(UblValidationError::new(
                rule(prefix, "08"),
                "cac:TaxSubtotal/cbc:TaxableAmount",
                format!(
                    "taxable amount ({}) of category {} at {}% does not equal the net of its lines ({expected})",
                    subtotal.taxable_amount,
                    category.code(),
                    subtotal.rate_percent
                ),
            ));
        }

        if !category.is_rate_based()
            && (subtotal.tax_amount != Money::ZERO || subtotal.rate_percent != Decimal::ZERO)
        {
            errors.push(UblValidationError::new(
                rule(prefix, "09"),
                "cac:TaxSubtotal/cbc:TaxAmount",
                format!(
                    "category {} has a zero rate and zero tax, found {}% and {}",
                    category.code(),
                    subtotal.rate_percent,
                    subtotal.tax_amount
                ),
            ));
        }

        if category.requires_exemption_reason()
            && subtotal
                .exemption_reason
                .as_deref()
                .is_none_or(|r| r.trim().is_empty())
        {
            errors.push(UblValidationError::new(
                rule(prefix, "10"),
                "cac:TaxCategory/cbc:TaxExemptionReason",
                format!(
                    "category {} shall state a VAT exemption reason",
                    category.code()
                ),
            ));
        }
    }
}

/// Build a rule id such as `BR-S-08`. Ids are `'static` so errors stay cheap.
fn rule(prefix: &'static str, suffix: &'static str) -> &'static str {
    // Every combination is listed so the id is a literal, not a leaked string.
    match (prefix, suffix) {
        ("BR-S", "01") => "BR-S-01",
        ("BR-S", "05") => "BR-S-05",
        ("BR-S", "08") => "BR-S-08",
        ("BR-S", "09") => "BR-S-09",
        ("BR-S", "10") => "BR-S-10",
        ("BR-Z", "01") => "BR-Z-01",
        ("BR-Z", "05") => "BR-Z-05",
        ("BR-Z", "08") => "BR-Z-08",
        ("BR-Z", "09") => "BR-Z-09",
        ("BR-Z", "10") => "BR-Z-10",
        ("BR-E", "01") => "BR-E-01",
        ("BR-E", "05") => "BR-E-05",
        ("BR-E", "08") => "BR-E-08",
        ("BR-E", "09") => "BR-E-09",
        ("BR-E", "10") => "BR-E-10",
        ("BR-AE", "01") => "BR-AE-01",
        ("BR-AE", "05") => "BR-AE-05",
        ("BR-AE", "08") => "BR-AE-08",
        ("BR-AE", "09") => "BR-AE-09",
        ("BR-AE", "10") => "BR-AE-10",
        ("BR-IC", "01") => "BR-IC-01",
        ("BR-IC", "05") => "BR-IC-05",
        ("BR-IC", "08") => "BR-IC-08",
        ("BR-IC", "09") => "BR-IC-09",
        ("BR-IC", "10") => "BR-IC-10",
        ("BR-G", "01") => "BR-G-01",
        ("BR-G", "05") => "BR-G-05",
        ("BR-G", "08") => "BR-G-08",
        ("BR-G", "09") => "BR-G-09",
        ("BR-G", "10") => "BR-G-10",
        ("BR-O", "01") => "BR-O-01",
        ("BR-O", "05") => "BR-O-05",
        ("BR-O", "08") => "BR-O-08",
        ("BR-O", "09") => "BR-O-09",
        ("BR-O", "10") => "BR-O-10",
        ("BR-AF", "01") => "BR-AF-01",
        ("BR-AF", "05") => "BR-AF-05",
        ("BR-AF", "08") => "BR-AF-08",
        ("BR-AF", "09") => "BR-AF-09",
        ("BR-AF", "10") => "BR-AF-10",
        ("BR-AG", "01") => "BR-AG-01",
        ("BR-AG", "05") => "BR-AG-05",
        ("BR-AG", "08") => "BR-AG-08",
        ("BR-AG", "09") => "BR-AG-09",
        ("BR-AG", "10") => "BR-AG-10",
        _ => "BR-CO-18",
    }
}

//! Integration tests: parse the PEPPOL BIS fixture, validate it, and prove
//! serialize → parse round-trips.

use tpt_doc_ubl::prelude::*;

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/peppol_invoice.xml"
    ))
    .expect("fixture file")
}

#[test]
fn parses_peppol_fixture_and_asserts_values() {
    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    assert_eq!(invoice.supplier.name, "Wellington Data Systems Ltd");
    assert_eq!(invoice.customer.name, "Auckland Analytics Ltd");
    assert_eq!(invoice.lines.len(), 2);
    assert_eq!(invoice.monetary_total.payable, 483.0);
    assert_eq!(invoice.monetary_total.tax_exclusive, 420.0);
    assert_eq!(invoice.tax_subtotals.len(), 1);
    assert_eq!(invoice.tax_subtotals[0].category.code(), "S");
    assert_eq!(invoice.lines[0].item_name, "Consulting");
    assert_eq!(invoice.currency, "NZD");
}

#[test]
fn fixture_passes_peppol_validation() {
    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    let violations = validate(&invoice);
    assert!(violations.is_empty(), "violations: {violations:?}");
}

#[test]
fn fixture_survives_our_serializer_round_trip() {
    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    let bytes = invoice.to_xml().expect("serialize");
    let restored = Invoice::from_xml(&bytes).expect("re-parse");
    assert_eq!(invoice, restored);
}

#[test]
fn tampered_totals_are_caught_by_validation() {
    let mut invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    invoice.monetary_total.payable = 9999.0;
    let violations = validate(&invoice);
    assert!(
        violations.iter().any(|v| v.rule == "BR-CO-15"),
        "expected BR-CO-15, got {violations:?}"
    );
}

#[test]
fn missing_peppol_identifiers_are_reported() {
    let date = time::Date::from_calendar_date(2026, time::Month::June, 1).expect("valid date");
    let mut invoice = Invoice::new(
        "INV-1",
        InvoiceTypeCode::Invoice,
        date,
        Party::new("Supplier"),
        Party::new("Customer"),
    );
    invoice.customization_id.clear();
    invoice.profile_id.clear();
    invoice.supplier.endpoint_id = None;
    invoice.push_line(InvoiceLine::new(1, "item", 1.0, 10.0));
    let violations = validate(&invoice);
    let rules: Vec<_> = violations.iter().map(|v| v.rule).collect();
    assert!(rules.contains(&"BR-01"), "{rules:?}");
    assert!(rules.contains(&"PEPPOL-EN16931-R001"), "{rules:?}");
    assert!(rules.contains(&"PEPPOL-EN16931-R062"), "{rules:?}");
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn xml_safe(s: &str) -> bool {
        s.chars().all(|c| {
            matches!(
                c,
                '\t' | '\n'
                    | '\r'
                    | '\u{20}'..='\u{D7FF}'
                    | '\u{E000}'..='\u{FFFD}'
                    | '\u{10000}'..='\u{10FFFF}'
            )
        })
    }

    fn arb_text(max: usize) -> impl Strategy<Value = String> {
        any::<String>().prop_filter("xml-safe", move |s| xml_safe(s) && s.len() < max)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        #[test]
        fn invoice_round_trips_through_xml(
            id in arb_text(16),
            supplier_name in arb_text(24),
            customer_name in arb_text(24),
            item in arb_text(16),
            quantity in any::<f64>().prop_filter("sane", |q: &f64| *q > 0.0 && *q < 1e9),
            unit_price_cents in 1u64..1_000_000_000u64,
        ) {
            let date = time::Date::from_calendar_date(2026, time::Month::January, 15).expect("valid date");
            let mut invoice = Invoice::new(
                id,
                InvoiceTypeCode::Invoice,
                date,
                Party::new(supplier_name),
                Party::new(customer_name),
            );
            // Monetary amounts are 2-decimal by nature of the format.
            let unit_price = unit_price_cents as f64 / 100.0;
            invoice.push_line(InvoiceLine::new(1, item, quantity, unit_price));
            invoice.push_tax_subtotal(TaxSubtotal {
                category: TaxCategory::Standard,
                rate_percent: 15.0,
                taxable_amount: invoice.monetary_total.tax_exclusive,
                tax_amount: (invoice.monetary_total.tax_exclusive * 0.15 * 100.0).round() / 100.0,
            });

            let bytes = invoice.to_xml().expect("ok");
            let restored = Invoice::from_xml(&bytes).expect("ok");
            prop_assert_eq!(restored, invoice);
        }
    }
}

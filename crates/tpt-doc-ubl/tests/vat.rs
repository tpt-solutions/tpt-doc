//! EN 16931 VAT breakdown rules (BG-23): one scenario per rule family.

use tpt_doc_ubl::prelude::*;

fn party(name: &str) -> Party {
    let mut p = Party::new(name);
    p.endpoint_id = Some("9429041234567".to_owned());
    p.endpoint_scheme = Some("0088".to_owned());
    p.address = Some(Address {
        country_code: Some("NZ".to_owned()),
        ..Address::default()
    });
    p
}

/// A compliant one-line invoice for `category` at `rate`, net 100.00.
fn invoice(category: TaxCategory, rate: i32) -> Invoice {
    let date = time::Date::from_calendar_date(2026, time::Month::June, 1).expect("date");
    let mut inv = Invoice::new(
        "INV-V",
        InvoiceTypeCode::Invoice,
        date,
        party("S"),
        party("C"),
    );
    inv.buyer_reference = Some("PO-1".to_owned());
    inv.push_line(
        InvoiceLine::new(1, "Item", 1.into(), 100.into())
            .expect("line")
            .with_tax(category, rate.into()),
    );
    let net = Money::from_cents(10_000);
    let tax = Decimal::from(rate).percent_of(net).expect("tax");
    let mut sub = TaxSubtotal::new(category, rate.into(), net, tax);
    if category.requires_exemption_reason() {
        sub = sub.with_exemption_reason("Exempt under section 11");
    }
    inv.push_tax_subtotal(sub);
    inv
}

fn rules(invoice: &Invoice) -> Vec<&'static str> {
    validate(invoice).iter().map(|e| e.rule).collect()
}

#[test]
fn every_category_has_a_compliant_form() {
    for (category, rate) in [
        (TaxCategory::Standard, 15),
        (TaxCategory::ZeroRated, 0),
        (TaxCategory::Exempt, 0),
        (TaxCategory::ReverseCharge, 0),
        (TaxCategory::EeaSupply, 0),
        (TaxCategory::FreeExport, 0),
        (TaxCategory::OutsideScope, 0),
        (TaxCategory::CanaryIslands, 7),
        (TaxCategory::CeutaMelilla, 10),
    ] {
        let inv = invoice(category, rate);
        assert!(rules(&inv).is_empty(), "{category:?}: {:?}", validate(&inv));
    }
}

#[test]
fn category_prefixes_follow_en16931() {
    let expected = [
        (TaxCategory::Standard, "BR-S"),
        (TaxCategory::ZeroRated, "BR-Z"),
        (TaxCategory::Exempt, "BR-E"),
        (TaxCategory::ReverseCharge, "BR-AE"),
        (TaxCategory::EeaSupply, "BR-IC"),
        (TaxCategory::FreeExport, "BR-G"),
        (TaxCategory::OutsideScope, "BR-O"),
        (TaxCategory::CanaryIslands, "BR-AF"),
        (TaxCategory::CeutaMelilla, "BR-AG"),
    ];
    for (category, prefix) in expected {
        assert_eq!(category.rule_prefix(), prefix);
    }
}

#[test]
fn a_line_category_without_a_breakdown_is_br_x_01() {
    for (category, rate, rule) in [
        (TaxCategory::Standard, 15, "BR-S-01"),
        (TaxCategory::ZeroRated, 0, "BR-Z-01"),
        (TaxCategory::Exempt, 0, "BR-E-01"),
        (TaxCategory::ReverseCharge, 0, "BR-AE-01"),
        (TaxCategory::EeaSupply, 0, "BR-IC-01"),
        (TaxCategory::FreeExport, 0, "BR-G-01"),
        (TaxCategory::OutsideScope, 0, "BR-O-01"),
    ] {
        let mut inv = invoice(category, rate);
        // Replace the breakdown with a different category.
        inv.tax_subtotals.clear();
        let other = if category == TaxCategory::Standard {
            TaxCategory::ZeroRated
        } else {
            TaxCategory::Standard
        };
        inv.push_tax_subtotal(TaxSubtotal::new(
            other,
            0.into(),
            Money::from_cents(10_000),
            Money::ZERO,
        ));
        assert!(rules(&inv).contains(&rule), "{rule} in {:?}", rules(&inv));
    }
}

#[test]
fn no_breakdown_at_all_is_br_co_18() {
    let mut inv = invoice(TaxCategory::Standard, 15);
    inv.tax_subtotals.clear();
    assert!(rules(&inv).contains(&"BR-CO-18"));
}

#[test]
fn line_rates_must_suit_the_category() {
    // Standard-rated at zero is not standard rated.
    let mut inv = invoice(TaxCategory::Standard, 15);
    inv.lines[0].tax = Some(LineTax {
        category: TaxCategory::Standard,
        rate_percent: 0.into(),
    });
    assert!(rules(&inv).contains(&"BR-S-05"));

    // A zero-rated line carrying a rate.
    let mut inv = invoice(TaxCategory::ZeroRated, 0);
    inv.lines[0].tax = Some(LineTax {
        category: TaxCategory::ZeroRated,
        rate_percent: 15.into(),
    });
    assert!(rules(&inv).contains(&"BR-Z-05"));

    // Canary Islands tax may be zero, but never negative.
    let mut inv = invoice(TaxCategory::CanaryIslands, 7);
    inv.lines[0].tax = Some(LineTax {
        category: TaxCategory::CanaryIslands,
        rate_percent: Decimal::from(-1),
    });
    assert!(rules(&inv).contains(&"BR-AF-05"));
}

#[test]
fn breakdown_taxable_amount_must_equal_its_lines() {
    let mut inv = invoice(TaxCategory::Standard, 15);
    inv.tax_subtotals[0].taxable_amount = Money::from_cents(9_000);
    assert!(rules(&inv).contains(&"BR-S-08"));

    // Two rates under one category: each breakdown sums only its own lines.
    let date = time::Date::from_calendar_date(2026, time::Month::June, 1).expect("date");
    let mut inv = Invoice::new(
        "INV-2",
        InvoiceTypeCode::Invoice,
        date,
        party("S"),
        party("C"),
    );
    inv.buyer_reference = Some("PO-1".to_owned());
    for (id, rate) in [(1, 15), (2, 10)] {
        inv.push_line(
            InvoiceLine::new(id, "Item", 1.into(), 100.into())
                .expect("line")
                .with_tax(TaxCategory::Standard, rate.into()),
        );
    }
    for rate in [15, 10] {
        let net = Money::from_cents(10_000);
        inv.push_tax_subtotal(TaxSubtotal::new(
            TaxCategory::Standard,
            rate.into(),
            net,
            Decimal::from(rate).percent_of(net).expect("tax"),
        ));
    }
    assert!(rules(&inv).is_empty(), "{:?}", validate(&inv));
    inv.tax_subtotals[1].taxable_amount = Money::from_cents(20_000);
    assert!(rules(&inv).contains(&"BR-S-08"));
}

#[test]
fn zero_rate_categories_carry_no_tax() {
    for (category, rule) in [
        (TaxCategory::ZeroRated, "BR-Z-09"),
        (TaxCategory::Exempt, "BR-E-09"),
        (TaxCategory::ReverseCharge, "BR-AE-09"),
        (TaxCategory::EeaSupply, "BR-IC-09"),
        (TaxCategory::FreeExport, "BR-G-09"),
        (TaxCategory::OutsideScope, "BR-O-09"),
    ] {
        let mut inv = invoice(category, 0);
        inv.tax_subtotals[0].rate_percent = 15.into();
        inv.tax_subtotals[0].tax_amount = Money::from_cents(1_500);
        assert!(rules(&inv).contains(&rule), "{rule}");
    }
}

#[test]
fn exempt_style_categories_need_a_reason() {
    for (category, rule) in [
        (TaxCategory::Exempt, "BR-E-10"),
        (TaxCategory::ReverseCharge, "BR-AE-10"),
        (TaxCategory::EeaSupply, "BR-IC-10"),
        (TaxCategory::FreeExport, "BR-G-10"),
        (TaxCategory::OutsideScope, "BR-O-10"),
    ] {
        let mut inv = invoice(category, 0);
        inv.tax_subtotals[0].exemption_reason = None;
        assert!(rules(&inv).contains(&rule), "{rule}");
        inv.tax_subtotals[0].exemption_reason = Some("   ".to_owned());
        assert!(rules(&inv).contains(&rule), "blank reason: {rule}");
    }
    // Zero-rated goods need no reason.
    let inv = invoice(TaxCategory::ZeroRated, 0);
    assert!(!rules(&inv).iter().any(|r| r.ends_with("-10")));
}

#[test]
fn exemption_reason_round_trips_through_xml() {
    let inv = invoice(TaxCategory::Exempt, 0);
    let xml = String::from_utf8(inv.to_xml().expect("xml")).expect("utf8");
    assert!(
        xml.contains("<cbc:TaxExemptionReason>Exempt under section 11</cbc:TaxExemptionReason>"),
        "{xml}"
    );
    // Schema order inside TaxCategory: ID, Percent, TaxExemptionReason, TaxScheme.
    let id = xml.find("<cbc:Percent>").expect("percent");
    let reason = xml.find("<cbc:TaxExemptionReason>").expect("reason");
    let scheme = xml[reason..].find("<cac:TaxScheme>").expect("scheme") + reason;
    assert!(id < reason && reason < scheme);
    let back = Invoice::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(back, inv);
    assert!(rules(&back).is_empty());
}

#[test]
fn line_extension_total_is_written_and_checked() {
    let inv = invoice(TaxCategory::Standard, 15);
    let xml = String::from_utf8(inv.to_xml().expect("xml")).expect("utf8");
    let total = xml.find("<cac:LegalMonetaryTotal>").expect("total");
    let first = xml[total..].find("<cbc:").expect("child") + total;
    assert!(
        xml[first..].starts_with("<cbc:LineExtensionAmount "),
        "{xml}"
    );

    let mut tampered = inv;
    tampered.monetary_total.line_extension = Money::from_cents(1);
    let found = rules(&tampered);
    assert!(found.contains(&"BR-CO-10"), "{found:?}");
    assert!(found.contains(&"BR-CO-13"), "{found:?}");
}

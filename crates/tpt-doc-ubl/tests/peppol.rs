//! Integration tests: parse the PEPPOL BIS fixture, validate it, and prove
//! serialize → parse round-trips.

use tpt_doc_core::Limits;
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
    assert_eq!(invoice.monetary_total.payable, Money::from_cents(48300));
    assert_eq!(
        invoice.monetary_total.tax_exclusive,
        Money::from_cents(42000)
    );
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
    invoice.monetary_total.payable = Money::from_cents(999_900);
    let violations = validate(&invoice);
    assert!(
        violations.iter().any(|v| v.rule == "BR-CO-16"),
        "expected BR-CO-16, got {violations:?}"
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
    invoice.push_line(InvoiceLine::new(1, "item", 1.into(), 10.into()).unwrap());
    let violations = validate(&invoice);
    let rules: Vec<_> = violations.iter().map(|v| v.rule).collect();
    assert!(rules.contains(&"BR-01"), "{rules:?}");
    assert!(rules.contains(&"PEPPOL-EN16931-R001"), "{rules:?}");
    for rule in [
        "PEPPOL-EN16931-R003",
        "PEPPOL-EN16931-R020",
        "PEPPOL-EN16931-R010",
        "BR-08",
        "BR-10",
        "BR-CO-26",
        "BR-CO-04",
    ] {
        assert!(rules.contains(&rule), "{rule} missing from {rules:?}");
    }
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
            quantity_milli in 1i64..1_000_000_000i64,
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
            // Exact decimals: quantity to 3 places, price to cents.
            let quantity = Decimal::new(quantity_milli, 3).unwrap();
            let unit_price = Decimal::new(i64::try_from(unit_price_cents).unwrap(), 2).unwrap();
            invoice.push_line(InvoiceLine::new(1, item, quantity, unit_price).unwrap());
            let rate = Decimal::from(15);
            invoice.push_tax_subtotal(TaxSubtotal {
                category: TaxCategory::Standard,
                rate_percent: 15.into(),
                taxable_amount: invoice.monetary_total.tax_exclusive,
                tax_amount: rate.percent_of(invoice.monetary_total.tax_exclusive).unwrap(),
                exemption_reason: None,
            });

            let bytes = invoice.to_xml().expect("ok");
            let restored = Invoice::from_xml(&bytes).expect("ok");
            prop_assert_eq!(restored, invoice);
        }
    }
}

/// A UBL invoice is only a few elements deep, so nesting past
/// `max_xml_depth` is refused rather than walked.
#[test]
fn deeply_nested_xml_is_rejected() {
    let mut xml = String::from("<Invoice>");
    for _ in 0..64 {
        xml.push_str("<a>");
    }
    for _ in 0..64 {
        xml.push_str("</a>");
    }
    xml.push_str("</Invoice>");

    let limits = Limits {
        max_xml_depth: 8,
        ..Limits::default()
    };
    let err = Invoice::from_xml_with_limits(xml.as_bytes(), limits)
        .expect_err("deep nesting must be rejected");
    assert!(err.to_string().contains("8 level limit"), "{err}");
}

/// The default limit leaves a real PEPPOL invoice untouched.
#[test]
fn default_limits_allow_the_fixture() {
    let invoice =
        Invoice::from_xml_with_limits(&fixture_bytes(), Limits::default()).expect("fixture parses");
    assert_eq!(invoice.currency, "NZD");
}

/// Wrap `body` in a minimal valid UBL invoice envelope.
fn invoice_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID>
  <cbc:ID>INV-1</cbc:ID>
  <cbc:IssueDate>2026-01-15</cbc:IssueDate>
  <cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>
  <cbc:DocumentCurrencyCode>NZD</cbc:DocumentCurrencyCode>
  <cac:AccountingSupplierParty><cac:Party><cac:PartyName><cbc:Name>Supplier Ltd</cbc:Name></cac:PartyName></cac:Party></cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty><cac:Party><cac:PartyName><cbc:Name>Customer Ltd</cbc:Name></cac:PartyName></cac:Party></cac:AccountingCustomerParty>
{body}
  <cac:LegalMonetaryTotal>
    <cbc:TaxExclusiveAmount currencyID="NZD">100.00</cbc:TaxExclusiveAmount>
    <cbc:PayableAmount currencyID="NZD">115.00</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>
</Invoice>"#
    )
}

/// A realistic `TaxSubtotal` carries `TaxScheme/ID` ("VAT") as well as
/// `TaxCategory/ID` ("S"). Any `ID` inside the subtotal used to overwrite the
/// tax category, so the *later* `TaxScheme` ID ("VAT") wiped the "S" set a
/// moment earlier — silently discarding the category and dropping the subtotal.
#[test]
fn tax_scheme_id_does_not_overwrite_the_tax_category() {
    let xml = invoice_xml(
        r#"  <cac:TaxTotal>
    <cbc:TaxAmount currencyID="NZD">15.00</cbc:TaxAmount>
    <cac:TaxSubtotal>
      <cbc:TaxableAmount currencyID="NZD">100.00</cbc:TaxableAmount>
      <cbc:TaxAmount currencyID="NZD">15.00</cbc:TaxAmount>
      <cac:TaxCategory>
        <cbc:ID>S</cbc:ID>
        <cbc:Percent>15.0</cbc:Percent>
        <cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>
      </cac:TaxCategory>
    </cac:TaxSubtotal>
  </cac:TaxTotal>"#,
    );

    let invoice = Invoice::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(
        invoice.tax_subtotals.len(),
        1,
        "the subtotal must survive TaxScheme/ID"
    );
    assert_eq!(invoice.tax_subtotals[0].category.code(), "S");
    assert_eq!(invoice.tax_subtotals[0].rate_percent, Decimal::from(15));
    assert_eq!(
        invoice.tax_subtotals[0].taxable_amount,
        Money::from_cents(10000)
    );
}

/// `TaxScheme/ID` is not a tax category, so a category code must come from
/// `TaxCategory/ID` specifically.
#[test]
fn an_unknown_tax_category_code_is_reported_not_dropped() {
    let xml = invoice_xml(
        r#"  <cac:TaxTotal>
    <cac:TaxSubtotal>
      <cbc:TaxableAmount currencyID="NZD">100.00</cbc:TaxableAmount>
      <cbc:TaxAmount currencyID="NZD">15.00</cbc:TaxAmount>
      <cac:TaxCategory><cbc:ID>QQ</cbc:ID><cbc:Percent>15.0</cbc:Percent></cac:TaxCategory>
    </cac:TaxSubtotal>
  </cac:TaxTotal>"#,
    );
    let err = Invoice::from_xml(xml.as_bytes()).expect_err("unknown category must fail");
    assert!(err.to_string().contains("QQ"), "{err}");
}

/// Every PEPPOL category code must parse, not just S/Z/E/AE.
#[test]
fn all_peppol_tax_categories_parse() {
    for (code, expected) in [
        ("S", "S"),
        ("Z", "Z"),
        ("E", "E"),
        ("AE", "AE"),
        ("K", "K"),
        ("G", "G"),
        ("O", "O"),
        ("L", "L"),
        ("M", "M"),
    ] {
        let xml = invoice_xml(&format!(
            r#"  <cac:TaxTotal>
    <cac:TaxSubtotal>
      <cbc:TaxableAmount currencyID="NZD">100.00</cbc:TaxableAmount>
      <cbc:TaxAmount currencyID="NZD">15.00</cbc:TaxAmount>
      <cac:TaxCategory><cbc:ID>{code}</cbc:ID><cbc:Percent>15.0</cbc:Percent></cac:TaxCategory>
    </cac:TaxSubtotal>
  </cac:TaxTotal>"#
        ));
        let invoice = Invoice::from_xml(xml.as_bytes()).expect("parse");
        assert_eq!(
            invoice.tax_subtotals[0].category.code(),
            expected,
            "category {code}"
        );
    }
}

/// A malformed amount silently became `0.0`, which then failed a validation
/// rule with a message pointing at the wrong thing.
#[test]
fn a_malformed_amount_is_reported() {
    let xml = invoice_xml(
        r#"  <cac:TaxTotal>
    <cac:TaxSubtotal>
      <cbc:TaxableAmount currencyID="NZD">not-a-number</cbc:TaxableAmount>
      <cbc:TaxAmount currencyID="NZD">15.00</cbc:TaxAmount>
      <cac:TaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>15.0</cbc:Percent></cac:TaxCategory>
    </cac:TaxSubtotal>
  </cac:TaxTotal>"#,
    );
    let err = Invoice::from_xml(xml.as_bytes()).expect_err("bad amount must fail");
    assert!(err.to_string().contains("TaxableAmount"), "{err}");
    assert!(err.to_string().contains("not-a-number"), "{err}");
}

/// Only `AccountingSupplierParty`/`AccountingCustomerParty` carry the parties
/// we model. Any other `Party` — `DeliveryParty`, `PayeeParty`,
/// `TaxRepresentativeParty` — was matching the same prefix test and overwriting
/// the supplier's name.
#[test]
fn other_party_elements_do_not_overwrite_the_supplier() {
    let xml = invoice_xml(
        r#"  <cac:DeliveryParty><cac:Party><cac:PartyName><cbc:Name>Wellington Warehouse</cbc:Name></cac:PartyName></cac:Party></cac:DeliveryParty>
  <cac:TaxRepresentativeParty><cac:Party><cac:PartyName><cbc:Name>Tax Rep Ltd</cbc:Name></cac:PartyName></cac:Party></cac:TaxRepresentativeParty>
  <cac:TaxTotal>
    <cbc:TaxAmount currencyID="NZD">15.00</cbc:TaxAmount>
    <cac:TaxSubtotal>
      <cbc:TaxableAmount currencyID="NZD">100.00</cbc:TaxableAmount>
      <cbc:TaxAmount currencyID="NZD">15.00</cbc:TaxAmount>
      <cac:TaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>15.0</cbc:Percent></cac:TaxCategory>
    </cac:TaxSubtotal>
  </cac:TaxTotal>"#,
    );

    let invoice = Invoice::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(invoice.supplier.name, "Supplier Ltd");
    assert_eq!(invoice.customer.name, "Customer Ltd");
}

/// Only the two `Accounting*Party` wrappers may set the supplier/customer slot.
/// A bare `<cac:Party>` elsewhere must not redirect writes into slot 0 or 1.
#[test]
fn a_party_outside_the_accounting_wrappers_is_ignored() {
    let xml = invoice_xml(
        r#"  <cac:AdditionalDocumentReference>
    <cac:DocumentReference><cbc:ID>REF-9</cbc:ID></cac:DocumentReference>
  </cac:AdditionalDocumentReference>"#,
    );
    let invoice = Invoice::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(
        invoice.id, "INV-1",
        "document ID must not overwrite the invoice ID"
    );
    assert_eq!(invoice.supplier.name, "Supplier Ltd");
}

/// Format-specific violations convert into the shared report without the
/// `Vec<UblValidationError>` API disappearing, and the PEPPOL rule ID survives
/// the conversion as a machine-readable `rule_id`.
#[test]
fn ubl_violations_convert_to_a_validation_report() {
    use tpt_doc_core::{Location, Severity};
    use tpt_doc_ubl::validate::UblValidationError;

    let mut invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    // Break BR-CO-10 so there is a known violation to convert.
    invoice.monetary_total.tax_exclusive = Money::from_cents(100);
    let violations = validate(&invoice);
    assert!(!violations.is_empty());

    let report = UblValidationError::to_report(&violations);
    assert!(report.has_errors(), "a PEPPOL rule violation is an error");
    let issue = report.errors().next().expect("an error");
    assert_eq!(issue.severity, Severity::Error);
    assert!(
        !issue.rule_id.is_empty(),
        "the PEPPOL rule ID must survive: {issue:?}"
    );
    assert!(issue.rule_id.starts_with("BR-") || issue.rule_id.starts_with("PEPPOL-"));
    assert!(
        matches!(issue.location, Location::Path(_)),
        "a UBL finding points at an element path, got {:?}",
        issue.location
    );
}

/// The empty case must produce an empty report, not a report with one phantom
/// issue.
#[test]
fn a_clean_invoice_converts_to_an_empty_report() {
    use tpt_doc_ubl::validate::UblValidationError;

    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    assert!(validate(&invoice).is_empty());
    let report = UblValidationError::to_report(&validate(&invoice));
    assert!(report.is_valid());
    assert!(report.issues().is_empty());
}

/// A malformed UBL document must say *where* it failed. "unknown entity
/// reference" alone leaves the reader re-scanning the whole document.
#[test]
fn a_malformed_invoice_reports_the_byte_offset() {
    let bytes = invoice_xml("  <cac:TaxTotal><cbc:TaxAmount>&nope;</cbc:TaxAmount></cac:TaxTotal>");
    let err = Invoice::from_xml(bytes.as_bytes()).expect_err("bad entity must fail");
    let text = err.to_string();
    assert!(
        text.contains("entity reference"),
        "the cause must survive: {text}"
    );
    assert!(
        err.byte_offset().is_some_and(|offset| offset > 0),
        "the error must carry a byte offset, got: {text}"
    );
}

#[cfg(feature = "facturx")]
#[test]
fn invoice_xml_attaches_to_pdf_as_facturx() {
    use tpt_doc_pdf::{Document, Font, Page};

    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text("Invoice INV-PEPPOL-001", font, 12.0, (72.0, 700.0));
    doc.add_page(page);

    tpt_doc_ubl::invoice::facturx::attach_xml(&invoice, &mut doc).expect("attach");
    let bytes = doc.write().expect("write");

    let text = String::from_utf8_lossy(&bytes).into_owned();
    assert!(text.contains("(factur-x.xml)"), "file name missing");
    assert!(text.contains("/AFRelationship /Data"));
    // The invoice XML payload is embedded verbatim.
    let xml = invoice.to_xml().expect("xml");
    assert!(
        bytes.windows(32).any(|w| w == &xml[..32]),
        "embedded payload mismatch"
    );
}

/// Three pens at 0.10 is exactly 0.30; binary floats give 0.30000000000000004,
/// which would make BR-CO-10 compare unequal after a round trip.
#[test]
fn exact_arithmetic_survives_a_round_trip_and_validates() {
    let date = time::Date::from_calendar_date(2026, time::Month::January, 15).expect("date");
    let mut party = Party::new("A");
    party.endpoint_id = Some("9429041234567".to_owned());
    party.endpoint_scheme = Some("0088".to_owned());
    party.address = Some(Address {
        country_code: Some("NZ".to_owned()),
        ..Address::default()
    });
    let mut invoice = Invoice::new(
        "INV-X",
        InvoiceTypeCode::Invoice,
        date,
        party.clone(),
        party,
    );
    invoice.buyer_reference = Some("PO-1".to_owned());
    let price: Decimal = "0.10".parse().expect("price");
    invoice.push_line(
        InvoiceLine::new(1, "Pens", 3.into(), price)
            .expect("line")
            .with_tax(TaxCategory::Standard, 15.into()),
    );
    assert_eq!(
        invoice.lines[0].line_extension_amount,
        Money::from_cents(30)
    );
    invoice.push_tax_subtotal(TaxSubtotal {
        category: TaxCategory::Standard,
        rate_percent: 15.into(),
        taxable_amount: Money::from_cents(30),
        tax_amount: Decimal::from(15)
            .percent_of(Money::from_cents(30))
            .expect("tax"),
        exemption_reason: None,
    });
    assert!(validate(&invoice).is_empty(), "{:?}", validate(&invoice));
    let restored = Invoice::from_xml(&invoice.to_xml().expect("xml")).expect("parse");
    assert_eq!(restored, invoice);
}

#[test]
fn unit_prices_keep_more_than_two_decimals() {
    let date = time::Date::from_calendar_date(2026, time::Month::January, 15).expect("date");
    let mut invoice = Invoice::new(
        "INV-P",
        InvoiceTypeCode::Invoice,
        date,
        Party::new("A"),
        Party::new("B"),
    );
    let price: Decimal = "0.0125".parse().expect("price");
    invoice.push_line(InvoiceLine::new(1, "Bolts", 1000.into(), price).expect("line"));
    assert_eq!(
        invoice.lines[0].line_extension_amount,
        Money::from_cents(1250)
    );
    let xml = String::from_utf8(invoice.to_xml().expect("xml")).expect("utf8");
    assert!(xml.contains(">0.0125</cbc:PriceAmount>"), "{xml}");
    let restored = Invoice::from_xml(xml.as_bytes()).expect("parse");
    assert_eq!(restored.lines[0].unit_price, price);
}

#[test]
fn amounts_that_are_not_exact_cents_or_not_numbers_are_rejected() {
    let xml = String::from_utf8(fixture_bytes()).expect("utf8");
    for bad in ["483.005", "NaN", "inf", "1e3", "4,83"] {
        let tampered = xml.replace(
            "<cbc:PayableAmount currencyID=\"NZD\">483.00<",
            &format!("<cbc:PayableAmount currencyID=\"NZD\">{bad}<"),
        );
        assert_ne!(tampered, xml);
        assert!(Invoice::from_xml(tampered.as_bytes()).is_err(), "{bad}");
    }
}

#[test]
fn fixture_exposes_the_bis_billing_elements() {
    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    assert_eq!(invoice.buyer_reference.as_deref(), Some("PO-2026-0042"));
    assert_eq!(
        invoice.due_date,
        Some(time::Date::from_calendar_date(2026, time::Month::February, 14).expect("date"))
    );
    let supplier = &invoice.supplier;
    assert_eq!(supplier.endpoint_scheme.as_deref(), Some("0088"));
    assert_eq!(supplier.endpoint_id.as_deref(), Some("9429041234567"));
    assert_eq!(supplier.tax_id.as_deref(), Some("GST-111"));
    assert_eq!(supplier.legal_id.as_deref(), Some("9429041234567"));
    assert_eq!(
        supplier.address.as_ref().and_then(|a| a.street.as_deref()),
        Some("1 Lambton Quay")
    );
    // The buyer has no VAT registration: PartyTaxScheme absent, legal entity present.
    assert_eq!(invoice.customer.tax_id, None);

    let means = invoice.payment_means.as_ref().expect("payment means");
    assert_eq!(means.code, "30");
    assert_eq!(means.account_id.as_deref(), Some("12-3456-7890123-00"));

    assert_eq!(invoice.lines[0].unit_code, "HUR");
    assert_eq!(invoice.lines[1].unit_code, "MON");
    // The line's own ID must survive the nested ClassifiedTaxCategory/ID.
    assert_eq!(invoice.lines[0].id, 1);
    assert_eq!(invoice.lines[1].id, 2);
    let tax = invoice.lines[0].tax.expect("line tax");
    assert_eq!(tax.category, TaxCategory::Standard);
    assert_eq!(tax.rate_percent, Decimal::from(15));
}

/// UBL is an XSD `sequence`: receivers validate element order, not just names.
#[test]
fn serialized_elements_follow_the_ubl_schema_order() {
    let invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    let xml = String::from_utf8(invoice.to_xml().expect("xml")).expect("utf8");
    let order = [
        "<cbc:CustomizationID>",
        "<cbc:ProfileID>",
        "<cbc:ID>",
        "<cbc:IssueDate>",
        "<cbc:DueDate>",
        "<cbc:InvoiceTypeCode>",
        "<cbc:DocumentCurrencyCode>",
        "<cbc:BuyerReference>",
        "<cac:AccountingSupplierParty>",
        "<cac:AccountingCustomerParty>",
        "<cac:PaymentMeans>",
        "<cac:TaxTotal>",
        "<cac:LegalMonetaryTotal>",
        "<cac:InvoiceLine>",
    ];
    let mut last = 0;
    for tag in order {
        let at = xml.find(tag).unwrap_or_else(|| panic!("{tag} missing"));
        assert!(at >= last, "{tag} is out of order");
        last = at;
    }
    // Inside a party: EndpointID, PartyName, PostalAddress, PartyTaxScheme, PartyLegalEntity.
    let party_order = [
        "<cbc:EndpointID schemeID=\"0088\">",
        "<cac:PartyName>",
        "<cac:PostalAddress>",
        "<cac:PartyTaxScheme>",
        "<cac:PartyLegalEntity>",
    ];
    let mut last = 0;
    for tag in party_order {
        let at = xml.find(tag).unwrap_or_else(|| panic!("{tag} missing"));
        assert!(at >= last, "{tag} is out of order");
        last = at;
    }
    // Every tax category names its scheme.
    assert!(xml.contains("<cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme>"));
    assert!(xml.contains("<cbc:InvoicedQuantity unitCode=\"HUR\">2</cbc:InvoicedQuantity>"));
    // And it all survives a re-parse.
    assert_eq!(
        Invoice::from_xml(xml.as_bytes()).expect("re-parse"),
        invoice
    );
}

#[test]
fn a_credit_note_has_no_document_due_date() {
    let mut invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    invoice.type_code = InvoiceTypeCode::CreditNote;
    let xml = String::from_utf8(invoice.to_xml().expect("xml")).expect("utf8");
    assert!(!xml.contains("<cbc:DueDate>"), "{xml}");
    assert!(xml.contains("<cac:PaymentMeans>"));
}

#[test]
fn incomplete_bis_data_is_reported_per_rule() {
    let mut invoice = Invoice::from_xml(&fixture_bytes()).expect("parse fixture");
    invoice.buyer_reference = None;
    invoice.customer.endpoint_scheme = None;
    invoice.supplier.address = None;
    invoice
        .customer
        .address
        .as_mut()
        .expect("address")
        .country_code = None;
    invoice.lines[0].unit_code.clear();
    invoice.lines[1].tax = None;
    invoice.payment_means.as_mut().expect("means").code.clear();
    invoice.id.clear();
    let rules: Vec<_> = validate(&invoice).iter().map(|v| v.rule).collect();
    for expected in [
        "PEPPOL-EN16931-R003",
        "BR-63",
        "BR-08",
        "BR-11",
        "BR-23",
        "BR-CO-04",
        "BR-49",
        "BR-02",
    ] {
        assert!(
            rules.contains(&expected),
            "{expected} missing from {rules:?}"
        );
    }
}

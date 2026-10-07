# tpt-doc-ubl

UBL 2.1/2.3 and PEPPOL BIS Billing 3.0 e-invoicing — parse and generate compliant XML invoices.

## Design

Invoices are typed structs covering the PEPPOL BIS Billing 3.0 mandatory elements (`ID`, `IssueDate`, supplier, customer, invoice lines, tax subtotals, `LegalMonetaryTotal`). Document totals are recomputed automatically as lines and tax subtotals are added, and a built-in validator checks the EN 16931 arithmetic rules (BR-01, BR-02, BR-05, BR-06..BR-11, BR-16, BR-23, BR-49, BR-62, BR-63, BR-CO-04, BR-CO-10, BR-CO-13, BR-CO-15, BR-CO-16, BR-CO-17, BR-CO-18, BR-CO-26, the VAT breakdown families BR-S/Z/E/AE/IC/G/O/AF/AG-01/05/08/09/10, PEPPOL-EN16931-R001, R003, R010, R020) without external schematron files.

## Usage

```rust
use tpt_doc_ubl::prelude::*;

let mut invoice = Invoice::new(
    "INV-001",
    InvoiceTypeCode::Invoice,
    issue_date,
    Party::new("Supplier Ltd"),
    Party::new("Customer Ltd"),
);
invoice.push_line(InvoiceLine::new(1, "Consulting", 2.into(), 150.into())?);
invoice.push_tax_subtotal(TaxSubtotal {
    category: TaxCategory::Standard,
    rate_percent: 15.into(),
    taxable_amount: Money::from_cents(30000),
    tax_amount: Money::from_cents(4500),
});

for violation in tpt_doc_ubl::validate(&invoice) {
    eprintln!("{violation}");
}
```

## Limits

`Invoice::from_xml_with_limits` takes explicit `tpt_doc_core::Limits`: nesting is
refused past `max_xml_depth`. A UBL invoice is only a few levels deep, so
anything deeper is corrupt or hostile.

## Element scoping

The XML parser matches every element by its **structural position**, not by
element name. UBL reuses names across unrelated parts of the document, so
name-only matching silently writes one part's value over another's:

| Element | Matched against | Why it matters |
|---|---|---|
| `cac:TaxCategory/cbc:ID` | the tax category (`S`, `Z`, `K`, …) | `cac:TaxScheme/cbc:ID` is also `cbc:ID`, but holds `"VAT"` |
| `cac:Accounting*Party/cac:Party` | the supplier or customer | `DeliveryParty`, `PayeeParty`, `TaxRepresentativeParty` are different things |
| `cbc:ID` as a direct child of the root | the invoice ID | nested aggregates reuse `cbc:ID` for their own purposes |

An unparsable amount is an error, not `0` — defaulting turned a corrupt
document into one that parsed cleanly and then failed an arithmetic rule,
pointing at the wrong field.

## Exact money

Amounts are [`Money`] (an `i64` count of cents) and quantities, unit prices and
tax rates are [`Decimal`] (an exact base-10 number). Binary floats cannot
represent `0.10`, so `3 x 0.10` was `0.30000000000000004` and the EN 16931
rules, which compare for equality, could disagree with the sender by a cent.
Rounding is half away from zero. `Decimal::percent_of` computes tax. Amounts
with more than two decimals, `NaN`, `inf` and exponents are rejected on
parsing; unit prices may carry more decimals. Currencies whose minor unit is
not 1/100 (JPY, KWD) are not modelled yet.

## Tax categories

`TaxCategory` covers the UN/EDIFACT 5305 codes PEPPOL BIS Billing 3.0 uses:
`S` (standard), `Z` (zero-rated), `E` (exempt), `AE` (reverse charge), `K`
(EEA supply), `G` (free export), `O` (outside scope), `L` (Canary Islands),
and `M` (Ceuta and Melilla).

## Credit notes and type codes

A `CreditNote` document is serialized with the credit-note element names, which
is what UBL requires and what PEPPOL receivers check:

| Element | Invoice | Credit note |
|---|---|---|
| Root | `Invoice` | `CreditNote` |
| Type code | `cbc:InvoiceTypeCode` | `cbc:CreditNoteTypeCode` |
| Line | `cac:InvoiceLine` | `cac:CreditNoteLine` |
| Quantity | `cbc:InvoicedQuantity` | `cbc:CreditedQuantity` |

`InvoiceTypeCode` covers `380` (commercial invoice) and `381` (credit note) as
named variants, and any other UN/EDIFACT 1001 code through
`InvoiceTypeCode::Other(u16)` — `384` (self-billed credit note), `386`
(prepayment invoice), `389` (self-billed debit note), `261`/`262`, `393`, and
`80` all round-trip. Codes outside `1..=9999` are rejected.

A `CreditNote` **root** normalises the type code to `CreditNote`, because the
root rather than the number determines the line element names. Preserving the
sub-code would mean re-serializing a credit note as an invoice.

## Standards

| Standard | Coverage |
|---|---|
| UBL 2.1/2.3 | `Invoice` and `CreditNote` document models |
| EN 16931 | Core model fields and arithmetic rules |
| PEPPOL BIS Billing 3.0 | Validation rules, endpoint identifiers |
| Factur-X / ZUGFeRD | Planned behind the `facturx` feature flag |

## BIS Billing 3.0 elements

Besides the arithmetic, a PEPPOL receiver rejects an invoice that lacks the
descriptive elements, so the model carries (and the writer emits, in UBL schema
order) all of them:

| Model | XML |
|---|---|
| `Invoice::buyer_reference` | `cbc:BuyerReference` (PEPPOL-EN16931-R003) |
| `Invoice::due_date` | `cbc:DueDate` (invoices only) |
| `Invoice::payment_means` | `cac:PaymentMeans` (code, `PaymentID`, payee account) |
| `Party::endpoint_scheme` | `cbc:EndpointID/@schemeID` |
| `Party::legal_name` / `legal_id` | `cac:PartyLegalEntity` (`RegistrationName` defaults to the trading name) |
| `Address::street` | `cbc:StreetName` |
| `InvoiceLine::unit_code` | `cbc:InvoicedQuantity/@unitCode` (UN/ECE Rec. 20, default `C62`) |
| `InvoiceLine::tax` | `cac:Item/cac:ClassifiedTaxCategory` — `with_tax(category, rate)` |
| every tax category | `cac:TaxScheme/cbc:ID` = `VAT` |

`validate` reports each missing piece under its published rule id, so a
failing invoice says *which* receiver rule it would trip.

## VAT breakdown rules

Each tax category has a rule family; the prefix is
`TaxCategory::rule_prefix()` (`S`→`BR-S`, `Z`→`BR-Z`, `E`→`BR-E`, `AE`→`BR-AE`,
`K`→`BR-IC`, `G`→`BR-G`, `O`→`BR-O`, `L`→`BR-AF`, `M`→`BR-AG`):

| Suffix | Rule |
|---|---|
| `-01` | a line in the category requires a breakdown in that category |
| `-05` | line rate: `> 0` for `S`, `>= 0` for `L`/`M`, `= 0` for the rest |
| `-08` | breakdown taxable amount = Σ net amounts of its lines (per rate for `S`/`L`/`M`) |
| `-09` | zero-rate categories carry a zero rate and zero tax |
| `-10` | `E`, `AE`, `K`, `G`, `O` state an exemption reason (`TaxSubtotal::with_exemption_reason`) |

`BR-CO-18` requires at least one breakdown, and `BR-CO-17` checks
`tax = taxable × rate` for every breakdown. `LegalMonetaryTotal` now carries
`LineExtensionAmount` (BT-106) so BR-CO-10 and BR-CO-13 are distinct checks.

Not modelled yet (so not checked): party VAT identifiers for `AE`/`K`
(`BR-AE-02..04`, `BR-IC-02..04`), delivery date/country for `K`
(`BR-IC-11/12`), the exemption reason *code*, and document-level allowances
and charges.

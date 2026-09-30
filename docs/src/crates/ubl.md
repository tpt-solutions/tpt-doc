# tpt-doc-ubl

UBL 2.1/2.3 and PEPPOL BIS Billing 3.0 e-invoicing — parse and generate compliant XML invoices.

## Design

Invoices are typed structs covering the PEPPOL BIS Billing 3.0 mandatory elements (`ID`, `IssueDate`, supplier, customer, invoice lines, tax subtotals, `LegalMonetaryTotal`). Document totals are recomputed automatically as lines and tax subtotals are added, and a built-in validator checks the EN 16931 arithmetic rules (BR-16, BR-CO-10, BR-CO-13, BR-CO-15, BR-CO-25) without external schematron files.

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
invoice.push_line(InvoiceLine::new(1, "Consulting", 2.0, 150.0));
invoice.push_tax_subtotal(TaxSubtotal {
    category: TaxCategory::Standard,
    rate_percent: 15.0,
    taxable_amount: 300.0,
    tax_amount: 45.0,
});

for violation in tpt_doc_ubl::validate(&invoice) {
    eprintln!("{violation}");
}
```

## Standards

| Standard | Coverage |
|---|---|
| UBL 2.1/2.3 | `Invoice` and `CreditNote` document models |
| EN 16931 | Core model fields and arithmetic rules |
| PEPPOL BIS Billing 3.0 | Validation rules, endpoint identifiers |
| Factur-X / ZUGFeRD | Planned behind the `facturx` feature flag |

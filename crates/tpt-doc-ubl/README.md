# tpt-doc-ubl

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-ubl.svg)](https://crates.io/crates/tpt-doc-ubl)
[![Docs.rs](https://docs.rs/tpt-doc-ubl/badge.svg)](https://docs.rs/tpt-doc-ubl)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

UBL 2.1 / 2.3 and PEPPOL BIS Billing 3.0 e-invoicing — parse and generate
compliant XML invoices.

Invoices are modelled as typed structs covering the mandatory PEPPOL BIS
Billing 3.0 elements, with built-in arithmetic validation against the
**EN 16931** business rules.

## Highlights

- Full typed invoice model: parties, addresses, lines, tax subtotals,
  monetary totals.
- PEPPOL BIS Billing 3.0 syntax rules and EN 16931 business rules validated
  automatically, returning `Vec<UblValidationError>` with rule references.
- XML round-tripping (`to_xml` / `from_xml`).
- Optional **Factur-X / ZUGFeRD** hybrid invoices: embed the UBL XML inside a
  PDF as an associated file.
- Tax category handling for `S`, `Z`, `E`, `AE`, `K`, `G`, `O`, and `L`.

## Installation

```toml
[dependencies]
tpt-doc-ubl = "0.1"
```

Enable Factur-X hybrid invoicing:

```toml
[dependencies]
tpt-doc-ubl = { version = "0.1", features = ["facturx"] }
```

## Usage

### Create an invoice

```rust
use tpt_doc_ubl::prelude::*;
use time::{Date, Month};

let date = Date::from_calendar_date(2026, Month::January, 15).unwrap();
let mut invoice = Invoice::new(
    "INV-001",
    InvoiceTypeCode::Invoice,
    date,
    Party::new("Supplier Ltd"),
    Party::new("Customer Ltd"),
);

invoice.push_line(InvoiceLine::new(1, "Consulting", 2.0, 150.0));
assert_eq!(invoice.total(), 300.0);

let xml = invoice.to_xml()?;
assert!(xml.windows(18).any(|w| w == b"urn:cen.eu:en16931"));
# Ok::<(), tpt_doc_core::DocError>(())
```

### Validate against PEPPOL / EN 16931

```rust
use tpt_doc_ubl::prelude::*;
use time::{Date, Month};

let date = Date::from_calendar_date(2026, Month::January, 15).unwrap();
let mut invoice = Invoice::new(
    "INV-002",
    InvoiceTypeCode::Invoice,
    date,
    Party::new("Supplier Ltd"),
    Party::new("Customer Ltd"),
);
invoice.push_line(InvoiceLine::new(1, "Widget", 1.0, 100.0));

let errors = validate(&invoice);
for error in &errors {
    println!("{} at {}: {}", error.rule, error.field_path, error.message);
}
```

### Tax subtotals and totals

```rust
use tpt_doc_ubl::prelude::*;
use time::{Date, Month};

let date = Date::from_calendar_date(2026, Month::January, 15).unwrap();
let mut invoice = Invoice::new(
    "INV-003",
    InvoiceTypeCode::Invoice,
    date,
    Party::new("Supplier Ltd"),
    Party::new("Customer Ltd"),
);
invoice.push_line(InvoiceLine::new(1, "Widget", 2.0, 120.0));
invoice.push_tax_subtotal(TaxSubtotal {
    category: TaxCategory::Standard,
    rate_percent: 15.0,
    taxable_amount: 240.0,
    tax_amount: 36.0,
});
// `total()` is the tax-inclusive payable amount: 240.00 net + 36.00 GST.
assert_eq!(invoice.total(), 276.0);
```

### Factur-X / ZUGFeRD hybrid invoice

```rust
use tpt_doc_ubl::invoice::facturx;
use tpt_doc_ubl::{Invoice, InvoiceTypeCode, Party};
use time::{Date, Month};

let date = Date::from_calendar_date(2026, Month::January, 15).unwrap();
let invoice = Invoice::new(
    "INV-004",
    InvoiceTypeCode::Invoice,
    date,
    Party::new("Supplier Ltd"),
    Party::new("Customer Ltd"),
);

let mut pdf = tpt_doc_pdf::Document::new();
facturx::attach_xml(&invoice, &mut pdf)?;
let bytes = pdf.write()?;
# Ok::<(), tpt_doc_core::DocError>(())
```

## Modules

| Module | Contents |
|---|---|
| `invoice` | `Invoice`, `InvoiceTypeCode`, `facturx` submodule (`attach_xml`). |
| `types` | `Party`, `Address`, `InvoiceLine`, `TaxCategory`, `TaxSubtotal`, `MonetaryTotal`, `IssueDate`. |
| `validate` | `validate`, `UblValidationError` — PEPPOL BIS Billing 3.0 rules. |

## Feature flags

| Feature | Default | Description |
|---|---|---|
| `facturx` | no | Enables Factur-X / ZUGFeRD hybrid invoices, pulling in `tpt-doc-pdf`. |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-ubl>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

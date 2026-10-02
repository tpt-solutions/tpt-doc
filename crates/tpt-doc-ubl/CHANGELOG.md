# Changelog

All notable changes to `tpt-doc-ubl` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-10

### Added

- `types` module: `Party`, `Address`, `InvoiceLine`, `TaxCategory`
  (`S`, `Z`, `E`, `AE`, `K`, `G`, `O`, `L`), `TaxSubtotal`, `MonetaryTotal`,
  and the `IssueDate` alias.
- `invoice` module: `Invoice` with `new()`, `push_line()`,
  `push_tax_subtotal()`, `total()`, and XML round-tripping via `to_xml()` /
  `from_xml()`; `InvoiceTypeCode` with UN/CEFACT 1001 code values.
- `validate` module implementing PEPPOL BIS Billing 3.0 syntax and EN 16931
  business rules, returning `Vec<UblValidationError>` with rule references.
- Optional `facturx` feature exposing `invoice::facturx::attach_xml()` to embed
  UBL invoice XML into a PDF as an associated file (Factur-X / ZUGFeRD),
  depending on `tpt-doc-pdf`.
- `prelude` module for single-line imports.
- Integration tests covering round-tripping, arithmetic totals, and validation
  failures, plus `proptest` property tests.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-ubl-v0.1.0

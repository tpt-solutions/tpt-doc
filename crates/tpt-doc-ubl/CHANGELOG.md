# Changelog

All notable changes to `tpt-doc-ubl` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- VAT breakdown rules: BR-CO-18 and the BR-S/Z/E/AE/IC/G/O/AF/AG families (`-01`, `-05`, `-08`, `-09`, `-10`). `TaxSubtotal::new` / `with_exemption_reason`, `TaxCategory::{is_rate_based, requires_exemption_reason, rule_prefix}`, `cbc:TaxExemptionReason`, and `LegalMonetaryTotal/LineExtensionAmount`.

### Fixed

- Rule ids were mislabelled: the net-total check was BR-CO-10 (now BR-CO-13), the gross total BR-CO-13 (now BR-CO-15), payable BR-CO-15 (now BR-CO-16) and the tax formula BR-CO-25 (now BR-CO-17). BR-CO-10 now compares the line sum with the declared `LineExtensionAmount`.


- BIS Billing 3.0 descriptive elements: `buyer_reference`, `due_date`, `payment_means`, endpoint `schemeID`, `PartyLegalEntity`, `StreetName`, per-line `unitCode` and `ClassifiedTaxCategory`, and `TaxScheme` on every tax category. Elements are written in UBL schema order.
- Validation rules BR-02, BR-05, BR-06..BR-11, BR-23, BR-49, BR-62, BR-63, BR-CO-04, BR-CO-26, PEPPOL-EN16931-R003/R010/R020.

### Changed

- **Breaking:** `Party`, `InvoiceLine` and `Invoice` gained fields (struct literals need updating); the endpoint rule is now PEPPOL-EN16931-R020/R010 rather than the mislabelled R062.

- **Breaking:** invoice amounts are exact. `Money` (cents) replaces `f64` for line, tax and total amounts; `Decimal` replaces `f64` for quantity, unit price and rate. `InvoiceLine::new` takes `Decimal`s and returns `Result`. Parsing rejects amounts with more than two decimals, `NaN`, `inf` and exponents. BR-CO-10/13/15/25 now compare exactly.

### Added

- `UblValidationError::to_report` and `From<&UblValidationError> for
  ValidationIssue`, converting the format-specific violations into the shared
  `tpt_doc_core::ValidationReport`. The PEPPOL rule ID survives as `rule_id` and
  the `cbc:`/`cac:` element path as a `Location::Path`. The existing
  `Vec<UblValidationError>` API is unchanged, so callers can migrate on their
  own schedule.

- `TaxCategory` variants `K` (EEA supply), `G` (free export), `O` (outside
  scope), `L` (Canary Islands), and `M` (Ceuta and Melilla). The changelog
  entry for 0.1.0 already listed `K`, `G`, `O`, and `L`, but only `S`, `Z`,
  `E`, and `AE` were implemented, so those invoices failed to parse.
- `InvoiceTypeCode::Other(u16)` carries any other UN/EDIFACT 1001 document type
  code, plus `InvoiceTypeCode::from_code`, `is_credit_note`, and `value`. Codes
  outside `1..=9999` or that are not numbers are rejected.

### Fixed

- A credit note is serialized with the credit-note element names:
  `cbc:CreditNoteTypeCode`, `cac:CreditNoteLine`, and `cbc:CreditedQuantity`.
  It previously emitted the invoice spellings on a `CreditNote` root, which is
  not valid UBL and is rejected by PEPPOL receivers.
- The parser handles `cac:CreditNoteLine`. It did not, so **every line of a
  credit note was silently dropped** on read.
- The document type code no longer has to be `380` or `381`. Other UN/EDIFACT
  1001 codes (`384` self-billed credit note, `386` prepayment invoice, `389`
  self-billed debit note, `261`/`262`, `393`, `80`, …) round-trip. A `CreditNote`
  root normalises the code to `CreditNote`, because the root — not the number —
  determines the line element names; preserving the sub-code would re-emit a
  credit note as an invoice.

- The XML parser now scopes every element by its structural position rather
  than by element name alone, via a `Parents` path view. UBL reuses element
  names across unrelated parts of the document, so name-only matching silently
  wrote one part's value over another's:
  - `cac:TaxScheme/cbc:ID` ("VAT") no longer overwrites
    `cac:TaxCategory/cbc:ID` ("S"). Because the category became unknown, the
    entire tax subtotal was dropped — the invoice then failed validation with a
    totals mismatch that pointed nowhere near the real cause.
  - `cac:DeliveryParty`, `cac:PayeeParty`, `cac:TaxRepresentativeParty`, and
    bare `cac:Party` elements no longer overwrite the supplier or customer name.
  - `cac:AdditionalDocumentReference/cbc:ID` and other nested `cbc:ID` elements
    no longer overwrite the invoice ID.
- An unparsable or non-finite amount is now an error instead of silently
  becoming `0.0`. Defaulting turned a corrupt document into one that parsed
  cleanly and then failed an arithmetic validation rule, sending the reader
  after the wrong field.
- An unknown tax category code is reported rather than dropping the subtotal.

### Added

- `Invoice::from_xml_with_limits` applying `tpt_doc_core::Limits`: nesting is
  refused past `max_xml_depth` rather than walked. A UBL invoice is only a few
  levels deep, so anything deeper is corrupt or hostile.

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
- Emitted text is validated against the XML 1.0 `Char` production, so a C0
  control in a party name, line item, invoice ID, or currency code is reported
  with its code point rather than producing an invoice a PEPPOL receiver rejects.
- Integration tests covering round-tripping, arithmetic totals, and validation
  failures, plus `proptest` property tests.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-ubl-v0.1.0

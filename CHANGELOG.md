# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial workspace scaffold with 7 crate skeletons
- GitHub Actions CI (fmt, clippy, deny, test, coverage)
- mdBook documentation skeleton

### Changed
- Workspace expanded from 7 to 10 crates: `tpt-doc-word` (.docx OOXML),
  `tpt-doc-ubl` (UBL 2.1/PEPPOL BIS e-invoicing), `tpt-doc-hl7v2`
  (streaming HL7 v2.x)
- **tpt-doc-spreadsheet**: streaming `.xlsx` reader (ZIP entries +
  `quick-xml` event loop, shared strings, inline strings, entity
  references) and spec-compliant writer; round-trip proptest over an
  independently built fixture
- **tpt-doc-word**: `.docx` writer (document.xml, styles, settings,
  headers/footers, ZIP assembly) and plain-text reader; known-good
  fixture, round-trip proptest, deterministic-XML snapshot
- **tpt-doc-fhir**: `Observation`, `Encounter`, `Organization` with
  two-mandatory-field type-state builders; FHIR XML (de)serialization for
  `Patient`/`Observation`; cross-field validators returning
  `ValidationReport`; typed `Bundle` with `resourceType` dispatch
- **tpt-doc-edi**: embedded UN/EDIFACT D.96A and X12 835/837/270/271
  schema tables with element-level, envelope, and transaction-set
  validation; EDIFACT and X12 fixtures with proptests
- **tpt-doc-hl7v2**: MSH-2 delimiter auto-detection, standard MSH
  numbering, typed field/component/repeat access, message encoder,
  embedded v2.5.1 segment definitions with message validation
- **tpt-doc-pdf**: full serializer with byte-exact 20-byte xref entries,
  deflate content streams, built-in Type1 fonts, JPEG passthrough, PNG
  transcoding, file attachments (PDF 2.0 associated files); spec-walkable
  parse-back tests and deterministic snapshots
- **tpt-doc-sign**: PKCS#8 PEM keys, hand-built DER/CMS `CAdES-BES`
  detached signatures (RSA-PSS SHA-256), XAdES enveloped `rsa-sha256`,
  PAdES `/ByteRange` incremental updates — all verified against the
  `openssl` CLI
- **tpt-doc-layout**: lenient HTML parser (block subset), inline CSS
  (`font-size`, `font-weight`, `text-align`, `margin`, `padding`, page
  breaks), line-box layout with word wrapping and fixed-width bordered
  tables; deterministic output, tag-soup fuzz proptests
- **tpt-doc-ubl**: typed `Invoice`/`CreditNote` with UBL 2.1 XML
  (de)serialization, PEPPOL BIS 3.0 validation rules (BR-01, BR-16,
  BR-CO-09/10/13/15/25, PEPPOL-EN16931-R001/R062), PEPPOL fixture,
  round-trip proptests, optional `facturx` feature embedding the invoice
  XML into a PDF

### Fixed
- `quick-xml` 0.42 (RUSTSEC-2026-0194, RUSTSEC-2026-0195)
- `ttf-parser` dependency removed (RUSTSEC-2026-0192); TrueType
  subsetting will use `skrifa`
- `deny.toml` migrated to the cargo-deny 0.20 schema
- `serde_json` `float_roundtrip` enabled for exact monetary round-trips

[Unreleased]: https://github.com/tpt-solutions/tpt-doc/compare/HEAD...HEAD

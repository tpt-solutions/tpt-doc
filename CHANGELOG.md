# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `tpt_doc_edi::x12_ack::build_999` and `parse_999`: X12 999 implementation acknowledgments (IK5 responses), sharing the 997 envelope code; a set with no response is refused
- `tpt_doc_edi::x12_856::parse_856`: X12 856 advance ship notices (shipment header, item levels, CTT check), read-only
- `tpt_doc_edi::x12_810::parse_810`: X12 810 invoices (header, buyer, lines, total in cents), read-only
- `tpt_doc_edi::x12_850::parse_850`: X12 850 purchase orders (header, buyer, lines, CTT check), read-only
- `tpt_doc_pdf::extract`: text extraction for classic-structure PDFs (xref table, `/Prev` chains, Flate streams, page tree); runs on one line are joined, and a new line starts when the position moves
- `tpt_doc_edi::edifact_ack`: EDIFACT CONTRL acknowledgments (interchange level), built and read back
- `tpt_doc_edi::x12_ack::parse_997`: reads a 997, checking AK9 counts against the sets listed
- `tpt_doc_hl7v2::batch`: HL7 v2 batches (BHS/BTS) built and read back; the BTS count must match
- `tpt_doc_edi::x12_ack::build_997`: X12 997 functional acknowledgments (swapped envelope, AK1/AK2/AK5/AK9)
- `tpt_doc_hl7v2::encode::escape_text`, the free-text escaper, now public
- `tpt_doc_hl7v2::ack`: HL7 v2 acknowledgments (AA/AE/AR) that swap the parties,
  name the original control ID in MSA-2, and escape free text
- `tpt_doc_hl7v2::mllp`: MLLP framing; `unframe` refuses truncated frames and stray bytes
- `Message::push_escaped_segment` for fields the caller has already escaped
- `templates/`: a gallery of five worked examples (HL7 to FHIR, X12 835 summary,
  UBL to Factur-X, CSV to XLSX, HTML to a PAdES-signed PDF), each checked against
  a golden output by `tpt-doc-templates`
- `CsvReader::headers`
- `tpt-doc-audit`: a tamper-evident audit log (SHA-256 hash chain, JSON Lines
  persistence that refuses edited files, and a CAdES seal over the head)
- `tpt-doc-core::tsv`: a strict reader for schema tables; `from_tsv` loaders
  for HL7 segment tables and EDIFACT/X12 element and transaction tables
- FHIR XML for `Encounter` and `Organization`
- `tpt-doc-mcp`: a Model Context Protocol server (`tpt-mcp` binary) exposing
  `validate_document`, `convert_hl7_to_fhir`, `extract_text`, and
  `generate_invoice` over stdio
- `tpt-doc-ooxml`: internal crate shared by the XLSX and DOCX writers and
  readers (`PackageWriter`, `open_archive`, `read_part`)
- `DocReader`, `DocWriter`, and `Validate` implemented for `Invoice`,
  the FHIR resources, and `DocxDocument`
- serde models: spreadsheet `Cell`/`Row`, the Word model, and (behind a
  `serde` feature) HL7 v2 `Message`, EDIFACT `Segment` (serialize only), and
  the PDF `Document`
- `tpt_doc_core::Limits` with bounded `read_to_end`, `check_rows`, and
  `check_xml_depth`; enforced in the `.xlsx` and `.docx` ZIP readers, the
  PNG IDAT inflate, and the XML parsers in spreadsheet, word, ubl, and fhir
- `XlsxReader::with_limits`, `DocxReader::extract_text_with_limits`,
  `Invoice::from_xml_with_limits`, `Patient::from_xml_with_limits`, and
  `Observation::from_xml_with_limits`
- `tpt_doc_pdf::image::ColorSpace` on `Image`, so JPEG component counts map
  to `/DeviceGray`, `/DeviceRGB`, or `/DeviceCMYK`
- Initial workspace scaffold with 7 crate skeletons
- GitHub Actions CI (fmt, clippy, deny, test, coverage)
- mdBook documentation skeleton

### Fixed
- DOCX tables written without explicit column widths now write the same cell widths the grid uses, so a write-read-write trip is byte-identical (the document snapshot was updated for this)
- PAdES incremental updates wrote a `startxref` relative to the appended update instead of the file, so readers that follow the trailer chain could not find the new xref table. Found by the text extractor; a regression test now checks it.

### Changed
- Patient and Observation XML readers now use the shared `xml::read_resource` driver
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
- `tpt-doc-pdf`: dangling font and image ids in `Page` operations are rejected
  before serialization instead of naming a nonexistent object number; non-finite
  coordinates are refused rather than written as `NaN`/`inf` into a content
  stream; attachment MIME subtypes are escaped as PDF name tokens (all
  delimiters, not just `/`); a PNG palette index past the end of `PLTE` is an
  error rather than black
- `quick-xml` 0.42 (RUSTSEC-2026-0194, RUSTSEC-2026-0195)
- `ttf-parser` dependency removed (RUSTSEC-2026-0192); TrueType
  subsetting will use `skrifa`
- `deny.toml` migrated to the cargo-deny 0.20 schema
- `serde_json` `float_roundtrip` enabled for exact monetary round-trips

[Unreleased]: https://github.com/tpt-solutions/tpt-doc/compare/HEAD...HEAD

# tpt-doc — Task Checklist

> **Project:** Enterprise Document & Data Substrate  
> **Org:** TPT Solutions · **License:** MIT OR Apache-2.0 · **Lang:** Rust 2024

---

## Phase 0: Workspace Scaffold

- [x] Init workspace `Cargo.toml` with all 7 member crates (expanded to 10)
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` files
- [x] Add `README.md` with project overview, license badges, crates.io badges
- [x] Add `CHANGELOG.md` (Keep a Changelog format)
- [x] Add `rustfmt.toml` (edition = "2024", grouped imports)
- [x] Add `clippy.toml` (msrv)
- [x] Add `deny.toml` (license allowlist, advisory deny, ring exception)
- [x] Create `.github/workflows/ci.yml` (fmt → clippy → deny → test → no_std → coverage)
- [x] Create `.github/workflows/pages.yml` (mdBook + rustdoc → GitHub Pages)
- [x] Create `docs/` mdBook skeleton (book.toml, SUMMARY.md, intro, architecture, 10 crate chapters)

## Phase 0: Per-Crate Skeletons

- [x] `crates/tpt-doc-core/` — Cargo.toml, src/lib.rs, src/error.rs, src/buf.rs, src/traits.rs
- [x] `crates/tpt-doc-spreadsheet/` — Cargo.toml, src/lib.rs, src/xlsx.rs, src/csv.rs
- [x] `crates/tpt-doc-fhir/` — Cargo.toml, src/lib.rs, src/patient.rs, src/types.rs, src/validate.rs
- [x] `crates/tpt-doc-edi/` — Cargo.toml, src/lib.rs, src/edifact.rs, src/x12.rs
- [x] `crates/tpt-doc-pdf/` — Cargo.toml, src/lib.rs, src/document.rs, src/page.rs, src/font.rs, src/xref.rs
- [x] `crates/tpt-doc-sign/` — Cargo.toml, src/lib.rs, src/pades.rs, src/xades.rs, src/cades.rs
- [x] `crates/tpt-doc-layout/` — Cargo.toml, src/lib.rs, src/render.rs, src/html.rs, src/css.rs
- [x] `crates/tpt-doc-word/` — Cargo.toml, src/lib.rs, src/document.rs, src/paragraph.rs, src/styles.rs, src/table.rs
- [x] `crates/tpt-doc-ubl/` — Cargo.toml, src/lib.rs, src/invoice.rs, src/types.rs, src/validate.rs
- [x] `crates/tpt-doc-hl7v2/` — Cargo.toml, src/lib.rs, src/parser.rs, src/segment.rs, src/encode.rs
- [x] Create `tests/` and `tests/fixtures/` directories in each crate that needs them

---

## Phase 1 — Months 1–2: Core + Spreadsheet

### tpt-doc-core
- [x] Verify `cargo check --target thumbv7em-none-eabihf` (no_std check)
- [x] Write unit tests for `BufSlice` (beyond what's in the file)
- [x] Write unit tests for `DocError` display and source chain
- [x] Write unit tests for `ValidationReport` merge

### tpt-doc-spreadsheet
- [x] Implement ZIP entry streaming over `.xlsx` bytes (`zip` crate)
- [x] Implement `quick-xml` streaming parser for `xl/worksheets/sheet1.xml`
- [x] Implement shared strings table (`xl/sharedStrings.xml`) lookup
- [x] Yield rows as `Vec<Cell>` via `Iterator` (no full-DOM load)
- [x] Implement `.xlsx` writer: workbook XML + worksheet XML assembled into ZIP
- [x] Add fixture file `tests/fixtures/simple.xlsx` (hand-written OOXML zipped with Python `zipfile` — independent of `XlsxWriter`)
- [x] Integration test: parse `simple.xlsx` and assert row/cell values
- [x] Integration test: write xlsx → read back → assert equal
- [x] proptest: round-trip arbitrary rows through xlsx encode → decode

---

## Phase 1.5 — Month 2–3: Word / DOCX

### tpt-doc-word
- [x] Implement `[Content_Types].xml` and `_rels/.rels` OOXML package parts
- [x] Implement `word/document.xml` serializer from `DocxDocument` body
- [x] Implement `word/styles.xml` with Normal, Heading1–6, Table styles (only styles actually used are emitted)
- [x] Implement `word/settings.xml` (compatibility settings)
- [x] Assemble parts into ZIP via `zip` crate (same pattern as tpt-doc-spreadsheet)
- [x] Implement `Paragraph` with `Run` elements and `RunStyle` (bold, italic, underline, font size)
- [x] Implement `Table` with rows, cells, and basic border styles
- [x] Implement headers and footers (`word/header1.xml`, `word/footer1.xml` + `sectPr` references)
- [x] Implement basic `.docx` reader: extract plain text from `word/document.xml` (paragraph breaks, tabs, line breaks, entities)
- [x] Integration test: hand-written known-good fixture (`tests/fixtures/simple.docx`, Python `zipfile`) parsed and asserted
- [x] proptest: round-trip paragraph text through docx encode → text extract
- [x] Snapshot test (`insta`): deterministic XML for a fixture document

---

## Phase 2 — Months 3–4: FHIR + EDI

### tpt-doc-fhir
- [x] Expand type-state pattern to additional mandatory-field combos (`Observation`/`Encounter` enforce two mandatory fields in either order)
- [x] Implement more FHIR R5 resource types (Observation, Encounter, Organization)
- [x] Implement `to_xml()` / `from_xml()` via `quick-xml` (`Patient`, `Observation`)
- [x] Add runtime validation for cross-field constraints (date ranges, required-if) returning `ValidationReport`
- [x] Add fixture `tests/fixtures/patient_bundle.json` (FHIR R5 sample bundle)
- [x] Integration test: parse the fixture bundle and assert resource counts
- [x] proptest: builder round-trips for all resource types (incl. Bundle; requires `serde_json/float_roundtrip`)

### tpt-doc-edi
- [x] Embed EDIFACT D.96A schema tables (`include_str!` from `schemas/edifact/`)
- [x] Embed X12 835/837/270/271 schema tables (`schemas/x12/`)
- [x] Schema validation: required data elements, data type/length checks, envelope rules (UNB/UNZ, UNH/UNT counts), transaction-set rules (required segments, ST/SE bracketing)
- [x] Add fixture `tests/fixtures/sample.edifact` and `tests/fixtures/sample_835.x12`
- [x] Integration test: parse fixtures and assert segment/element values
- [x] proptest: round-trip segment data; no-panic property over arbitrary `&[u8]`

---

## Phase 2.5 — Month 4–5: HL7 v2.x

### tpt-doc-hl7v2
- [x] Implement streaming HL7 v2.x tokeniser (segment separator `\r`, field `|`, component `^`, repeat `~`, escape `\`, subcomponent `&`)
- [x] Read encoding characters from `MSH-2` to handle non-default delimiters
- [x] Yield zero-allocation `Segment<'a>` items borrowing from input `&[u8]` (same pattern as tpt-doc-edi)
- [x] Implement typed access: `segment.field(n)`, `segment.component(n, m)`, `segment.repeat(n, r)`
- [x] Implement message encoder: build `MSH` and append segments, serialize to `\r`-delimited bytes
- [x] Embed segment definitions for MSH, PID, PV1, OBR, OBX, NTE, EVN, AL1, DG1 (v2.5.1)
- [x] Add fixture `tests/fixtures/adt_a01.hl7` (ADT^A01 admit), `tests/fixtures/oru_r01.hl7` (lab result)
- [x] Integration test: parse fixtures and assert MSH-9 message type, PID-3 patient ID
- [x] proptest: no-panic property over arbitrary `&[u8]` input

---

## Phase 3 — Months 5–6: PDF + Sign + Layout

### tpt-doc-pdf
- [x] Implement object ID allocator and indirect object table (`ObjectWriter`)
- [x] Implement PDF serialiser: object stream → byte-exact xref table (free head entry 0, 20-byte entries)
- [x] Implement font embedding: built-in Type1 (Helvetica/Times/Courier families, WinAnsiEncoding)
- [ ] Implement TrueType font subsetting via `skrifa` (ttf-parser unmaintained, RUSTSEC-2026-0192)
- [x] Implement image embedding (JPEG passthrough via SOF parsing; PNG → inflate + unfilter + RGB + flate2 deflate)
- [x] Implement deflate-compressed content streams via `flate2`
- [x] Verify xref offsets with byte-exact assertion tests (spec-walkable parse-back integration test)
- [x] Integration test: generate a multi-page PDF and walk it with a miniature spec-compliant reader
- [x] Snapshot test (`insta`): deterministic document structure for a fixture document

### tpt-doc-sign
- [x] Verify `ring` license against `deny.toml` (ISC exception present, `cargo deny check` green)
- [x] Implement `PrivateKey::from_pem()` → `ring::signature::RsaKeyPair` (PKCS#8, RSA-PSS + PKCS#1 v1.5 SHA-256)
- [x] Implement CAdES detached signature (CMS `SignedData`, SHA-256/RSA-PSS, signed attrs, `signingCertificateV2`) — verified by `openssl cms -verify`
- [x] Implement XAdES enveloped XML signature (`<ds:SignedInfo>`, `rsa-sha256`, cert embedding) — SignatureValue verified by `openssl dgst`
- [x] Implement PAdES: CMS sig integrated into PDF `/ByteRange` incremental update — verified by `openssl cms -verify` over the byte ranges
- [x] Implement X.509 certificate embedding in `SignedData` (signer cert; chain collection when intermediates provided)
- [ ] Integration test: sign a document, verify signature with `openssl` CLI in CI
- [ ] Integration test: HIPC 2020 compliance fixture (if reference fixtures available)

### tpt-doc-layout
- [x] Implement HTML parser (hand-rolled lenient tokenizer for the block subset — avoids html5ever's dependency tree; entities decoded, head content dropped)
- [x] Implement inline CSS `parse_inline()` for supported property subset (`font-size`/`pt`+`px`, `font-weight`, `text-align`, `margin`, `padding`, page breaks)
- [x] Implement block layout engine: line-box model (word wrapping with deterministic metric approximation), headings, `page-break-before`/`after`
- [x] Implement table layout: fixed-width columns, stroked cell borders, row page-breaks
- [x] Connect layout output → `tpt_doc_pdf::Document` content streams (four Helvetica faces, ids stable by construction)
- [x] Integration test: render a fixture HTML report → PDF and assert decompressed content streams (text ops, entity decoding, cell-border count, bold faces)
- [x] proptest: fuzz arbitrary HTML and tag-soup input, assert no panics (128 cases)

---

## Phase 4 — Month 7–8: UBL / PEPPOL e-Invoicing

### tpt-doc-ubl
- [ ] Implement `Invoice` type covering UBL 2.1 mandatory elements (ID, IssueDate, Supplier, Customer, LineItems, TaxTotal, LegalMonetaryTotal)
- [ ] Implement `CreditNote` mirroring Invoice structure
- [ ] Serialize to UBL 2.1/2.3-compliant XML with correct namespaces (`urn:oasis:names:specification:ubl:schema:xsd:Invoice-2`)
- [ ] Parse incoming UBL XML invoices → typed `Invoice` struct
- [ ] Implement PEPPOL BIS Billing 3.0 validation rules (schematron-style, embedded as Rust assertions)
- [ ] Implement `Party` (supplier/customer): name, address, VAT/tax ID, endpoint ID
- [ ] Implement `InvoiceLine`: item description, quantity, unit price, line extension amount
- [ ] Implement `TaxSubtotal`: tax category (S/Z/E/AE), rate, taxable amount, tax amount
- [ ] Add Factur-X/ZUGFeRD hybrid PDF+XML output as optional feature (`facturx` feature flag, depends on `tpt-doc-pdf`)
- [ ] Add fixture `tests/fixtures/peppol_invoice.xml` (valid PEPPOL BIS 3.0 invoice)
- [ ] Integration test: parse fixture and assert supplier name, total amounts, line count
- [ ] Integration test: generate invoice → serialize → parse back → assert equal
- [ ] proptest: round-trip `Invoice` through serialize → parse

---

## Cross-Cutting / Ongoing

- [ ] All public items have rustdoc `///` comments with `# Examples` sections
- [ ] mdBook chapter content for each crate (architecture + usage guide)
- [x] `cargo deny check` passes: zero advisory violations, all licenses allowed
- [ ] CI green on main for all 3 phases (fmt, clippy, deny, test, no_std, coverage)
- [ ] crates.io metadata complete in every `Cargo.toml` (description, keywords, categories)
- [ ] All crates at `0.1.0` on first publish; no `publish = false` blockers
- [ ] CHANGELOG updated per release (Keep a Changelog format)
- [ ] GitHub release tags aligned to crate versions
- [ ] GitHub repo created at `github.com/tpt-solutions/tpt-doc` and CI enabled

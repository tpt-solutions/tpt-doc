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
- [x] Integration test: sign a document, verify signature with `openssl` CLI (CAdES + PAdES via `openssl cms -verify`, XAdES via `openssl dgst -verify`; skips gracefully when openssl is absent)
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
- [x] Implement `Invoice` type covering UBL 2.1 mandatory elements (ID, IssueDate, Supplier, Customer, LineItems, TaxTotal, LegalMonetaryTotal) plus PEPPOL CustomizationID/ProfileID
- [x] Implement `CreditNote` via `InvoiceTypeCode::CreditNote` (381) — same structure, distinct root element and type code in XML
- [x] Serialize to UBL 2.1-compliant XML with correct namespaces (`urn:oasis:names:specification:ubl:schema:xsd:Invoice-2` / `CreditNote-2`, cac:/cbc: prefixes, currencyID attributes)
- [x] Parse incoming UBL XML invoices → typed `Invoice` struct (both Invoice and CreditNote roots, entity references, per-leaf text buffering)
- [x] Implement PEPPOL BIS Billing 3.0 validation rules (BR-01, BR-16, BR-CO-09/10/13/15/25, PEPPOL-EN16931-R001/R062 as Rust assertions)
- [x] Implement `Party` (supplier/customer): name, address, VAT/tax ID, endpoint ID
- [x] Implement `InvoiceLine`: item description, quantity, unit price, line extension amount (auto-computed, 2dp rounding)
- [x] Implement `TaxSubtotal`: tax category (S/Z/E/AE), rate, taxable amount, tax amount
- [x] Add Factur-X/ZUGFeRD hybrid PDF+XML output as optional feature (`facturx` feature flag, depends on `tpt-doc-pdf`; PDF 2.0 associated-file embedding with /AF, /Names /EmbeddedFiles, /AFRelationship /Data; PDF/A-3 XMP left to the caller)
- [x] Add fixture `tests/fixtures/peppol_invoice.xml` (valid PEPPOL BIS 3.0 invoice)
- [x] Integration test: parse fixture and assert supplier name, total amounts, line count + PEPPOL validation + tampered-total detection
- [x] Integration test: generate invoice → serialize → parse back → assert equal
- [x] proptest: round-trip `Invoice` through serialize → parse (integer-cent monetary values, xml-safe strings)

---

## Cross-Cutting / Ongoing

- [x] All public items have rustdoc `///` comments (missing_docs enforced in CI); every crate's root item carries a compile-checked `# Examples` doctest
- [x] mdBook chapter content for each crate (architecture + usage guide; 10 chapters in docs/src/crates/)
- [x] `cargo deny check` passes: zero advisory violations, all licenses allowed
- [ ] CI green on main for all 3 phases (fmt, clippy, deny, test, no_std, coverage)
- [x] crates.io metadata complete in every `Cargo.toml` (description, keywords, categories, rust-version — verified for all 10 crates)
- [x] All crates at `0.1.0` on first publish; no `publish = false` blockers
- [x] CHANGELOG updated per release (Keep a Changelog format; Unreleased section tracks all implementation work)
- [ ] GitHub release tags aligned to crate versions
- [ ] GitHub repo created at `github.com/tpt-solutions/tpt-doc` and CI enabled

---

## Review Follow-ups (2026-10-02)

> Source: platform review. Findings come from reading source; not yet reproduced by running code (B1 was re-checked by hand). Fix each bug test-first.

### Step 1 — Correctness hotfixes
- [x] **B1** FIXED: `write_attachment` now uses the captured `stream_id` for `/EF /F` and `/EF /UF` (was self-referencing the filespec). Added an xref-walking object-graph test in `crates/tpt-doc-pdf/tests/pdf.rs` (3 tests) that resolves indirect refs rather than substring-matching.
- [x] **B2** FIXED: added `escape_pdf_string_bytes` returning WinAnsi bytes; `content_stream` now builds `Vec<u8>` and writes raw bytes. Latin-1 chars emit one byte (`café` -> `0xE9`); `\r`/`\n`/`\t` now escaped. Dead `escape_pdf_string` removed.
- [x] **B8** FIXED: added `column_index()` (bijective base-26) and column-aware `finish_cell()` that pads skipped columns with `Cell::Blank`. Cells without an `r` attribute still append sequentially. 5 new tests.
- [x] FIXED: `edifact.rs`, `x12.rs`, and `hl7v2/parser.rs` now loop instead of recursing on empty segments. Regression tests cover 200k empty segments in all three parsers.

### Step 2 — Hardening
- [ ] Shared `Limits` in core (max decompressed bytes, rows, nesting depth); apply to xlsx/docx zip entries, PNG IDAT inflate, XML depth
- [ ] pdf `image.rs`: bounds-check IHDR/CRC slicing, overflow-check width*height, reject oversized dimensions, verify CRC, handle PLTE/alpha properly
- [ ] pdf: validate font/image ids in `Page`, read JPEG component count (grayscale/CMYK), sanitise attachment `mime_subtype`, reject NaN/inf in `format_number`
- [ ] EDIFACT: apply release character `?` and unescape elements/components; error on unterminated trailing segment; error (not silently fold) on `splitn` field overflow -- **BLOCKED**: `Segment::elements()` returns a slice of `&str` borrowed from the input, so unescaping needs owned storage. Requires a breaking API change to `Segment` (or `Cow<'a, str>`). Deferred rather than half-done.
- [x] PARTIAL (edi): `X12Parser::new` validates ISA01/ISA16/terminator (printable, distinct) via 6 tests in `tests/x12_isa.rs`. NOT DONE: repetition separator support, truncated-interchange error.
- [x] PARTIAL (hl7v2): `Message::to_bytes` now escapes the field separator, escape character, CR and LF via `escape_value` (4 tests). `component(n,0)` / `repeat(n,0)` guarded. NOT DONE: CRLF/LF endings, Latin-1 (MSH-18), MSH delimiter validation.
- [ ] Reject XML 1.0-illegal control characters in all XML writers (spreadsheet, word, ubl, fhir, xades)
- [ ] sign: check key matches certificate public key and validity period; accept PKCS#1 keys; add ECDSA; fix `der::integer` sign handling and `der::oid` silent arc drop

### Step 3 — Standards fixes
- [ ] **B3** sign `cades.rs`: `signingCertificateV2` needs `SEQUENCE OF` level; omit default `hashAlgorithm`; validate with a strict validator (EU DSS / Adobe)
- [ ] **B4** sign `xades.rs`: add `QualifyingProperties`/`SignedProperties`/`SigningCertificate`; canonicalise (C14N) before digesting; sign `SignedInfo` in namespace context
- [ ] **B5** sign `pades.rs`: add `/AcroForm`, `/SigFlags`, signature field + widget, use `ETSI.CAdES.detached`, real signing time, robust trailer/`/Root` parsing, preserve `/Info` and `/ID`, search placeholder from `base`, support xref streams
- [ ] sign: add a public verification API, certificate-chain input (intermediates), RFC 3161 timestamp (PAdES-T/LT)
- [ ] **B6** ubl parser: scope elements by path (TaxScheme/ID vs TaxCategory/ID, line IDs, party/company IDs); support tax categories K, G, O, L, M; stop silently defaulting bad amounts to 0 (error instead)
- [ ] ubl: replace `f64` money with decimal/minor-units type; read `currencyID`; reject NaN/inf
- [ ] **B7** ubl CreditNote: emit `CreditNoteLine` / `CreditedQuantity` / `CreditNoteTypeCode`; parse them back; accept other type codes (384, 389, …)
- [ ] ubl: emit mandatory BIS 3.0 elements (TaxScheme, unitCode, PartyLegalEntity, endpoint schemeID, BuyerReference, PaymentMeans, ClassifiedTaxCategory, due date, address)
- [ ] ubl: expand rule coverage beyond 9 rules toward EN 16931 + Peppol BIS 3 (~200); add CII (Factur-X native) and XRechnung
- [ ] ubl `facturx`: PDF/A-3 + XMP, CII payload (depends on B1)
- [ ] **B9** layout: render nested blocks (div > h1/p, tables in divs), implicit `<p>` close, wrap table cell text, honour margin/padding/font-weight, handle `>` inside quoted attributes
- [ ] fhir: preserve unmodelled fields (extensions, meta, text, `_field`); check `resourceType`; fix XML parser (Start/End pairs, depth-scoped elements); decimal `Quantity`; validate date formats; XML for all resources
- [ ] spreadsheet: resolve sheet via `workbook.xml` + rels (not hard-coded `sheet1.xml`); multi-sheet; formulas, styles, dates; drop rich-text phonetic `<rPh>` text; error on bad shared-string index; CSV formula-injection guard
- [ ] edi: implement ST/SE, GS/GE, ISA/IEA control-number and count checks promised in docs; report missing-segment index correctly; guard `position - 1`

### Step 4 — Foundation
- [ ] Implement `DocReader` / `DocWriter` / `Validate` in every crate (currently unused); unify method names (`write`/`finish`/`to_bytes`/`to_xml`/`to_json`)
- [ ] Extend `ValidationReport`: severity, rule ID, location (path/offset/segment index), serde; adopt in ubl, edi, hl7v2, fhir (replace `UblValidationError`, `SchemaViolation`, `Hl7Violation`, `FhirValidationError`)
- [ ] `DocError` positional info (byte offset / line / segment) instead of bare strings
- [ ] Share EDI/HL7 segment tokenizer + TSV schema-table loader; share OOXML packaging between spreadsheet and word
- [ ] Add serde to all models (HL7 `Message`, EDI segments, spreadsheet `Row`/`Cell`, `DocxDocument`, pdf `Document`)
- [ ] `Write`-based streaming writers and `Read + Seek` readers; stop buffering whole xlsx sheets; drop per-segment `Vec` allocations to match the "zero-allocation" claim
- [ ] Runtime-loadable EDI/HL7 schemas (tables are `&'static str` today)

### Step 5 — Adoption & docs
- [x] FIXED: root README lists all 10 crates with changelog links, and the quick-start is now exercised by `crates/tpt-doc-sign/tests/quickstart.rs` (was broken: wrong `build()` arg and a non-existent `tpt_doc_sign::prelude`). Roadmap extension NOT yet done.
- [ ] Fix doc drift: `docs/src/introduction.md`, `architecture.md`, `spec.txt`, `CHANGELOG.md` still say 7 crates; `docs/src/crates/sign.md` references non-existent `load_private_key` / `SignedDocument` / `CertChain`
- [ ] `examples/` per crate: HTML→PDF, FHIR Patient→JSON, HL7 ADT parse, X12 835 parse, xlsx round-trip, UBL invoice + Factur-X, PAdES sign
- [ ] Per-crate `README.md` + `readme`/`documentation` in each `Cargo.toml`; per-crate keywords (max 5, relevant)
- [ ] `templates/` gallery with sample data + expected output: NZ-style tax invoice (UBL+PDF), HL7 ADT→FHIR, 835 remittance parser, spreadsheet report, signed PDF contract
- [ ] `tpt-doc` facade crate with per-format feature flags and a unified prelude (only fhir, hl7v2, ubl, word have preludes today); `serde` / `std` feature toggles; make heavy deps (`ring`, `x509-parser`, `zip`) optional
- [ ] Community files: `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, issue/PR templates, `CODEOWNERS`, dependabot
- [ ] CI: MSRV job (1.85), separate `cargo test --doc`, `cargo semver-checks`, run examples; add `rust-toolchain.toml`, `.devcontainer`, Dockerfile; add `.kilo/` to `.gitignore`
- [ ] Tag-triggered publish workflow (release-plz / cargo-release)
- [ ] mdBook: tutorials, recipes, migration + compliance guides (NZISM, HIPC), error-handling guide, longer crate chapters
- [ ] `cargo-fuzz` targets for hl7v2, edi, fhir JSON/XML, xlsx, docx, png/pdf; criterion benches to back performance claims

### Step 6 — New features
- [ ] `tpt` CLI crate (`validate | convert | inspect | diff | sign | render`) with format sniffing; revisit `Cargo.lock` gitignore once a binary exists
- [ ] HL7v2 → FHIR converter (ADT^A01: PID→Patient, PV1→Encounter; ORU^R01: OBX→Observation; Bundle)
- [ ] JSON → e-invoice pipeline (Invoice → validate → PDF → Factur-X → PAdES); blocked on B1, B3, B5, B6, B7
- [ ] Data-merge templating (`{{field}}` HTML→PDF; same JSON fills DOCX/XLSX templates)
- [ ] PHI redaction / de-identification by field path (HL7, FHIR), aligned with HIPC 2020
- [ ] Structural diff (HL7, EDI, FHIR, DOCX/XLSX) using JSON-pointer-style paths
- [ ] Audit/provenance hash chain (embedded PDF attachment or FHIR Provenance), signed
- [ ] WASM playground (`wasm-bindgen`; check `ring` on wasm32 for sign)
- [ ] MCP server (`validate_document`, `convert`, `extract_text`, `generate_invoice`); needs serde + structured reports first
- [ ] Format gaps: X12 850/810/856/997/999 + loops, EDIFACT INVOIC/ORDERS/DESADV, EDI writer + 999 ack, HL7 ACK + MLLP + batch, more FHIR resources/R4 + profile validation, DOCX structural read/lists/images/hyperlinks, PDF parser/text extraction/PDF/A/merge-split

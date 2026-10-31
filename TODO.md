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
- [x] DONE: shared `tpt_doc_core::Limits` (`max_entry_bytes`, `max_rows`, `max_xml_depth`, bounded `read_to_end`, plus `check_rows`/`check_xml_depth`) enforced everywhere it applies — xlsx and docx ZIP entry reads, `max_rows` in the xlsx row reader, `max_xml_depth` in the xlsx shared-strings/worksheet, docx, ubl, and fhir XML parsers, and the PNG IDAT inflate. New `*_with_limits` entry points let callers tighten or relax the bounds.
- [x] MOSTLY DONE (pdf): IHDR out-of-bounds panic fixed; compression/filter/colour type validated; MAX_PIXELS bound before allocating; chunk length via try_from; per-chunk CRC-32 verified. PLTE indices out of range are now rejected instead of rendered black; RGBA alpha is dropped deliberately and documented (`/DeviceRGB` has no alpha). NOT DONE: tRNS / SMask transparency.
- [x] DONE (pdf): `Page` font/image ids validated against what `Document` actually embeds; JPEG SOF component count read so grayscale/CMYK get `/DeviceGray`/`/DeviceCMYK` (unknown counts rejected); attachment `mime_subtype` escaped as a PDF name token (all delimiters, not just `/`); `format_number` returns `Result` and rejects NaN/inf, so `content_stream` and `write` propagate the error
- [x] FIXED: `Segment::elements_released()` resolves release sequences without a breaking change (`elements()` still returns raw borrowed slices). Splitting now respects released separators, so `A?+B` is one element. Also reports truncated trailing segments and rejects >128 elements.
- [x] DONE (edi): ISA01/ISA11/ISA16/terminator validated (printable, distinct); `repetition_separator()` exposes ISA11; truncated trailing segment reported. 11 tests in `tests/x12_isa.rs`.
- [x] MOSTLY DONE (hl7v2): encoder escapes field/escape chars and CR/LF; `component(n,0)`/`repeat(n,0)` guarded; parser accepts CR, CRLF and bare LF terminators. MSH-1/MSH-2 delimiters validated (printable, mutually distinct). NOT DONE: MSH-18 Latin-1.
- [x] FIXED: shared `escape_xml_text` / `validate_xml_chars` / `is_xml_char` added to tpt-doc-core `xml` module and wired into all XML writers (spreadsheet, word, ubl, fhir). Verified a raw 0x01 was previously written into sheet1.xml; now rejected with the offending code point. NOT DONE: xades needs nothing (base64 only).
- [x] MOSTLY DONE (sign): `der::integer` strips leading zeros *before* deciding on the `0x00` sign pad (it encoded `[0x00,0x80]` as -128) and encodes zero as one octet; `der::oid` returns `Result` instead of silently dropping unparsable arcs (a typo produced a valid OID for a different algorithm) and validates the X.660 leading-arc rules. PKCS#1 keys (`openssl genrsa` default) load via a hand-rolled `PrivateKeyInfo` wrapper with tag/length/trailing-byte checks. New `Certificate` type with `check_key_matches` and `check_validity`, both enforced by `cades::sign_detached_at`, `xades::sign_at`, and `pades::sign_at`. NOT DONE: ECDSA.

### Step 3 — Standards fixes
- [x] MOSTLY DONE (B3) sign `cades.rs`: `signingCertificateV2` now has the `SEQUENCE OF` level and omits the default `hashAlgorithm`; pinned by a DER-structure test. NOT DONE: validation with a strict validator (EU DSS / Adobe).
- [x] MOSTLY DONE (sign `xades.rs`, B4): the signature now carries the XAdES `QualifyingProperties` / `SignedProperties` / `SigningCertificate` block, with `CertDigest` (SHA-256 of the embedded certificate) and `IssuerSerial`, referenced from `SignedInfo` as a second `Reference` so the properties are actually signed rather than merely present. `<ds:Signature>` gained the `Id` that `QualifyingProperties/@Target` names. `SignedInfo` declares `xmlns:ds` on itself and is emitted whitespace-free, so the signature is computed over the element in its own namespace context in a form C14N reproduces. NOT DONE: a real C14N implementation for the *document* reference (the digest is still taken over the input bytes as provided), and `IssuerName` (x509-parser 0.16 cannot render the issuer RDNs back to XML).
- [x] MOSTLY DONE (sign `pades.rs`): trailer parsing is now robust — the `trailer` dictionary is delimited by matching `<<`/`>>` depth (a hex string in `/ID` previously truncated it), the last `startxref` is found across the whole file so a decoy in a stream cannot win, `/Info` and `/ID` are carried into the incremental update's trailer instead of being dropped, and the `/Contents` placeholder is searched from `base` so a zero run in the original body cannot be overwritten. `/M` records the real signing time and `/SubFilter` is `ETSI.CAdES.detached`. The signature is now discoverable: a signature field merged with its widget, an `/AcroForm` with `/SigFlags 3`, and a redefined catalog referencing it (previously the `/Sig` object was cryptographically valid but unreachable). The xref table emits sorted contiguous runs with exact 20-byte entries. NOT DONE: xref-stream support.
- [ ] sign: add a public verification API, certificate-chain input (intermediates), RFC 3161 timestamp (PAdES-T/LT)
- [x] DONE (ubl parser, B6): elements are scoped by structural position instead of element name, via a `Parents` path view. `TaxScheme/ID` ("VAT") no longer overwrites `TaxCategory/ID` ("S") — which silently dropped the whole tax subtotal; `DeliveryParty`/`PayeeParty`/`TaxRepresentativeParty` and bare `cac:Party` elements no longer overwrite the supplier/customer; `AdditionalDocumentReference/cbc:ID` no longer overwrites the invoice ID; the document ID is matched only as a direct child of the root. Malformed and non-finite amounts now error instead of defaulting to `0.0` (which turned a corrupt document into one that then failed an arithmetic rule). Unknown tax categories are reported. Categories `K`, `G`, `O`, `L`, `M` added — the changelog and README already claimed `K`/`G`/`O`/`L`.
- [x] MOSTLY DONE (ubl money): `Money` (i64 cents, saturating add) and `Decimal` (exact base-10, value equality, half-away-from-zero rounding, `percent_of`) replace `f64` throughout the model, parser, writer and BR-CO rules; amounts with >2 decimals, NaN/inf and exponents are rejected; unit prices keep extra decimals. NOT DONE: `currencyID` is still not read per amount, and non-1/100 minor units (JPY, KWD).
- [x] DONE (ubl CreditNote, B7): a `CreditNote` now emits `cbc:CreditNoteTypeCode`, `cac:CreditNoteLine`, and `cbc:CreditedQuantity` instead of the invoice spellings (the old output was not valid UBL and PEPPOL receivers reject it). The parser handles `CreditNoteLine`, which it did not at all — every line of a credit note was silently dropped. Other UN/EDIFACT 1001 type codes (`384`, `386`, `389`, `261`, `262`, `393`, `80`, …) round-trip through `InvoiceTypeCode::Other(u16)` instead of being refused; a `CreditNote` root normalises the type code to `CreditNote` so re-serializing cannot emit invoice line elements.
- [x] MOSTLY DONE (ubl BIS 3.0 elements): TaxScheme on tax categories and parties, unitCode, PartyLegalEntity, endpoint schemeID, BuyerReference, PaymentMeans, ClassifiedTaxCategory, DueDate, StreetName — written in UBL schema order, parsed back, and checked by 12 new rules (BR-02/05/06..11/23/49/62/63, BR-CO-04/26, PEPPOL R003/R010/R020). NOT DONE: XSD/Schematron validation against the official artefacts, PartyIdentification, payment terms, VAT breakdown rules (BR-S-*, BR-Z-*), allowances/charges.
- [x] MOSTLY DONE (ubl VAT rules): BR-CO-18 and the per-category families for S, Z, E, AE, IC, G, O, AF, AG (-01/-05/-08/-09/-10), exemption reason element, LineExtensionAmount in the monetary total, and corrected BR-CO-10/13/15/16/17 ids (they were mislabelled). ~45 rules now. NOT DONE: party VAT-id rules (BR-AE-02..04, BR-IC-02..04), BR-IC-11/12, exemption reason codes, allowances/charges, ~150 further EN 16931 / PEPPOL rules, CII (Factur-X native) and XRechnung.
- [ ] ubl `facturx`: PDF/A-3 + XMP, CII payload (depends on B1)
- [x] DONE (layout, B9): the tokenizer tracks quoting, so a `>` inside a quoted attribute value no longer ends the tag (it used to leak `700">` into the text). `<div>` is a container rather than a paragraph, so nested blocks (`div > h1/p`, tables in divs) are laid out instead of being collected into an inline child list the renderer never walked. `<p>` and headings implicitly close an open paragraph, so `<p>one<p>two` is two paragraphs. `margin` and `padding` are applied (top/bottom margin, scoped left padding) instead of being parsed and discarded, `font-weight: bold` now selects the bold face via `ComputedStyle::is_bold`, and table cell text wraps inside its column — previously the row box grew to fit but the text was still drawn on one line, overflowing the column.
- [ ] fhir: preserve unmodelled fields (extensions, meta, text, `_field`); check `resourceType`; fix XML parser (Start/End pairs, depth-scoped elements); decimal `Quantity`; validate date formats; XML for all resources
- [x] MOSTLY DONE (spreadsheet): the first sheet is resolved via `workbook.xml` + rels (not hard-coded `sheet1.xml`); rich-text phonetic `<rPh>` text is dropped; a bad or out-of-range shared-string index is an error instead of a blank cell. NOT DONE: multi-sheet, formulas, styles, dates, CSV formula-injection guard.
- [x] DONE (edi): `validate_x12_envelope` implements the envelope checks the docs promised but never shipped — `ST`/`SE`, `GS`/`GE`, and `ISA`/`IEA` bracketing, with `SE01` (segments ST→SE), `GE01` (transaction sets in the group), `IEA01` (functional groups), and the control-number agreements `SE02`=`ST02`, `GE02`=`GS06`, `IEA02`=`ISA13`. Every violation is reported at the offending segment's own index; unclosed `ST`/`GS` were previously reported at index 0. A missing `ISA` first segment or `IEA` last segment is now caught.
- [x] DONE (edi): guarded the `elements.get(position - 1)` underflow. Element positions are 1-based, and a schema table row declaring `0` panicked in debug (`attempt to subtract with overflow`) and silently skipped the check in release. It is now reported as a malformed-table violation. Verified by removing the guard and observing the panic.

### Step 4 — Foundation
- [ ] Implement `DocReader` / `DocWriter` / `Validate` in every crate (currently unused); unify method names (`write`/`finish`/`to_bytes`/`to_xml`/`to_json`)
- [x] DONE (extend `ValidationReport`): `Severity` (`Warning`/`Error`), `Location` (`Path`/`Index`/`Offset`), and `rule_id` on every `ValidationIssue`, with builders and `ValidationReport::has_errors`/`errors`/`warnings`. Optional `serde` feature (off by default, keeps the `no_std` build dependency-free). `push(path, message)` still defaults to an error. ADOPTED across all four validating formats: `fhir` findings carry FHIR R5 invariant ids (`obs-7`, `enc-1`, `enc-3`) and `FhirValidationError::to_report` maps the pointer path to `Location::Path`; `ubl::UblValidationError::to_report` preserves the PEPPOL rule as `rule_id`; `edi::SchemaViolation::to_report` and `hl7v2::Hl7Violation::to_report` map the segment index to `Location::Index`. Each format's own error type and API is untouched, so callers can migrate on their own schedule; nothing is deprecated.
- [x] MOSTLY DONE (`DocError` positional info): new `DocError::Located` variant plus `with_location`, `location`, `byte_offset`, and `without_location`. `Display` appends the position and `Error::source` exposes the cause, so error chains keep working. Applied in `ubl`, `spreadsheet`, and `word`: XML parse errors and unresolvable entity references now carry the byte offset `quick_xml` reports. NOT DONE: `edi`/`hl7v2` segment indices and line/column for line-oriented formats are not attached — the parsers know the segment index but no call site uses it yet.
- [ ] Share EDI/HL7 segment tokenizer + TSV schema-table loader; share OOXML packaging between spreadsheet and word
- [ ] Add serde to all models (HL7 `Message`, EDI segments, spreadsheet `Row`/`Cell`, `DocxDocument`, pdf `Document`)
- [ ] `Write`-based streaming writers and `Read + Seek` readers; stop buffering whole xlsx sheets; drop per-segment `Vec` allocations to match the "zero-allocation" claim
- [ ] Runtime-loadable EDI/HL7 schemas (tables are `&'static str` today)

### Step 5 — Adoption & docs
- [x] FIXED: root README lists all 10 crates with changelog links, and the quick-start is now exercised by `crates/tpt-doc-sign/tests/quickstart.rs` (was broken: wrong `build()` arg and a non-existent `tpt_doc_sign::prelude`). Roadmap extension NOT yet done.
- [x] DONE: doc drift fixed — `architecture.md` and `spec.txt` now say 10 crates (`introduction.md` and `sign.md` were already correct; the CHANGELOG 7→10 lines are historical entries and intentionally kept).
- [x] DONE: `examples/` for xlsx round-trip, HL7 ADT parse, HTML→PDF, EDIFACT parse, FHIR Patient→JSON, X12 835 parse, UBL invoice + Factur-X (`--features facturx`), PAdES sign (all run).
- [x] DONE: per-crate `README.md`, `readme`/`documentation` in each `Cargo.toml` (CLI gained `documentation`); every crate has at most 5 keywords.
- [ ] `templates/` gallery with sample data + expected output: NZ-style tax invoice (UBL+PDF), HL7 ADT→FHIR, 835 remittance parser, spreadsheet report, signed PDF contract
- [x] MOSTLY DONE: `tpt-doc` facade crate (`crates/tpt-doc`) with per-format feature flags (`pdf`, `layout`, `sign`, `spreadsheet`, `word`, `fhir`, `hl7v2`, `edi`, `ubl`, `facturx`, `serde`, `full`), each re-exported as a module; `ring`/`x509-parser` are only built with `sign`. Every single-feature build checked. NOT DONE: a type-level unified prelude (names collide across formats, so the prelude is core types only), and making `zip`/`ring` optional *inside* their own crates.
- [x] DONE: community files — `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, issue/PR templates, `CODEOWNERS`, dependabot.
- [x] MOSTLY DONE (CI): MSRV (1.85) and separate doc-test jobs, `rust-toolchain.toml`, `.kilo/` ignored. NOT DONE: `cargo semver-checks`, run-examples job, `.devcontainer`, Dockerfile.
- [x] MOSTLY DONE: tag-triggered publish workflow (`.github/workflows/release.yml`: `v*.*.*` tag → tag/version check → `cargo test --workspace --all-features` → `cargo publish` in dependency order; needs the `CARGO_REGISTRY_TOKEN` secret). NOT YET EXERCISED by a real tag. NOT DONE: release-plz / changelog automation.
- [ ] mdBook: tutorials, recipes, migration + compliance guides (NZISM, HIPC), error-handling guide, longer crate chapters
- [ ] `cargo-fuzz` targets for hl7v2, edi, fhir JSON/XML, xlsx, docx, png/pdf; criterion benches to back performance claims

### Step 6 — New features
- [x] MOSTLY DONE: `tpt` CLI crate (`crates/tpt-doc-cli`, binary `tpt`): `sniff | inspect | validate [--json] | render | text` with content-based format sniffing and exit codes 0/1/2; 6 end-to-end tests over the other crates' fixtures. NOT DONE: `convert`, `diff`, `sign`; revisit `Cargo.lock` gitignore (a binary now exists).
- [x] MOSTLY DONE: HL7v2 → FHIR converter (`crates/tpt-doc-convert`, `hl7v2_to_fhir`, also `tpt convert` and the facade `convert` feature). ADT: PID→Patient, PV1→Encounter (class, status from trigger event, period); ORU: OBX→Observation (code, OBX-11 status, NM→valueQuantity, effective time); returns a collection Bundle plus mapping warnings and FHIR-invariant findings. HL7 timestamps with a time but no UTC offset keep only the date (FHIR requires an offset) and warn. NOT DONE: non-numeric OBX values (the `Observation` model only has `valueQuantity`), PV1 location/practitioner, AL1/DG1/NTE, FHIR→HL7v2.
- [ ] JSON → e-invoice pipeline (Invoice → validate → PDF → Factur-X → PAdES); blocked on B1, B3, B5, B6, B7
- [x] MOSTLY DONE: data-merge templating for HTML→PDF — `tpt_doc_layout::template::merge` / `Renderer::render_template` (`{{field}}` with dotted paths and array indexes, `#if`/`else`, `#each` with `.` and `@index`; always HTML-escaped; a missing field is an error; nesting and output size bounded) and `tpt render --data D.json`. NOT DONE: filling DOCX/XLSX templates from the same JSON, formatting helpers (dates, money), partials.
- [x] MOSTLY DONE: HL7 v2 PHI de-identification — `tpt_doc_hl7v2::redact` (`RedactionProfile::safe_harbor()` plus custom `Rule`s with `Clear`/`Replace`/`Pseudonym`/`YearOnly`; numeric OBX results kept, text results and NTE cleared; per-message stable pseudonyms; audit of positions, never values; unrecognised text is refused rather than passed through) and `tpt redact`. NOT DONE: FHIR JSON redaction by path, cross-message pseudonym consistency (needs a keyed hash), date shifting, a free-text scrubber.
- [x] MOSTLY DONE: FHIR JSON de-identification — `tpt_doc_fhir::redact` (path rules like `Patient.identifier.value`, `*.subject.display`; `Remove`/`Replace`/`Pseudonym`/`YearOnly`/`PatientReference`; works on any resource incl. `contained` and unmodelled types; `Patient/id` references and `fullUrl` follow the same pseudonym so bundles stay linked; narrative/notes/`valueString` removed; audit of paths and counts). `tpt redact` handles both formats. NOT DONE: FHIR XML, cross-document pseudonyms, date shifting, free-text scrubbing, Binary/Attachment payloads.
- [x] MOSTLY DONE: structural diff — `crates/tpt-doc-diff` (`diff_hl7`/`diff_edifact`/`diff_x12`/`diff_json`/`diff_xlsx`/`diff_docx`; paths like `PID[0]-5.1`, `/entry/0/resource/birthDate`, `B3`; ignore prefixes for volatile fields; a field gaining components still lines up), `tpt diff A B [--ignore P] [--json]` (exit 0 identical / 1 differ / 2 error) and the facade `diff` feature. NOT DONE: UBL diff, array alignment by key rather than index (a removed first array element shifts the rest), DOCX run-level and table diffs, multi-sheet XLSX.
- [ ] Audit/provenance hash chain (embedded PDF attachment or FHIR Provenance), signed
- [ ] WASM playground (`wasm-bindgen`; check `ring` on wasm32 for sign)
- [ ] MCP server (`validate_document`, `convert`, `extract_text`, `generate_invoice`); needs serde + structured reports first
- [ ] Format gaps: X12 850/810/856/997/999 + loops, EDIFACT INVOIC/ORDERS/DESADV, EDI writer + 999 ack, HL7 ACK + MLLP + batch, more FHIR resources/R4 + profile validation, DOCX structural read/lists/images/hyperlinks, PDF parser/text extraction/PDF/A/merge-split

# tpt-doc

The facade crate for the tpt-doc workspace. It re-exports the format crates
behind Cargo feature flags, so one dependency gives access to PDF, spreadsheet,
FHIR, EDI, HL7 v2, Word, UBL and signing support.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## Features

| Feature | Re-exports | Notes |
|---|---|---|
| `pdf` | `tpt-doc-pdf` | Writer and text extraction |
| `spreadsheet` | `tpt-doc-spreadsheet` | XLSX and CSV |
| `fhir` | `tpt-doc-fhir` | FHIR R5 resources |
| `edi` | `tpt-doc-edi` | EDIFACT and X12 |
| `hl7v2` | `tpt-doc-hl7v2` | HL7 v2 messages |
| `word` | `tpt-doc-word` | DOCX |
| `ubl` | `tpt-doc-ubl` | UBL 2.1 and PEPPOL invoices |
| `layout` | `tpt-doc-layout` | HTML to PDF |
| `sign` | `tpt-doc-sign` | PAdES, XAdES, CAdES |
| `facturx` | Factur-X attachment support in `tpt-doc-ubl` | Enabled by `ubl` and `pipeline` |
| `convert` | `tpt-doc-convert` | HL7 v2 to FHIR |
| `diff` | `tpt-doc-diff` | Semantic diff |
| `serde` | Serde support in the enabled crates | |
| `pipeline` | `invoice_json_to_signed_pdf` | JSON invoice to signed Factur-X PDF |
| `full` | Everything above | |

## Quick start

Enable the features you need:

```toml
[dependencies]
tpt-doc = { version = "0.1", features = ["pdf", "fhir", "hl7v2"] }
```

The `prelude` module brings the common types into scope:

```rust,ignore
use tpt_doc::prelude::*;
```

To turn an invoice from JSON into a signed Factur-X PDF in one call, enable
`pipeline` and use `invoice_json_to_signed_pdf`. The walkthrough is in
`docs/src/guides/invoice-pipeline.md`.

## When to depend on a format crate directly

Depend on a format crate such as `tpt-doc-fhir` when you only need that format.
It keeps your dependency tree and compile time smaller, and it lets you pin
crates independently.

## Testing

```sh
cargo test -p tpt-doc --all-features
```

The pipeline test builds a signed PDF end to end and checks that the embedded
Factur-X XML and the signature are both present.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

# tpt-doc

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-core.svg)](https://crates.io/crates/tpt-doc-core)
[![Docs.rs](https://docs.rs/tpt-doc-core/badge.svg)](https://docs.rs/tpt-doc-core)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

**The Enterprise Document & Data Substrate** — high-assurance, pure-Rust, spec-compliant document generation and parsing for civic and enterprise platforms.

No shelling out to Python or Java. No C-FFI. No bloated dependency trees. 100% MIT/Apache-2.0 compatible.

## Crates

| Crate | Description | Docs |
|---|---|---|
| [`tpt-doc-core`](crates/tpt-doc-core) | Shared traits, zero-copy buffers, unified errors | [![docs.rs](https://docs.rs/tpt-doc-core/badge.svg)](https://docs.rs/tpt-doc-core) |
| [`tpt-doc-spreadsheet`](crates/tpt-doc-spreadsheet) | Streaming `.xlsx` (OOXML) and `.csv` parsing/generation | [![docs.rs](https://docs.rs/tpt-doc-spreadsheet/badge.svg)](https://docs.rs/tpt-doc-spreadsheet) |
| [`tpt-doc-fhir`](crates/tpt-doc-fhir) | HL7 FHIR R5 typed parsing, validation, and serialization | [![docs.rs](https://docs.rs/tpt-doc-fhir/badge.svg)](https://docs.rs/tpt-doc-fhir) |
| [`tpt-doc-edi`](crates/tpt-doc-edi) | EDIFACT and X12 parsing with schema validation | [![docs.rs](https://docs.rs/tpt-doc-edi/badge.svg)](https://docs.rs/tpt-doc-edi) |
| [`tpt-doc-pdf`](crates/tpt-doc-pdf) | Pure-Rust PDF 1.7/2.0 generation and parsing | [![docs.rs](https://docs.rs/tpt-doc-pdf/badge.svg)](https://docs.rs/tpt-doc-pdf) |
| [`tpt-doc-sign`](crates/tpt-doc-sign) | Cryptographic document signing (PAdES, XAdES, CAdES) | [![docs.rs](https://docs.rs/tpt-doc-sign/badge.svg)](https://docs.rs/tpt-doc-sign) |
| [`tpt-doc-layout`](crates/tpt-doc-layout) | Headless HTML/CSS-to-PDF layout engine | [![docs.rs](https://docs.rs/tpt-doc-layout/badge.svg)](https://docs.rs/tpt-doc-layout) |

## Quick Start

```toml
[dependencies]
tpt-doc-fhir = "0.1"
tpt-doc-sign = "0.1"
```

```rust
use tpt_doc_fhir::prelude::*;
use tpt_doc_sign::prelude::*;

// Build a strictly typed FHIR Patient resource
let patient = Patient::builder()
    .id("nz-12345".into())
    .name(HumanName::new("Smith", "John"))
    .build()?;

// Serialize to compliant JSON
let json_bytes = patient.to_json()?;

// Sign for HIPC 2020 compliance
let signature = tpt_doc_sign::pades::sign(&json_bytes, &private_key)?;
```

## Compliance Targets

- **NZISM** — New Zealand Information Security Manual
- **HL7 FHIR R5** — Fast Healthcare Interoperability Resources
- **HIPC 2020** — Health Information Privacy Code

## Development Roadmap

- **Phase 1 (Months 1–2):** `tpt-doc-core` + `tpt-doc-spreadsheet`
- **Phase 2 (Months 3–4):** `tpt-doc-fhir` + `tpt-doc-edi`
- **Phase 3 (Months 5–6):** `tpt-doc-pdf` + `tpt-doc-sign` + `tpt-doc-layout`

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

Copyright (c) 2026 TPT Solutions

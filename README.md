# tpt-doc

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-core.svg)](https://crates.io/crates/tpt-doc-core)
[![Docs.rs](https://docs.rs/tpt-doc-core/badge.svg)](https://docs.rs/tpt-doc-core)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

**The Enterprise Document & Data Substrate** — high-assurance, pure-Rust, spec-compliant document generation and parsing for civic and enterprise platforms.

No shelling out to Python or Java. No C-FFI. No bloated dependency trees. 100% MIT/Apache-2.0 compatible.

## Crates

| Crate | Description | Docs | Changelog |
|---|---|---|---|
| [`tpt-doc-core`](crates/tpt-doc-core) | Shared traits, zero-copy buffers, unified errors | [![docs.rs](https://docs.rs/tpt-doc-core/badge.svg)](https://docs.rs/tpt-doc-core) | [CHANGELOG](crates/tpt-doc-core/CHANGELOG.md) |
| [`tpt-doc-spreadsheet`](crates/tpt-doc-spreadsheet) | Streaming `.xlsx` (OOXML) and `.csv` parsing/generation | [![docs.rs](https://docs.rs/tpt-doc-spreadsheet/badge.svg)](https://docs.rs/tpt-doc-spreadsheet) | [CHANGELOG](crates/tpt-doc-spreadsheet/CHANGELOG.md) |
| [`tpt-doc-fhir`](crates/tpt-doc-fhir) | HL7 FHIR R5 typed parsing, validation, and serialization | [![docs.rs](https://docs.rs/tpt-doc-fhir/badge.svg)](https://docs.rs/tpt-doc-fhir) | [CHANGELOG](crates/tpt-doc-fhir/CHANGELOG.md) |
| [`tpt-doc-edi`](crates/tpt-doc-edi) | EDIFACT and X12 parsing with schema validation | [![docs.rs](https://docs.rs/tpt-doc-edi/badge.svg)](https://docs.rs/tpt-doc-edi) | [CHANGELOG](crates/tpt-doc-edi/CHANGELOG.md) |
| [`tpt-doc-hl7v2`](crates/tpt-doc-hl7v2) | Streaming HL7 v2.x message parser and encoder | [![docs.rs](https://docs.rs/tpt-doc-hl7v2/badge.svg)](https://docs.rs/tpt-doc-hl7v2) | [CHANGELOG](crates/tpt-doc-hl7v2/CHANGELOG.md) |
| [`tpt-doc-pdf`](crates/tpt-doc-pdf) | Pure-Rust PDF 1.7/2.0 generation and parsing | [![docs.rs](https://docs.rs/tpt-doc-pdf/badge.svg)](https://docs.rs/tpt-doc-pdf) | [CHANGELOG](crates/tpt-doc-pdf/CHANGELOG.md) |
| [`tpt-doc-sign`](crates/tpt-doc-sign) | Cryptographic document signing (PAdES, XAdES, CAdES) | [![docs.rs](https://docs.rs/tpt-doc-sign/badge.svg)](https://docs.rs/tpt-doc-sign) | [CHANGELOG](crates/tpt-doc-sign/CHANGELOG.md) |
| [`tpt-doc-layout`](crates/tpt-doc-layout) | Headless HTML/CSS-to-PDF layout engine | [![docs.rs](https://docs.rs/tpt-doc-layout/badge.svg)](https://docs.rs/tpt-doc-layout) | [CHANGELOG](crates/tpt-doc-layout/CHANGELOG.md) |
| [`tpt-doc-word`](crates/tpt-doc-word) | Pure-Rust `.docx` (OOXML Word) generation and parsing | [![docs.rs](https://docs.rs/tpt-doc-word/badge.svg)](https://docs.rs/tpt-doc-word) | [CHANGELOG](crates/tpt-doc-word/CHANGELOG.md) |
| [`tpt-doc-ubl`](crates/tpt-doc-ubl) | UBL 2.1/2.3 and PEPPOL BIS Billing 3.0 e-invoicing | [![docs.rs](https://docs.rs/tpt-doc-ubl/badge.svg)](https://docs.rs/tpt-doc-ubl) | [CHANGELOG](crates/tpt-doc-ubl/CHANGELOG.md) |

Each crate ships its own README with compiled examples and its own CHANGELOG.

## Quick Start

```toml
[dependencies]
tpt-doc-fhir = "0.1"
tpt-doc-sign = "0.1"
ring = "0.17"          # supplies SecureRandom for signing
```

```rust
use ring::rand::SystemRandom;
use tpt_doc_fhir::prelude::*;
use tpt_doc_sign::{cades, pem, PrivateKey};

// Build a strictly typed FHIR Patient resource. `build()` only exists once
// every mandatory field is set, so an incomplete patient will not compile.
let patient = Patient::builder()
    .id("nz-12345")
    .name(HumanName::new("Smith", "John"))
    .build();

// Serialize to compliant FHIR JSON.
let json_bytes = patient.to_json()?;
assert!(String::from_utf8_lossy(&json_bytes).contains(r#""resourceType": "Patient""#));

// Detached CAdES signature over those bytes, for HIPC 2020 audit trails.
let key = PrivateKey::from_pem(&include_bytes!("key.pem")?)?;
let cert_der = pem::pem_to_der(&include_bytes!("cert.pem")?)?;
let signature =
    cades::sign_detached(&json_bytes, &key, &cert_der, &SystemRandom::new())?;
assert!(!signature.is_empty());

# Ok::<(), Box<dyn std::error::Error>>(())
```

See the per-crate READMEs for more examples; every one of them is compiled and
run as a doctest in CI.

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

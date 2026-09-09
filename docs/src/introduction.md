# tpt-doc

**The Enterprise Document & Data Substrate** — high-assurance, pure-Rust document generation and parsing for civic and enterprise platforms.

## Why tpt-doc?

Enterprise document workflows typically rely on:
- Shelling out to Python/Java libraries
- C-FFI bindings to pdfium, LibreOffice, or similar
- Fragile, heavyweight crates with GPL-adjacent dependencies

tpt-doc eliminates all of that. Every crate is:
- **Pure Rust** — zero C-FFI, zero subprocess spawning
- **MIT OR Apache-2.0** — 100% compatible dependency chain, enforced by `cargo-deny`
- **Spec-compliant** — targets NZISM, HL7 FHIR R5, and HIPC 2020
- **Memory-efficient** — streaming parsers; no loading entire documents into RAM

## Compliance Targets

| Standard | Crate |
|---|---|
| HL7 FHIR R5 | `tpt-doc-fhir` |
| HIPC 2020 | `tpt-doc-sign` |
| NZISM | All crates |
| EDIFACT D.96A | `tpt-doc-edi` |
| X12 (835/837/270/271) | `tpt-doc-edi` |
| PDF 1.7 / 2.0 | `tpt-doc-pdf` |
| PAdES / XAdES / CAdES | `tpt-doc-sign` |

## Getting Started

Add the crates you need to `Cargo.toml`:

```toml
[dependencies]
tpt-doc-fhir = "0.1"
tpt-doc-sign = "0.1"
```

See the individual crate chapters for usage examples.

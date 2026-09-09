# Architecture Overview

## Workspace Structure

The tpt-doc workspace is divided into seven focused crates arranged in a dependency hierarchy:

```
tpt-doc-layout ──┐
tpt-doc-sign   ──┤
tpt-doc-pdf    ──┼── tpt-doc-core
tpt-doc-edi    ──┤
tpt-doc-fhir   ──┤
tpt-doc-spreadsheet ─┘
```

`tpt-doc-core` is the only `no_std` crate and carries zero external dependencies. All other crates depend on it and require `std`.

## Design Principles

### Zero-Copy Parsing

Where possible, parsers operate on borrowed `&[u8]` slices and yield data without heap allocation. The `BufSlice<'a>` type in `tpt-doc-core` encodes this guarantee in the type system.

### Type-State Validation

Rather than panicking on invalid documents at runtime, builders use Rust's type system to enforce mandatory fields at compile time. The FHIR crate pioneered this pattern; it propagates across all builder APIs.

### Deterministic Output

Generated documents (PDF, OOXML, EDI) produce byte-identical output given identical inputs. This enables content-addressable storage, audit trails, and reproducible builds.

### 100% Pure Rust

No C-FFI. No subprocess invocations. The entire pipeline — from bytes in to bytes out — runs inside the Rust process with Rust's safety guarantees.

## Phase Roadmap

| Phase | Months | Crates |
|---|---|---|
| 1 | 1–2 | `tpt-doc-core`, `tpt-doc-spreadsheet` |
| 2 | 3–4 | `tpt-doc-fhir`, `tpt-doc-edi` |
| 3 | 5–6 | `tpt-doc-pdf`, `tpt-doc-sign`, `tpt-doc-layout` |

# tpt-doc-core

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-core.svg)](https://crates.io/crates/tpt-doc-core)
[![Docs.rs](https://docs.rs/tpt-doc-core/badge.svg)](https://docs.rs/tpt-doc-core)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

Shared traits, zero-copy buffer abstractions, and unified error types for the
[`tpt-doc`](https://github.com/tpt-solutions/tpt-doc) ecosystem.

This crate is the foundation every other `tpt-doc-*` crate builds on. It has
**zero external dependencies** and is `no_std` compatible when the default
`std` feature is disabled.

## Highlights

- Zero external dependencies — nothing else is pulled into your build.
- `no_std` support behind a feature flag (`alloc` only).
- `BufSlice<'a>`: an explicit, copyable "this is a view, not a copy" byte type.
- One `DocError` enum for every parser, writer, and validator in the ecosystem.
- `DocReader`, `DocWriter`, and `Validate` traits so your own formats plug in.

## Installation

```toml
[dependencies]
tpt-doc-core = "0.1"
```

To build without the standard library:

```toml
[dependencies]
tpt-doc-core = { version = "0.1", default-features = false }
```

## Usage

### Zero-copy buffer views

```rust
use tpt_doc_core::BufSlice;

let buf = BufSlice::new(b"document bytes");
assert_eq!(buf.as_bytes(), b"document bytes");
let (head, tail) = buf.split_at(8);
assert_eq!(head.as_bytes(), b"document");
assert_eq!(tail.as_bytes(), b" bytes");
```

### Unified error handling

```rust
use tpt_doc_core::DocError;

fn parse(bytes: &[u8]) -> Result<u32, DocError> {
    if bytes.is_empty() {
        return Err(DocError::MissingField("document body"));
    }
    if bytes[0] != b'%' {
        return Err(DocError::invalid_format("expected a header marker"));
    }
    Ok(bytes.len() as u32)
}

assert_eq!(parse(b"%PDF").unwrap(), 4);
assert!(matches!(parse(b""), Err(DocError::MissingField("document body"))));
```

### Implementing your own document format

```rust
use tpt_doc_core::{DocError, DocReader, DocWriter, Validate, ValidationReport};

/// A minimal example document type.
struct Doc(Vec<u8>);

impl DocReader for Doc {
    fn from_bytes(bytes: &[u8]) -> Result<Self, DocError> {
        if bytes.is_empty() {
            return Err(DocError::MissingField("body"));
        }
        Ok(Doc(bytes.to_vec()))
    }
}

impl DocWriter for Doc {
    fn to_bytes(&self) -> Result<Vec<u8>, DocError> {
        Ok(self.0.clone())
    }
}

impl Validate for Doc {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();
        if self.0.is_empty() {
            report.push("/body", "must not be empty");
        }
        report
    }
}

assert!(Doc::from_bytes(b"content").unwrap().validate().is_valid());
assert!(Doc::from_bytes(b"").is_err());
```

## Feature flags

| Feature | Default | Description |
|---|---|---|
| `std` | yes | Enables `std::io` integration and the `DocError::Io` variant. |

## Safety and quality guarantees

- `#![forbid(unsafe_code)]`
- `#![warn(missing_docs, clippy::pedantic)]`
- Property tests via `proptest` in CI.

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-core>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

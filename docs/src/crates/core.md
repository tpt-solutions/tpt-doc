# tpt-doc-core

Shared traits, zero-copy buffer abstractions, and unified error types for the tpt-doc ecosystem.

## Features

- `no_std` compatible (with `alloc`)
- `std` feature (default) enables `std::io` trait implementations
- Zero external dependencies

## Key Types

### `BufSlice<'a>`

A zero-copy wrapper over `&'a [u8]` that signals "this slice is borrowed from the original input — no copies were made."

### `DocError`

The unified error enum used across all tpt-doc crates. `#[non_exhaustive]` so new variants can be added without a semver break.

### Traits

| Trait | Purpose |
|---|---|
| `DocReader` | Implemented by all document parsers |
| `DocWriter` | Implemented by all document generators |
| `Validate` | Runtime validation returning a structured `ValidationReport` |

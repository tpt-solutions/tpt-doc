# Changelog

All notable changes to `tpt-doc-core` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-10

### Added

- `BufSlice<'a>` zero-copy borrowed byte-slice abstraction with `split_at`,
  `len`, `is_empty`, and `as_bytes`.
- Unified, `#[non_exhaustive]` `DocError` enum with `InvalidFormat`,
  `MissingField`, `ValidationFailed`, `Other`, and an `Io` variant behind the
  `std` feature. `Display`, `Error::source`, and `From<std::io::Error>` are
  implemented.
- `DocReader` and `DocWriter` traits for implementing custom document formats.
- `Validate` trait plus `ValidationReport` and `ValidationIssue` for
  structured, field-path-anchored validation reporting.
- `no_std` support through the `std` feature flag (enabled by default).
- `#![forbid(unsafe_code)]` and `#![warn(missing_docs, clippy::pedantic)]`.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-core-v0.1.0

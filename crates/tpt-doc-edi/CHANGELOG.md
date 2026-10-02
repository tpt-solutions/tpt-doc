# Changelog

All notable changes to `tpt-doc-edi` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-10

### Added

- `edifact` module: `EdifactParser` streaming iterator over UN/EDIFACT D.96A
  messages, yielding borrowed `Segment<'a>` values.
- `x12` module: `X12Parser` for ANSI X12 interchanges, yielding `X12Segment<'a>`.
- `schema` module with embedded, TSV-compiled tables:
  - `edifact_d96a()` and `x12_elements()` returning `ElementTables`.
  - `x12_transaction_sets()` returning `TransactionTables`.
  - `SchemaViolation`, `SegmentRequirement`, and `ElemKind` types.
  - `check_edifact` / `check_x12` / `validate_edifact` / `TransactionTables::validate`
    entry points.
- Zero-allocation-per-segment parsing over borrowed `&[u8]` input.
- `proptest` property tests for delimiter handling and segment splitting.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-edi-v0.1.0

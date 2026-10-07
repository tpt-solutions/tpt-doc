# Changelog

All notable changes to `tpt-doc-edi` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `SchemaViolation::to_report` and `From<&SchemaViolation> for ValidationIssue`,
  converting the format-specific violations into the shared
  `tpt_doc_core::ValidationReport`. The segment index becomes a
  `Location::Index`, so a caller can find the offending line in the stream.
  The existing `Vec<SchemaViolation>` API is unchanged.
- `validate_x12_envelope` implementing the X12 envelope checks the docs
  promised but never shipped: `ST`/`SE`, `GS`/`GE`, and `ISA`/`IEA`
  bracketing, plus `SE01` (segment count from ST through SE), `GE01`
  (transaction sets in the group), `IEA01` (functional groups), and the
  control-number agreements `SE02`=`ST02`, `GE02`=`GS06`, `IEA02`=`ISA13`.

### Fixed

- An element position of `0` in a schema table no longer underflows
  `elements.get(position - 1)`. Element positions are 1-based, so a malformed
  table row panicked in debug builds and silently skipped the check in
  release. It is now reported as a malformed-table violation.
- Every X12 envelope violation is reported at the offending segment's own
  index. An unclosed `ST` or `GS` was previously reported at index 0, which
  pointed a reader at the wrong line.
- An interchange that omits `ISA` at the start or `IEA` at the end is now
  reported instead of validating clean.

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
- `X12Parser::new` validates the ISA envelope: ISA01 (element separator),
  ISA11 (repetition separator), ISA16 (component separator), and the segment
  terminator must all be printable and mutually distinct where they overlap,
  since every segment split depends on them. `repetition_separator()` exposes
  ISA11.
- `X12Parser` reports a truncated trailing segment instead of silently
  discarding everything after the last terminator.
- `EdifactParser` reports a truncated trailing segment instead of silently
  discarding everything after the last terminator, and rejects a segment
  carrying more than 128 data elements rather than folding the overflow into
  the last one.
- `Segment::elements_released` resolves EDIFACT release sequences (`?`), so
  `A?+B` reads as the single element `A+B`. `Segment::elements` still returns
  the raw borrowed slices, so this is additive rather than a breaking change,
  and elements without a release character are borrowed without allocating.
- `proptest` property tests for delimiter handling and segment splitting.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-edi-v0.1.0

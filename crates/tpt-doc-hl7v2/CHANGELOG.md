# Changelog

All notable changes to `tpt-doc-hl7v2` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-10

### Added

- `parser` module: `Hl7Parser` streaming iterator with `new()` (delimiters
  inferred from MSH-2) and `with_delimiters()` constructors.
- `segment` module: borrowed `Segment<'a>` with `field(n)`, `component(n, m)`,
  and `repeat(n, r)` accessors, plus the `Delimiters` set
  (field, component, repeat, escape, subcomponent).
- `encode` module: `Message` with `msh()` / `msh_with_version()` constructors,
  `push_segment()`, `segments()`, and `to_bytes()`; `MessageSegment`.
- `schema` module: TSV-compiled `SegmentTables`, `FieldDef`, `Hl7Violation`,
  the embedded `v251()` (HL7 v2.5.1) tables, `SegmentTables::definition()`,
  `SegmentTables::validate_segment()`, and `validate_message()`.
- `prelude` module for single-line imports.
- `proptest` property tests for delimiter edge cases.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-hl7v2-v0.1.0

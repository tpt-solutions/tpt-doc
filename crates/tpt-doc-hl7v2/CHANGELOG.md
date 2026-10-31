# Changelog

All notable changes to `tpt-doc-hl7v2` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `redact` module: HIPAA Safe Harbor de-identification of HL7 v2 messages (`redact`, `RedactionProfile`, `Rule`, `Action`), with a per-field audit trail that never records values.

- `Hl7Violation::to_report` and `From<&Hl7Violation> for ValidationIssue`,
  converting the format-specific violations into the shared
  `tpt_doc_core::ValidationReport`. The segment index becomes a
  `Location::Index`, so a caller can find the offending line in the message.
  The existing `Vec<Hl7Violation>` API is unchanged.

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
- `Message::to_bytes` escapes field values: the field separator and escape
  character become `\F\` and `\E\`, and CR/LF become `\X0D\`/`\X0A\`, so a
  delimiter inside free text can no longer corrupt the message structure.
  Component, repetition, and subcomponent separators are left intact because a
  field value is expected to carry them.
- `prelude` module for single-line imports.
- `proptest` property tests for delimiter edge cases.
- The parser accepts CR, CRLF, and bare LF segment terminators, so messages
  written with common Windows or Unix line endings parse identically instead of
  collapsing into a single segment.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-hl7v2-v0.1.0

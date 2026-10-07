# Changelog

All notable changes to `tpt-doc-word` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `DocxReader::extract_text_with_limits` applying `tpt_doc_core::Limits`: the
  decompressed `word/document.xml` part is size-bounded, and nesting is refused
  past `max_xml_depth` rather than walked.

## [0.1.0] - 2026-02-10

### Added

- `document` module with the in-memory `DocxDocument` model, `BodyElement`
  paragraphs/tables, header and footer support, `DocxWriter::write` ZIP
  assembly, and `DocxReader::extract_text` parsing.
- `paragraph` module: `Paragraph` with multiple `Run`s, `RunStyle`
  (bold/italic/underline), and `plain_text()`.
- `styles` module: `BuiltinStyle` (Heading1–Heading6, Normal, Title, etc.) and
  `StyleSet` with `mark_used()` for emitting only the styles a document needs.
- `table` module: `Table`, `Row`, `Cell`, and `BorderStyle`.
- `prelude` module for single-line imports.
- Text is validated against the XML 1.0 `Char` production. A C0 control in a run,
  style id, table cell, or header/footer is reported with its code point instead
  of producing a `.docx` Word cannot open.
- Deterministic XML output, verified with `insta` snapshots and `proptest`
  round-trip tests.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-word-v0.1.0

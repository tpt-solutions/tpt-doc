# Changelog

All notable changes to `tpt-doc-word` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-10

### Added

- `DocxReader::extract_text_with_limits` applying `tpt_doc_core::Limits`: the
  decompressed `word/document.xml` part is size-bounded, and nesting is refused
  past `max_xml_depth` rather than walked.

- `Paragraph::list_item` with `ListKind`: bulleted and decimal lists. The writer adds `word/numbering.xml` only when a document has list items. The reader maps each paragraph's `numId` through the numbering part, so lists from other producers keep their kind. Lists are read in body paragraphs and table cells.
- `Run::linked`: external hyperlinks. The writer wraps the run in `w:hyperlink` with one external relationship per distinct address. Two addresses that would collide are refused. Read back in body paragraphs and table cells.

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

### Fixed

- A table written without column widths gave its grid a 2340-twip default but left the cells auto-sized, so a read-back table wrote different cells. Cells now carry the width of their grid column.

[0.1.0]: https://crates.io/crates/tpt-doc-word/0.1.0

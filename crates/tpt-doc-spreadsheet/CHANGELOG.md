# Changelog

All notable changes to `tpt-doc-spreadsheet` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-10

### Added

- `csv` module with `CsvReader::records()` streaming iterator and
  `CsvWriter::write_record()` / `flush()`.
- `xlsx` module with `XlsxWriter` for generating OOXML workbooks and
  `XlsxReader::rows()` for streaming them back.
- Typed `Cell` enum (`String`, `Number`, `Bool`, `Null`) and `Row` with an
  explicit row index and `cells()` accessor.
- Column-name helpers (`A`, `B`, ... `AA`) for spreadsheet addressing.
- Round-trip determinism tests plus `proptest` property tests.

### Notes

- Documents are streamed entry-by-entry; peak memory is proportional to a
  single row.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-spreadsheet-v0.1.0

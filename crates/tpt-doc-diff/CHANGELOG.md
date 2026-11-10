# Changelog

All notable changes to `tpt-doc-diff` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-11-10

### Added

- Semantic diff functions for HL7 v2, EDIFACT, X12, JSON, XLSX and DOCX, each
  taking an `Options` value with `ignoring(prefix)` to skip paths.
- `Diff::is_empty` to test whether two documents match.

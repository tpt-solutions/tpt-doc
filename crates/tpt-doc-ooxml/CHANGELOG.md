# Changelog

All notable changes to `tpt-doc-ooxml` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-11-10

### Added

- `PackageWriter` for deterministic ZIP packages: fixed entry order and the DOS
  epoch timestamp, so identical parts give identical bytes.
- `open_archive`, `read_part` (bounded by `Limits::max_entry_bytes`) and the
  `XML_DECL` constant.

# Changelog

All notable changes to `tpt-doc` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `pipeline` feature: `invoice_json_to_signed_pdf`, which turns a JSON invoice
  into a PDF with an embedded Factur-X XML and a PAdES signature.
- `prelude` module with the common types from the enabled crates.
- `full` feature enabling every format.

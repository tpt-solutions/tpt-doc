# Changelog

All notable changes to `tpt-doc-templates` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Five worked templates: HL7 ADT to FHIR, X12 835 remittance, UBL invoice with
  Factur-X, spreadsheet report and PAdES-signed contract.
- Golden tests in `tests/gallery.rs`, regenerated with `TPT_UPDATE_GOLDEN=1`.
- `tests/signed_extract.rs`, which checks that a signed template's text can be
  extracted.

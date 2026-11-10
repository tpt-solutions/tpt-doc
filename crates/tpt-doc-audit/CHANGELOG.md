# Changelog

All notable changes to `tpt-doc-audit` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `AuditLog` with a SHA-256 hash chain: `append`, `head`, `verify`.
- JSON Lines storage with `to_jsonl` and `from_jsonl`.
- CAdES seal of the head hash with `seal`, and `verify_seal`.
- `attach_to_pdf`, which embeds the log as `audit-log.jsonl`.
- `provenance`, which builds a FHIR `Provenance` resource for an entry.
- `GENESIS` constant for the first hash in every chain.

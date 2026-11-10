# Changelog

All notable changes to `tpt-doc-convert` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-11-10

### Added

- `hl7v2_to_fhir`: converts an HL7 v2 ADT message to a FHIR `Patient` and
  `Encounter`, and an ORU message to `Observation` resources, with mapping
  warnings returned alongside the bundle.

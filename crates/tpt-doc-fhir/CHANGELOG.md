# Changelog

All notable changes to `tpt-doc-fhir` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Patient::from_xml_with_limits` and `Observation::from_xml_with_limits`
  applying `tpt_doc_core::Limits`: nesting is refused past `max_xml_depth`.

## [0.1.0] - 2026-02-10

### Added

- `Patient` with a type-state `PatientBuilder` (`NoId` -> `HasId`) and `Gender`.
- `Organization` with a type-state `OrganizationBuilder` (`NoName` -> `HasName`).
- `Encounter` and `Observation` with type-state builders and status enums.
- `Bundle` and `Resource` for heterogeneous resource collections.
- Shared FHIR R5 datatypes: `HumanName`, `Identifier`, `Coding`,
  `CodeableConcept`, `Reference`, `Period`, `Quantity`.
- JSON and XML serialization (`to_json` / `from_json` / `to_xml` / `from_xml`)
  for every resource.
- `validate_observation` and `validate_encounter` returning structured
  `ValidationReport` values with field paths.
- `FhirValidationError` for parse/serialization failures.
- `From<DocError> for FhirValidationError`.
- XML serialization validates against the XML 1.0 `Char` production and reports
  the offending code point; JSON serialization is unaffected, since JSON can
  represent those characters as `\uXXXX` escapes.
- `prelude` module for single-line imports.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-fhir-v0.1.0

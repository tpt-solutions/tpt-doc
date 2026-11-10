# Changelog

All notable changes to `tpt-doc-fhir` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-10

### Added

- `redact` module: Safe Harbor de-identification of FHIR JSON by element path (`redact`, `RedactionProfile`, `Rule`, `Action`), keeping `Patient/id` references and `fullUrl` consistent with the patient pseudonym.

- `FhirValidationError::to_report` and `From<&FhirValidationError> for
  ValidationIssue`, converting the format-specific errors into the shared
  `tpt_doc_core::ValidationReport`. The JSON-pointer field path becomes a
  `Location::Path`. The existing `Result<_, FhirValidationError>` API is
  unchanged.


- `Patient::from_xml_with_limits` and `Observation::from_xml_with_limits`
  applying `tpt_doc_core::Limits`: nesting is refused past `max_xml_depth`.

- `Patient`, `Observation`, `Encounter` and `Organization` keep the JSON elements the model does not represent, so a JSON round trip loses nothing.
- `birthDate`, `effectiveDateTime` and encounter period bounds are checked against the FHIR date grammar in both the JSON and XML readers. Impossible dates such as `1970-02-30` are refused.

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

### Changed

- Validation findings now carry the `FHIR R5` invariant they enforce, so a
  caller can map a result back to the specification without parsing the
  message text: `obs-7` (a final/amended/corrected observation needs a value or
  an effective time), `enc-1` (a finished encounter needs a period end), and
  `enc-3` (the period end must not precede the start).

- `Quantity` keeps the decimal text it was read from, so `5.40` is written back as `5.40`. Equality compares the text only when both sides carry it, so a quantity built in code still equals its read-back.

[0.1.0]: https://crates.io/crates/tpt-doc-fhir/0.1.0

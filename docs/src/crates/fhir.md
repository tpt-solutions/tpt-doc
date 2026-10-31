# tpt-doc-fhir

HL7 FHIR R5 typed parsing, validation, and serialization with compile-time resource validation.

## Design

Resources are built via type-state builders. The `build()` method is only callable once all mandatory fields have been provided — the compiler enforces this, eliminating a class of runtime panics.

## Usage

```rust
use tpt_doc_fhir::prelude::*;

let patient = Patient::builder()
    .id("nz-12345".into())
    .name(HumanName::new("Smith", "John"))
    .build()?;

let json = patient.to_json()?;
let xml  = patient.to_xml()?;
```

## Limits

`Patient::from_xml_with_limits` and `Observation::from_xml_with_limits` take
explicit `tpt_doc_core::Limits`: nesting is refused past `max_xml_depth`. A
resource is a handful of elements deep, so deeper nesting is corrupt or hostile.

## Supported Resources (Phase 2)

- `Patient`
- (more to follow)

## Common Types

- `HumanName` — `family` + `given` names
- `Identifier` — system + value pair
- `CodeableConcept` — code + display text
- `FhirValidationError` — structured error with field path reporting

## De-identification

`redact::redact(json, &RedactionProfile::safe_harbor())` works on the JSON
itself, so extensions, `contained` resources and resource types this crate has
no model for are still covered. Rules are element paths — `Patient.name`,
`Patient.identifier.value`, `*.subject.display` (`*` is any resource type;
arrays are traversed automatically) — each with an action: `Remove`,
`Replace`, `Pseudonym`, `YearOnly` (`1970-01-01` → `1970`, a valid FHIR partial
date) or `PatientReference`.

The Safe Harbor profile pseudonymises `Patient.id` (`PATIENT-1`, numbered by
first appearance) and rewrites every `Patient/<id>` reference and bundle
`fullUrl` to the same value, so the bundle still hangs together and distinct
patients stay distinct. Names, contact details, addresses, photos, contacts,
narrative `text`, `note` and `Observation.valueString` are removed;
coding `display`/`text` (clinical terms) are not. The audit
(`Redacted::changes`) lists resource type, rule path and count — never values.
It is a starting point, not a guarantee of anonymity. CLI: `tpt redact bundle.json`.

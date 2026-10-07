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

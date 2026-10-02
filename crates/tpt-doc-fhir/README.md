# tpt-doc-fhir

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-fhir.svg)](https://crates.io/crates/tpt-doc-fhir)
[![Docs.rs](https://docs.rs/tpt-doc-fhir/badge.svg)](https://docs.rs/tpt-doc-fhir)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

HL7 FHIR R5 typed parsing, validation, and serialization with **compile-time**
resource validation.

Resources are built through type-state builders: `build()` simply does not
exist until every mandatory field has been supplied, so an incomplete
`Patient` is a compile error rather than a runtime surprise.

## Highlights

- Type-state builders for `Patient`, `Organization`, `Encounter`, `Observation`.
- JSON and XML round-tripping for every supported resource.
- Structured, field-path-anchored `ValidationReport` output for runtime
  cross-field rules (date ranges, required-if constraints).
- Typed `Bundle` with heterogeneous `Resource` entries.
- Alignment targets: **HL7 FHIR R5**, NZ **HIPC 2020**, **NZISM**.

## Installation

```toml
[dependencies]
tpt-doc-fhir = "0.1"
```

## Usage

### Build a Patient

```rust
use tpt_doc_fhir::prelude::*;

// `build()` is only available once the mandatory fields are set.
let patient = Patient::builder()
    .id("nz-12345")
    .name(HumanName::new("Smith", "John"))
    .build();

let bytes = patient.to_json()?;
let json = String::from_utf8_lossy(&bytes);
assert!(json.contains(r#""resourceType": "Patient""#));
# Ok::<(), FhirValidationError>(())
```

### Attach optional demographics

```rust
use tpt_doc_fhir::prelude::*;

let patient = Patient::builder()
    .id("nz-12345")
    .name(HumanName::new("Smith", "John"))
    .gender(Gender::Female)
    .birth_date("1984-03-02")
    .identifier(Identifier {
        system: Some("https://nzihi.systemsitnz-services.govt.nz".into()),
        value: "123456789".into(),
    })
    .build();
# Ok::<(), FhirValidationError>(())
```

### Validate cross-field rules

```rust
use tpt_doc_fhir::prelude::*;

let observation = Observation::builder()
    .status(ObservationStatus::Final)
    .code(CodeableConcept::text("Body temperature"))
    .value_quantity(Quantity::new(37.2, "C"))
    .build();

let report = validate_observation(&observation);
assert!(report.is_valid(), "unexpected issues: {:?}", report.issues());
```

## Modules

| Module | Contents |
|---|---|
| `patient` | `Patient`, `PatientBuilder`, `Gender` |
| `organization` | `Organization`, `OrganizationBuilder` |
| `encounter` | `Encounter`, `EncounterBuilder`, `EncounterStatus` |
| `observation` | `Observation`, `ObservationBuilder`, `ObservationStatus` |
| `bundle` | `Bundle`, `Resource` |
| `types` | `HumanName`, `Identifier`, `Coding`, `CodeableConcept`, `Reference`, `Period`, `Quantity` |
| `validate` | `FhirValidationError`, `validate_observation`, `validate_encounter` |

## Validation strategy

| Rule class | When it is enforced |
|---|---|
| Mandatory elements present | Compile time (type-state builder) |
| Element value types | Compile time (typed fields) |
| Cross-field / date-range rules | Runtime (`validate_*`) |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-fhir>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

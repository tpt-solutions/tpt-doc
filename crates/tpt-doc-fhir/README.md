# tpt-doc-fhir

FHIR R5 resources in Rust, with type-state builders, JSON and XML
serialization, validation with FHIR invariant identifiers, and
de-identification. A resource read from JSON keeps the elements the model does
not represent, so a round trip loses nothing.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## What it does

- **Resources:** `Patient`, `Observation`, `Encounter`, `Organization` and
  `Bundle`, with their datatypes (`HumanName`, `Identifier`, `CodeableConcept`,
  `Coding`, `Reference`, `Period`, `Quantity`).
- **Builders** enforce mandatory fields at compile time. `Patient` needs an id,
  and `Observation` needs a status and a code, in either order.
- **JSON** round trips for every resource, with `resourceType` checked on read.
- **XML** for `Patient`, `Observation`, `Encounter` and `Organization`. Readers
  follow element positions, so nested elements with the same names are ignored.
- **Validation** returns a `ValidationReport` with FHIR invariant ids (for
  example `obs-7`, `enc-1`). Date fields are checked against the FHIR `date` and
  `dateTime` grammar.
- **Decimals as written:** `Quantity` keeps the decimal text it was read from,
  so `5.40` is not silently changed to `5.4`.
- **De-identification** by element path (`fhir::redact`), with a Safe Harbor
  style profile and an audit of what changed.

## Quick start

```rust,ignore
use tpt_doc_fhir::prelude::*;

let patient = Patient::builder()
    .id("p1")
    .birth_date("1970-02-01")
    .build();

let json = patient.to_json()?;
let back = Patient::from_json(&json)?;
assert_eq!(patient, back);
```

Reading a JSON document with an unknown element keeps that element:

```rust,ignore
let text = br#"{"resourceType":"Patient","id":"p1","meta":{"versionId":"3"}}"#;
let patient = Patient::from_json(text)?;
let out = patient.to_json()?; // still contains "meta"
```

## Main types

| Item | Purpose |
|---|---|
| `Patient`, `Observation`, `Encounter`, `Organization`, `Bundle` | The resources |
| `Patient::builder()` and the other builders | Type-state construction |
| `types::{HumanName, Identifier, CodeableConcept, Coding, Reference, Period, Quantity}` | Datatypes |
| `validate::{validate_observation, validate_encounter, FhirValidationError}` | Validation |
| `redact` | De-identification by path |
| `traits` | `DocReader` and `DocWriter` implementations (JSON) and `Validate` |

## Format notes

- The canonical format for the shared traits is JSON. XML is available through
  each type's `to_xml` and `from_xml`.
- `Observation` values are numeric quantities (`valueQuantity`). Non-numeric
  values are not modelled yet.
- XML readers ignore elements the model does not represent. Those elements are
  not kept on an XML round trip; JSON keeps them.

## Limits and safety

XML nesting is bounded by `Limits::max_xml_depth`. Readers refuse unknown
resource types and malformed dates, and they never panic on arbitrary bytes.

## Testing

```sh
cargo test -p tpt-doc-fhir
```

The tests include property-based round trips (with valid generated dates), XML
position tests, and checks of the date grammar.

## Scope

This crate targets FHIR R5. R4 and profile validation are not yet supported.

## Related crates

- `tpt-doc-convert` maps HL7 v2 messages to these resources.
- `tpt-doc-core` provides errors, validation reports and limits.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

# tpt-doc-convert

Converts HL7 v2 messages into FHIR R5 resources. It uses the `tpt-doc-hl7v2`
parser to read the message and the `tpt-doc-fhir` types to build the output.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## What it does

- An `ADT` message produces a FHIR `Patient` from PID and an `Encounter` from
  PV1. An ADT message without PV1 produces the Patient alone, with a warning.
- An `ORU` message produces an `Observation` for each OBX segment.
- Mapping problems are returned as warnings alongside the bundle, so nothing is
  dropped without a record.

## Quick start

```rust,ignore
use tpt_doc_convert::hl7v2_to_fhir;

let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|20261001120000||ADT^A01|MSG1|P|2.5.1\rPID|1||12345^^^NZHPI^MR||Smith^John||19700101|M\rPV1|1|I|WARD4^101^A";
let conversion = hl7v2_to_fhir(message)?;
assert_eq!(conversion.bundle.entries.len(), 2); // Patient + Encounter
```

The command-line tool gives the same result:

```sh
tpt convert message.hl7 -o bundle.json
```

## Scope and known gaps

- Only the ADT and ORU message families are mapped. Other message types return
  an error that names the type.
- The mapping is a starting point, not a certified interface. Check the output
  against your own site's HL7 and FHIR profile before use.

## Testing

```sh
cargo test -p tpt-doc-convert
```

The golden template `templates/01-hl7-adt-to-fhir` in `tpt-doc-templates`
exercises this crate end to end.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

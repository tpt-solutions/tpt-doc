# tpt-doc-hl7v2

Streaming HL7 v2.x (pipe-delimited) message parser and encoder.

## Design

The parser yields `Segment<'_>` items borrowed from the input `&[u8]` — the same zero-allocation-per-segment pattern as `tpt-doc-edi`. Encoding characters are read from MSH-2, so messages using non-default delimiters parse transparently. Standard MSH numbering is preserved: `field(1)` is the field separator itself, `field(2)` holds the encoding characters.

## Usage

```rust
use tpt_doc_hl7v2::prelude::*;

let bytes = b"MSH|^~\\&|HIS|ACME|RIS|ACME|20261001120000||ADT^A01|MSG00001|P|2.5.1\rPID|1||12345^^^NZHPI^MR||Smith^John";

for segment in Hl7Parser::new(bytes) {
    let seg = segment?;
    if seg.tag() == "PID" {
        println!("patient id: {}", seg.component(3, 1).unwrap_or(""));
    }
}
```

Encoding uses a `Message` builder that constructs the `MSH` header and serializes to `\r`-delimited bytes.

## Supported Segments

Embedded segment definitions (v2.5.1) are planned for: `MSH`, `PID`, `PV1`, `OBR`, `OBX`, `NTE`, `EVN`, `AL1`, `DG1`.

## De-identification

`redact::redact(message, &RedactionProfile::safe_harbor())` removes the direct
identifiers HIPAA Safe Harbor lists that HL7 v2 commonly carries — names,
addresses, phone numbers, identifiers, next of kin, staff — and cuts dates to
the year. `PID-3` and `PV1-19` become stable pseudonyms (`PATIENT-1`,
`VISIT-1`), so two repeats that were different stay different and equal
values stay equal within the message. Numeric `OBX-5` results are kept (they
are the clinical payload); text results and `NTE-3` are cleared because free
text can hold anything.

```rust
use tpt_doc_hl7v2::redact::{redact, RedactionProfile};

let out = redact(message, &RedactionProfile::safe_harbor())?;
std::fs::write("deidentified.hl7", &out.bytes)?;
for change in &out.changes {
    // segment, field and component only — original values are never recorded
    println!("{}-{}", change.segment, change.field);
}
```

Custom profiles are lists of `Rule::field` / `Rule::component` with an
`Action`. Input that does not start with `MSH` is rejected, not passed
through, because unrecognised text would otherwise leak unredacted. The
profile is a starting point — review it against your own data flows; it is
not a guarantee of anonymity. From the shell: `tpt redact message.hl7 -o out.hl7`.

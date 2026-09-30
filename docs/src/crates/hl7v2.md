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

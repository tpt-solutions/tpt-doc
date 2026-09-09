# tpt-doc-edi

EDIFACT and X12 parsing with schema validation and zero-allocation segment streaming.

## Design

The parser yields `Segment<'_>` items borrowed from the input `&[u8]`, allocating no heap memory per segment. Schema tables for EDIFACT D.96A and X12 transaction sets are embedded at compile time via `include_str!`.

## Usage

```rust
use tpt_doc_edi::edifact::EdifactParser;

let msg = b"UNA:+.? 'UNB+...";
let parser = EdifactParser::new(msg);
for segment in parser {
    let seg = segment?;
    println!("{}: {:?}", seg.tag(), seg.elements());
}
```

## Supported Formats

| Format | Transaction Sets |
|---|---|
| EDIFACT | UN/EDIFACT D.96A (envelopes + generic segment streaming) |
| X12 | 835 (Payment), 837 (Claim), 270/271 (Eligibility) |

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

## Envelope Validation

`validate_edifact` checks `UNB`/`UNZ` and `UNH`/`UNT` pairing, the `UNT`
segment count, the `UNZ` message count, and that `UNT02` repeats `UNH01`.

`validate_x12_envelope` does the X12 equivalent:

| Check | Rule |
|---|---|
| Interchange | opens with `ISA`, closes with `IEA` |
| `IEA01` | number of functional groups |
| `IEA02` | repeats `ISA13` |
| `GS`/`GE` | bracketed pair; `GE` may not close a group it did not open |
| `GE01` | transaction sets in the group |
| `GE02` | repeats `GS06` |
| `ST`/`SE` | bracketed pair |
| `SE01` | segments from `ST` through `SE` inclusive |
| `SE02` | repeats `ST02` |

Every violation carries the offending segment's own index, so a reader can be
pointed at the right line rather than at the top of the file.

```rust,ignore
use tpt_doc_edi::schema::validate_x12_envelope;
use tpt_doc_edi::x12::X12Parser;

let segments: Vec<_> = X12Parser::new(&bytes)?
    .collect::<Result<_, _>>()?;
for violation in validate_x12_envelope(&segments) {
    eprintln!("{violation}");
}
```

## Supported Formats

| Format | Transaction Sets |
|---|---|
| EDIFACT | UN/EDIFACT D.96A (envelopes + generic segment streaming) |
| X12 | 835 (Payment), 837 (Claim), 270/271 (Eligibility) |

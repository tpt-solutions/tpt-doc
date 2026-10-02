# tpt-doc-edi

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-edi.svg)](https://crates.io/crates/tpt-doc-edi)
[![Docs.rs](https://docs.rs/tpt-doc-edi/badge.svg)](https://docs.rs/tpt-doc-edi)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

EDIFACT and X12 parsing with embedded schema validation and **zero-allocation**
segment streaming.

The parser operates on a borrowed `&[u8]` and yields `Segment` values that
reference the original input — nothing is copied or heap-allocated per segment.

## Highlights

- Streaming `Iterator` parsers for UN/EDIFACT D.96A and ANSI X12.
- Zero allocation per segment: segments borrow from the input buffer.
- Embedded schema tables — `edifact_d96a()`, `x12_elements()`,
  `x12_transaction_sets()` — parsed from compact TSV.
- Schema-driven validation returning `Vec<SchemaViolation>` with segment
  positions and element references.
- Small dependency footprint: `tpt-doc-core` and `memchr` only.

## Installation

```toml
[dependencies]
tpt-doc-edi = "0.1"
```

## Usage

### Streaming EDIFACT

```rust
use tpt_doc_edi::edifact::EdifactParser;

let message = b"UNB+UNOA:1+SENDER+RECEIVER+260101:0900+1'";
for segment in EdifactParser::new(message) {
    let segment = segment?;
    assert_eq!(segment.tag(), "UNB");
    assert_eq!(segment.elements()[0], "UNOA:1");
}
# Ok::<(), tpt_doc_core::DocError>(())
```

### Streaming X12

```rust
use tpt_doc_edi::x12::X12Parser;

// An ISA segment is always exactly 106 bytes: the element separator is byte 3
// and the segment terminator is byte 105.
let interchange = concat!(
    "ISA*00*          *00*          *ZZ*SENDERID       *ZZ*RECEIVERID     ",
    "*260101*0900*^*00501*000000001*0*P*:~",
    "GS*HP*SENDER*RECEIVER*20260101*0900*1*X*005010X221A1~",
    "ST*835*0001~",
    "BPR*I*1000*C*ACH*CCP*01*021000021*DA*12345678**01*999888777*20260101~",
    "SE*6*0001~",
    "GE*1*1~",
    "IEA*1*000000001~",
);
assert_eq!(interchange.find('~'), Some(105), "ISA must be 106 bytes");

let tags: Vec<String> = X12Parser::new(interchange.as_bytes())?
    .map(|segment| segment.map(|s| s.tag().to_string()))
    .collect::<Result<_, _>>()?;
assert_eq!(tags, vec!["ISA", "GS", "ST", "BPR", "SE", "GE", "IEA"]);
# Ok::<(), tpt_doc_core::DocError>(())
```

### Schema validation

```rust
use tpt_doc_edi::schema::{check_edifact, edifact_d96a, validate_edifact, ElementTables};

let message = b"UNB+UNOA:1+SENDER+RECEIVER+260101:0900+1'";
let segments: Vec<_> = tpt_doc_edi::EdifactParser::new(message).collect::<Result<_, _>>()?;
assert!(check_edifact(&segments).is_ok());

let tables: ElementTables = edifact_d96a();
for violation in validate_edifact(&tables, &segments) {
    println!("{violation:?}");
}
# Ok::<(), tpt_doc_core::DocError>(())
```

### `UNA` service string advice

Delimiters are read automatically from a leading `UNA` segment, so messages
using non-default separators parse transparently:

```rust
use tpt_doc_edi::EdifactParser;

let message = b"UNA:+.? 'UNB+UNOA:1+SENDER'";
let segments: Vec<_> = EdifactParser::new(message).collect::<Result<_, _>>()?;
assert_eq!(segments[0].tag(), "UNB");
# Ok::<(), tpt_doc_core::DocError>(())
```

## Modules

| Module | Contents |
|---|---|
| `edifact` | `EdifactParser`, `Segment` — UN/EDIFACT D.96A streaming. |
| `x12` | `X12Parser`, `X12Segment` — ANSI X12 streaming. |
| `schema` | `ElementTables`, `TransactionTables`, `SchemaViolation`, `SegmentRequirement`, `ElemKind`. |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-edi>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

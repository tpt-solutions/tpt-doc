# tpt-doc-hl7v2

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-hl7v2.svg)](https://crates.io/crates/tpt-doc-hl7v2)
[![Docs.rs](https://docs.rs/tpt-doc-hl7v2/badge.svg)](https://docs.rs/tpt-doc-hl7v2)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

Streaming HL7 v2.x (pipe-delimited) message parser and encoder — **zero
allocation per segment**.

The parser operates on a borrowed `&[u8]` and yields `Segment` values
referencing the original input — the same zero-allocation-per-segment pattern
as [`tpt-doc-edi`](https://crates.io/crates/tpt-doc-edi).

## Highlights

- Streaming `Iterator` over segments of an HL7 v2.x message.
- Encoding characters are read from **MSH-2**, so messages using non-default
  delimiters parse transparently.
- Three-level field addressing: `field(n)`, `component(n, m)`, and
  `repeat(n, r)`.
- Embedded v2.5.1 segment tables with schema validation returning
  `Vec<Hl7Violation>`.
- Message builder for constructing outbound messages (`Message`, `MessageSegment`).
- Small dependency footprint: `tpt-doc-core` and `memchr` only.

## Installation

```toml
[dependencies]
tpt-doc-hl7v2 = "0.1"
```

## Usage

### Parse a message

```rust
use tpt_doc_hl7v2::prelude::*;

let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|MSG1|P|2.5.1\rPID|1||12345";
let segments: Vec<_> = Hl7Parser::new(message).collect::<Result<_, _>>()?;

assert_eq!(segments[0].tag(), "MSH");
assert_eq!(segments[0].component(9, 1), Some("ADT"));
assert_eq!(segments[1].field(3), Some("12345"));
# Ok::<(), tpt_doc_core::DocError>(())
```

### Handle non-default delimiters

Delimiters are read from MSH-2 automatically; pass them explicitly only when
parsing a message without a valid header.

```rust
use tpt_doc_hl7v2::{Delimiters, Hl7Parser};

// MSH-1 is the field separator (`|`); MSH-2 holds the other four encoding
// characters, here component `%`, repeat `~`, escape `\`, subcomponent `&`.
let message = b"MSH|%~\\&|HIS|FACILITY|RIS|FACILITY|||ORU%R01|MSG2|P|2.5.1\rPID|1||54321\r";

let segments: Vec<_> = Hl7Parser::new(message).collect::<Result<_, _>>()?;
assert_eq!(segments.iter().map(|s| s.tag()).collect::<Vec<_>>(), vec!["MSH", "PID"]);
assert_eq!(segments[1].field(3), Some("54321"));
assert_eq!(segments[0].component(9, 1), Some("ORU"));

// Passing explicit delimiters ignores MSH-2, so `ORU%R01` is no longer split
// into components and stays a single field value.
let explicit = Delimiters {
    field: b'|',
    component: b'^',
    repeat: b'~',
    escape: b'&',
    subcomponent: b'&',
};
let segments: Vec<_> = Hl7Parser::with_delimiters(message, explicit).collect::<Result<_, _>>()?;
assert_eq!(segments[0].field(9), Some("ORU%R01"));
# Ok::<(), tpt_doc_core::DocError>(())
```

### Build and encode a message

```rust
use tpt_doc_hl7v2::prelude::*;

let mut message = Message::msh(
    "HIS", "ACME",   // sending application / facility (MSH-3, MSH-4)
    "RIS", "ACME",   // receiving application / facility (MSH-5, MSH-6)
    "20261001120000",// message datetime (MSH-7)
    "ADT^A01",       // message type (MSH-9)
    "MSG00001",      // control id (MSH-10)
);
message.push_segment("EVN", &["A01", "20261001120000"]);
message.push_segment("PID", &["1", "", "12345^^^NZHPI^MR"]);
let bytes = message.to_bytes();

assert_eq!(
    String::from_utf8_lossy(&bytes),
    "MSH|^~\\&|HIS|ACME|RIS|ACME|20261001120000||ADT^A01|MSG00001|P|2.5.1\r\
     EVN|A01|20261001120000\r\
     PID|1||12345^^^NZHPI^MR\r"
);
```

### Schema validation

```rust
use tpt_doc_hl7v2::{schema::validate_message, schema::v251, Hl7Parser};

let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|MSG4|P|2.5.1\r\
                PID|1||12345^^^NZHPI^MR||Smith^John";
let segments: Vec<_> = Hl7Parser::new(message).collect::<Result<_, _>>()?;

let violations = validate_message(&v251(), &segments);
assert!(violations.is_empty(), "unexpected violations: {violations:?}");
# Ok::<(), tpt_doc_core::DocError>(())
```

## Modules

| Module | Contents |
|---|---|
| `parser` | `Hl7Parser` — streaming segment iterator. |
| `segment` | `Segment`, `Delimiters` — borrowed segment view and delimiter set. |
| `encode` | `Message`, `MessageSegment` — message building and serialization. |
| `schema` | `SegmentTables`, `FieldDef`, `Hl7Violation`, `v251()`, `validate_message()`. |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-hl7v2>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

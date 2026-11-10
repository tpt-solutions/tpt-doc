# tpt-doc-edi

Electronic data interchange in Rust: streaming parsers for EDIFACT and X12,
schema validation against embedded tables, functional and implementation
acknowledgments, and read-only readers for the common purchase-order, invoice,
ship-notice and despatch documents.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## What it does

- **Streaming parsers.** `EdifactParser` and `X12Parser` yield segments that
  borrow from the input, with no per-segment copies. Release characters in
  EDIFACT are handled by `elements_released`.
- **Schema validation.** Embedded EDIFACT D.96A and X12 tables check required
  elements, data types and lengths, envelope bracketing (UNB/UNZ, UNH/UNT,
  ISA/IEA, GS/GE, ST/SE), and control-number agreement. Tables can also be
  loaded at runtime with `ElementTables::from_tsv`.
- **Acknowledgments.** `x12_ack` builds and reads a `997` (functional) and a
  `999` (implementation) acknowledgment. `edifact_ack` builds and reads a
  `CONTRL`.
- **Document readers (read-only).** `x12_850` (purchase order), `x12_810`
  (invoice), `x12_856` (advance ship notice), and `edifact_docs` for `ORDERS`,
  `INVOIC` and `DESADV`. Each checks its control count.

## Quick start

```rust,ignore
use tpt_doc_edi::x12::X12Parser;
use tpt_doc_edi::x12_ack::build_997;
use tpt_doc_edi::x12_850::parse_850;

// Stream the segments of an interchange.
for segment in X12Parser::new(interchange)? {
    let segment = segment?;
    println!("{} {:?}", segment.tag(), segment.elements());
}

// Read a purchase order.
let order = parse_850(interchange)?;
println!("{} lines", order.lines.len());

// Acknowledge it: a 997 accepts the functional group and its sets.
let ack = build_997(interchange, true, "000000042", "260110", "0930")?;
```

## Main modules

| Module | Contents |
|---|---|
| `edifact`, `x12` | Streaming parsers and segment types |
| `schema` | Embedded and runtime tables, envelope and transaction-set validation |
| `x12_ack` | `build_997`, `parse_997`, `build_999`, `parse_999` |
| `edifact_ack` | `build_contrl`, `read_contrl` |
| `x12_850`, `x12_810`, `x12_856` | Purchase orders, invoices, ship notices (`parse_850`, `parse_810`, `parse_856`) |
| `edifact_docs` | `parse_document` for ORDERS, INVOIC and DESADV |

## Scope and known gaps

- The readers are read-only, apart from the acknowledgments. There is no
  writer for the transaction sets themselves yet.
- A `997` or `999` acknowledges whole groups and sets. It does not write
  element-level `AK3`/`AK4` or `IK3`/`IK4` error detail, so a rejection names
  the set, not the element.
- The `CONTRL` action codes are the ones this crate uses; check them against the
  UN/EDIFACT directory before relying on them with a trading partner.
- X12 `850`, `810` and `856` are read as the common single-transaction form.

## Limits and safety

The parsers return errors for malformed input and never panic on arbitrary
bytes. Fuzz targets for the EDIFACT and X12 parsers are in the repository's
`fuzz/` directory.

## Testing

```sh
cargo test -p tpt-doc-edi
```

The tests include round trips, envelope and control-count checks, runtime table
loading, and fixtures for each transaction set.

## Serde

The `serde` feature adds serde support to the crate's data types. Segments
borrow from the input, so they can be serialized but not deserialized.

## Related crates

- `tpt-doc-core` provides errors and validation reports.
- The facade `tpt-doc` re-exports this crate behind the `edi` feature.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

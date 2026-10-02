# tpt-doc-spreadsheet

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-spreadsheet.svg)](https://crates.io/crates/tpt-doc-spreadsheet)
[![Docs.rs](https://docs.rs/tpt-doc-spreadsheet/badge.svg)](https://docs.rs/tpt-doc-spreadsheet)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

Streaming `.xlsx` (OOXML) and `.csv` parsing and generation — without loading
entire files into RAM.

Peak memory is proportional to a **single row**, not to the whole workbook.

## Highlights

- Streaming `Iterator`-based row reading for both `.xlsx` and `.csv`.
- Dependency-light OOXML assembly using `zip` and `quick-xml` only.
- Typed `Cell` values (`String`, `Number`, `Bool`, `Null`).
- Round-trip tested: write a workbook, read it back, get identical cells.
- Deterministic output — identical input always produces identical bytes.

## Installation

```toml
[dependencies]
tpt-doc-spreadsheet = "0.1"
```

## Usage

### Round-tripping an `.xlsx` workbook

```rust
use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxReader, XlsxWriter};

// Write a workbook.
let mut writer = XlsxWriter::new();
writer.push_row(Row {
    index: 1,
    cells: vec![Cell::String("name".into()), Cell::Number(42.0)],
});
let bytes = writer.finish()?;

// Read it back.
let mut reader = XlsxReader::new(&bytes)?;
let rows: Vec<Row> = reader.rows().collect::<Result<_, _>>()?;
assert_eq!(
    rows[0].cells,
    vec![Cell::String("name".into()), Cell::Number(42.0)]
);
# Ok::<(), tpt_doc_core::DocError>(())
```

### Streaming CSV

```rust
use std::io::Cursor;
use tpt_doc_spreadsheet::csv::{CsvReader, CsvWriter};

let mut out = Vec::new();
{
    let mut writer = CsvWriter::new(&mut out);
    writer.write_record(["id", "status"])?;
    writer.write_record(["12345", "active"])?;
    writer.flush()?;
}
assert_eq!(String::from_utf8_lossy(&out), "id,status\n12345,active\n");

// The reader treats the first record as the header row, so `records()` yields
// the data rows only.
let mut reader = CsvReader::new(Cursor::new(out));
let records: Vec<Vec<String>> = reader.records().collect::<Result<_, _>>()?;
assert_eq!(records, vec![vec!["12345".to_string(), "active".to_string()]]);
# Ok::<(), tpt_doc_core::DocError>(())
```

## Modules

| Module | Contents |
|---|---|
| `xlsx` | `XlsxReader`, `XlsxWriter`, `Row`, `Cell` — streaming OOXML. |
| `csv` | `CsvReader`, `CsvWriter` — RFC 4180 style delimited text. |

## Memory model

`XlsxReader::rows()` yields `Result<Row, DocError>` items one at a time. The
workbook is never materialised as a whole, so a 2 GB spreadsheet costs the same
resident memory as a 2 KB one.

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-spreadsheet>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

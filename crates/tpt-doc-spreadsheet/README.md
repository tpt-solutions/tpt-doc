# tpt-doc-spreadsheet

Read and write `.xlsx` workbooks, and read and write CSV, in pure Rust. Reading
is streaming: rows are produced one at a time, and the whole sheet is never
built as a tree in memory. Output is deterministic, so the same rows always
produce the same bytes.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## What it does

- **Read `.xlsx`**: finds the first worksheet through `workbook.xml` and its
  relationships, resolves shared strings, and yields each row as `Row`.
- **Write `.xlsx`**: a minimal, spec-compliant package with a workbook, a
  worksheet and the shared parts, assembled into a ZIP.
- **Read and write CSV**: `CsvReader` treats the first line as the header, and
  `CsvWriter` writes records.
- **Limits**: row counts and entry sizes are bounded, and XML text is checked
  for characters XML cannot carry.

## Quick start

```rust,ignore
use tpt_doc_spreadsheet::xlsx::{Cell, Row, XlsxReader, XlsxWriter};

// Write a workbook.
let mut writer = XlsxWriter::new();
writer.push_row(Row {
    index: 1,
    cells: vec![Cell::String("Item".into()), Cell::Number(10.0)],
});
let bytes = writer.finish()?;

// Read it back, one row at a time.
let mut reader = XlsxReader::new(&bytes)?;
for row in reader.rows() {
    let row = row?;
    println!("{:?}", row.cells());
}
```

CSV with a header row:

```rust,ignore
use tpt_doc_spreadsheet::csv::CsvReader;

let mut reader = CsvReader::new(csv_bytes);
let header = reader.headers()?;
for record in reader.records() {
    let fields = record?;
    // ...
}
```

## Main types

| Item | Purpose |
|---|---|
| `xlsx::XlsxReader` | Streaming reader: `new`, `with_limits`, `rows` |
| `xlsx::XlsxWriter` | Writer: `new`, `push_row`, `finish` |
| `xlsx::Row`, `xlsx::Cell` | A row (index and cells) and a cell (`String`, `Number`, `Boolean`, `Error`, `Blank`) |
| `csv::CsvReader` | `new`, `headers`, `records` |
| `csv::CsvWriter` | `new`, `write_record`, `flush` |

## Limits

`XlsxReader::with_limits` takes a `tpt_doc_core::Limits`. Its `max_rows` caps
how many rows are yielded, and an over-limit read is an error, not a silent
truncation. The decompressed size of each part is bounded by `max_entry_bytes`.

## Scope and known gaps

- The reader handles the first worksheet only. Multi-sheet workbooks are not
  yet supported.
- Formulas, styles, dates and the CSV formula-injection guard are not handled.
  Values are read as the text or number stored in the file.

## Testing

```sh
cargo test -p tpt-doc-spreadsheet
```

The tests include round trips, a hand-written fixture, and property tests that
round-trip arbitrary rows.

## Related crates

- `tpt-doc-ooxml` provides the ZIP package handling this crate uses.
- `tpt-doc-core` provides errors and limits.
- The facade `tpt-doc` re-exports this crate behind the `spreadsheet` feature.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

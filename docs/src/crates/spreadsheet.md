# tpt-doc-spreadsheet

Strict, memory-efficient `.xlsx` (OOXML) and `.csv` parsing and generation without loading entire files into RAM.

## Design

`.xlsx` files are ZIP archives of XML. This crate streams ZIP entries using the `zip` crate and parses XML with `quick-xml`, mapping directly to Rust structs — no intermediate DOM tree.

## Usage

```rust
use tpt_doc_spreadsheet::xlsx::XlsxReader;

let bytes = std::fs::read("report.xlsx")?;
let mut reader = XlsxReader::new(&bytes)?;
for row in reader.rows() {
    let row = row?;
    println!("{:?}", row.cells());
}
```

## Key Types

- `XlsxReader` — streaming worksheet row iterator
- `XlsxWriter` — builds and serializes an `.xlsx` workbook
- `CsvReader` / `CsvWriter` — thin wrappers over the `csv` crate with `tpt-doc-core` error integration
- `Cell` — a single spreadsheet cell value (string, number, boolean, error, blank)
- `Row` — an ordered slice of `Cell` values with a 1-based row index

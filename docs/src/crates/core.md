# tpt-doc-core

Shared traits, zero-copy buffer abstractions, and unified error types for the tpt-doc ecosystem.

## Features

- `no_std` compatible (with `alloc`)
- `std` feature (default) enables `std::io` trait implementations
- Zero external dependencies

## Key Types

### `BufSlice<'a>`

A zero-copy wrapper over `&'a [u8]` that signals "this slice is borrowed from the original input — no copies were made."

### `DocError`

The unified error enum used across all tpt-doc crates. `#[non_exhaustive]` so new variants can be added without a semver break.

### `Limits`

Bounds applied when parsing untrusted documents. Every parser in the workspace
decompresses or buffers data whose size is declared by the input rather than
chosen by the caller, so these limits stop a malicious document from exhausting
memory before it is rejected.

| Field | Default | Bounds |
|---|---|---|
| `max_entry_bytes` | 200 MB | A single decompressed entry (ZIP member, PNG IDAT, XML part) |
| `max_rows` | 1,000,000 | Rows a tabular reader will yield |
| `max_xml_depth` | 256 | Nesting depth of XML elements |

Helpers: `read_to_end` is a bounded replacement for `Read::read_to_end`;
`check_rows` and `check_xml_depth` are the guards parsers call per row and per
element.

Where a limit applies, the crate's default entry point uses `Limits::default()`
and a `*_with_limits` variant takes explicit values:

```rust,ignore
use tpt_doc_core::Limits;
use tpt_doc_spreadsheet::xlsx::XlsxReader;

let limits = Limits {
    max_rows: 10_000,
    ..Limits::default()
};
let mut reader = XlsxReader::with_limits(&bytes, limits)?;
for row in reader.rows() {
    println!("{:?}", row?);
}
```

Exceeding a limit is always an error, never a silent stop: the caller sees the
row that tripped `max_rows`, or a `level limit` error for deep nesting.

### Traits

| Trait | Purpose |
|---|---|
| `DocReader` | Implemented by all document parsers |
| `DocWriter` | Implemented by all document generators |
| `Validate` | Runtime validation returning a structured `ValidationReport` |

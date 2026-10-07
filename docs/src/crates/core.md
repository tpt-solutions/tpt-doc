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

### `ValidationReport`

Every finding carries more than a message:

| Field | Purpose |
|---|---|
| `field_path` | JSON-pointer-style path, e.g. `/lines/0/quantity` |
| `message` | Human-readable description |
| `severity` | `Warning` or `Error` |
| `rule_id` | The published rule, e.g. `BR-CO-10`, `PEPPOL-EN16931-R001`, `obs-7` |
| `location` | `Path`, `Index` (segment), or `Offset` (byte position) |

`rule_id` matters because a caller should be able to trace a finding back to
the clause it came from without parsing prose.

```rust,ignore
let report = invoice.validate();
if report.has_errors() {
    for issue in report.errors() {
        eprintln!("{} [{}]: {}", issue.field_path, issue.rule_id, issue.message);
    }
}
// `has_errors` is what a receiver should check: warnings do not make a
// document unacceptable, so `is_valid()` alone is too strict for triage.
```

`ValidationReport::push(path, message)` records a `Severity::Error`, matching
its existing meaning; use `push_issue` with an explicit severity for
advisories.

Enable the `serde` feature to derive `Serialize`/`Deserialize`, so a report can
cross a process boundary (an MCP server, a CLI, an HTTP API). It is off by
default so the `no_std` build stays dependency-free.

# tpt-doc-core

Shared foundations for the tpt-doc crates: the unified error type, validation
reports, parser limits, the reader/writer/validator traits, and a strict reader
for the schema tables. It has no dependencies beyond `alloc`, and it builds for
`no_std` targets.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.
- **Repository:** <https://github.com/tpt-solutions/tpt-doc>

## What it provides

- **`DocError`**: one error type for every format. It has `InvalidFormat`,
  `MissingField`, `ValidationFailed`, `Located` (an error with a position),
  `Io` (with `std`) and `Other` (a wrapped dependency error). Its `Display`
  includes the position, and `source()` keeps the cause chain.
- **`Location`**: where a problem is: `Unknown`, a JSON-pointer-style `Path`, a
  segment or record `Index`, or a byte `Offset`.
- **`ValidationReport`**, **`ValidationIssue`** and **`Severity`**: validators
  return every issue they find, each with a severity, a location and, where the
  standard has one, a rule id. Only errors make a document invalid.
- **`Limits`**: bounds on untrusted input. The defaults are 200 MB for one
  decompressed entry, 1,000,000 rows, and 256 levels of XML nesting.
- **`DocReader`**, **`DocWriter`** and **`Validate`**: the shared traits. A type
  implements `from_bytes` and `to_bytes` for its canonical format.
- **`tsv`**: a strict reader for tab-separated schema tables. A malformed row is
  an error that names its line.
- **`escape_xml_text`** and **`validate_xml_chars`**: XML text escaping that
  refuses characters XML 1.0 cannot represent.
- **`BufSlice`**: a zero-copy wrapper over `&[u8]`.

## Quick start

```rust,ignore
use tpt_doc_core::{DocError, Limits, Location, ValidationReport};

fn check(value: &str) -> Result<(), DocError> {
    if value.is_empty() {
        return Err(DocError::invalid_format("empty value")
            .with_location(Location::Offset(0)));
    }
    Ok(())
}

fn report(values: &[&str]) -> ValidationReport {
    let mut report = ValidationReport::default();
    for (index, value) in values.iter().enumerate() {
        if value.is_empty() {
            report.push(format!("[{index}]"), "empty value");
        }
    }
    report
}

// Parsers take limits from the caller; the defaults are safe for most inputs.
let limits = Limits { max_rows: 10_000, ..Limits::default() };
```

## Features

| Feature | Default | Effect |
|---|---|---|
| `std` | yes | Enables `std::io` integration and `DocError::Io`. |
| `serde` | no | `Serialize`/`Deserialize` for `ValidationReport` and its parts. |

Build for a `no_std` target with `--no-default-features`, for example
`cargo check -p tpt-doc-core --no-default-features --target thumbv7em-none-eabihf`.

## Main types

| Item | Purpose |
|---|---|
| `DocError` | The error type for every format |
| `Location` | Where in the input a problem is |
| `ValidationReport`, `ValidationIssue`, `Severity` | Structured validation results |
| `Limits` | Bounds for untrusted input |
| `DocReader`, `DocWriter`, `Validate` | Shared traits |
| `tsv::rows`, `tsv::Row` | Strict schema-table reading |
| `escape_xml_text`, `validate_xml_chars` | XML safety |

## Safety with untrusted input

Parsers in the workspace return errors and do not panic on arbitrary bytes.
Property tests check this for each format, and a fuzz target per parser is
kept in the repository's `fuzz/` directory. Decompression and row counts are
bounded by `Limits`.

## Testing

```sh
cargo test -p tpt-doc-core
cargo check -p tpt-doc-core --no-default-features --target thumbv7em-none-eabihf
```

## Related crates

Every other tpt-doc crate depends on this one. The facade crate `tpt-doc`
re-exports the format crates behind feature flags.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

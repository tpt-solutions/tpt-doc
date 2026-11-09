# Error handling

tpt-doc has two kinds of failure, and they are handled differently:

- **Errors** (`DocError`) mean the input could not be read at all: it is not the
  expected format, a mandatory field is missing, or a limit was exceeded.
  They are returned as `Err(..)` and stop the parse.
- **Validation findings** (`ValidationReport`) mean the input was read, but
  some rules were broken. A validator collects every finding it can, each with a
  severity, a location, and where available a rule id, so you can report all of
  them at once.

Treat untrusted input as the normal case: every parser returns an error rather
than panicking, and every crate's fuzz-style property tests check that arbitrary
bytes never cause a panic.

## `DocError`

`DocError` is `#[non_exhaustive]`, so match the variants you care about and keep
a catch-all arm.

| Variant | Meaning | Typical cause |
|---|---|---|
| `InvalidFormat(String)` | Bytes are not valid for the format | Truncated file, wrong format, bad delimiter |
| `MissingField(&'static str)` | A mandatory field was absent | Invoice without a document ID |
| `ValidationFailed(String)` | A field value broke a constraint | Amount with three decimals |
| `Located { source, location }` | Wraps another error with a position | XML syntax error at byte 412 |
| `Io(std::io::Error)` | I/O failure (`std` feature) | Reading from a closed file |
| `Other(Box<dyn Error>)` | Wrapped error from a dependency | Underlying ZIP or XML error |

### Positions

Parsers attach a `Location` when they know where the problem is:

- `Location::Offset(u64)`: a byte offset in the raw input (XML, PDF, ZIP).
- `Location::Index(usize)`: a segment or record index (EDI, HL7 v2).
- `Location::Path(String)`: a JSON-pointer-style path (FHIR, data-merge).
- `Location::Unknown`: no position is available.

```rust,ignore
match tpt_doc_ubl::Invoice::from_xml(xml) {
    Ok(invoice) => { /* ... */ }
    Err(e) => {
        if let Some(offset) = e.byte_offset() {
            eprintln!("parse error at byte {offset}: {e}");
        } else {
            eprintln!("{e}");
        }
    }
}
```

`Display` appends the position to the message, and `Error::source()` still
returns the underlying cause, so error chains keep working with `anyhow`,
`eyre` and similar crates.

If you want the bare cause without the position, call `without_location()`.
If you only need to inspect the position, use `location()` or `byte_offset()`.

## `ValidationReport`

Validators return a `ValidationReport` rather than stopping at the first
problem. Each `ValidationIssue` has:

- `severity`: `Severity::Error` or `Severity::Warning`.
- `rule_id`: the standard's rule identifier, such as a PEPPOL `BR-CO-15`, a FHIR invariant `obs-7`, or an EDI segment rule. It is empty when the check has no published id.
- `location`: where the problem is.
- `field_path` and `message`: where and what.

```rust,ignore
let report = /* validator output */;

if report.has_errors() {
    for issue in report.errors() {
        println!("{} {}: {}", issue.rule_id, issue.field_path, issue.message);
    }
}

for warning in report.warnings() {
    eprintln!("warning: {}", warning.message);
}
```

Only errors make a document invalid. A document with warnings alone is still
valid, so `has_errors()` is the check to use for pass or fail.

Reports can be combined with `merge`, so a pipeline can validate several
layers (for example the envelope and the payload) and act on one combined
result.

### Mapping per-format errors

Each format keeps its own error type, and most provide a `to_report` function
that converts it into a `ValidationReport`:

| Crate | Error type | Converter |
|---|---|---|
| `tpt-doc-edi` | `SchemaViolation` | `SchemaViolation::to_report` |
| `tpt-doc-hl7v2` | `Hl7Violation` | `Hl7Violation::to_report` |
| `tpt-doc-fhir` | `FhirValidationError` | `FhirValidationError::to_report` |
| `tpt-doc-ubl` | `UblValidationError` | `UblValidationError::to_report` |

Use these when you need one reporting shape across formats, for example to
produce a single JSON response. The per-format types remain the source of truth
if you need the exact variant.

## Limits are errors

Exceeding a [`Limits`](../crates/core.md) bound is always an error, never a
silent truncation. A reader that stops at `max_rows` returns the row that
tripped the limit, so a caller can tell a short file from a capped one.

## The CLI

The `tpt` CLI maps outcomes to exit codes, which is useful in scripts:

| Exit code | Meaning |
|---|---|
| `0` | Success, or valid (`validate`), or identical (`diff`) |
| `1` | Validation found errors, or inputs differ (`diff`) |
| `2` | The input could not be read, or the command failed |

```sh
tpt validate --json invoice.xml || echo "rejected"
```

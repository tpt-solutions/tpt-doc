# tpt-doc-diff

Semantic diff for healthcare and business documents. It compares two documents
by their structure and meaning, not their bytes, so reordered segments with the
same content do not show as changes.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## Supported formats

| Format | Function |
|---|---|
| HL7 v2 | `diff_hl7` |
| EDIFACT | `diff_edifact` |
| X12 | `diff_x12` |
| JSON (FHIR and other) | `diff_json` |
| XLSX | `diff_xlsx` |
| DOCX | `diff_docx` |

Each function takes the left document, the right document and an `Options`
value. `Options::ignoring(prefix)` skips every path that starts with `prefix`,
for example `meta.lastUpdated` for FHIR resources that change on every save.
Each function returns a `Diff`, and `Diff::is_empty()` tells you whether the
two documents match.

## Quick start

```rust,ignore
use tpt_doc_diff::{diff_json, Options};

let options = Options::default().ignoring("meta.lastUpdated");
let diff = diff_json(left, right, &options)?;
if !diff.is_empty() {
    // inspect the reported changes
}
```

The command-line tool:

```sh
tpt diff old.hl7 new.hl7 --ignore PID --json
```

Exit codes: `0` when the documents are equivalent, `1` when they differ, and `2`
when either input cannot be read.

## Scope and known gaps

- Changes are reported by path. The crate does not yet produce a patch that can
  be applied to the left document.
- The `ignore` list matches path prefixes only, not patterns.

## Testing

```sh
cargo test -p tpt-doc-diff
```

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

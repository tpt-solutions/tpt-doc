# tpt-doc-cli

The `tpt` command-line tool. It inspects, validates, renders, converts,
redacts and compares documents from the terminal, and it is built on the same
library crates as the rest of the workspace.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## Commands

| Command | Purpose |
|---|---|
| `tpt sniff <FILE>` | Identify the format of a file |
| `tpt inspect <FILE>` | Show the structure of a document |
| `tpt validate <FILE> [--json]` | Validate and print the report |
| `tpt render <FILE.html> [--data D.json] [-o OUT]` | Render HTML to PDF, optionally with JSON data |
| `tpt text <FILE.docx>` | Print the text of a DOCX file |
| `tpt convert <FILE.hl7> [-o OUT]` | Convert an HL7 v2 message to FHIR |
| `tpt redact <FILE.hl7> [-o OUT]` | Remove identifying fields from an HL7 v2 message |
| `tpt diff <A> <B> [--ignore PREFIX]... [--json]` | Compare two documents |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success, or the documents are equivalent or valid |
| `1` | The document is invalid, or the documents differ |
| `2` | The command could not run, for example a missing or unreadable file |

Scripts can rely on these codes, so `tpt validate` can gate a pipeline.

## Install

From a checkout of the repository:

```sh
cargo install --path crates/tpt-doc-cli
```

## Examples

```sh
tpt validate invoice.xml --json
tpt render report.html --data report.json -o report.pdf
tpt convert adt.hl7 -o patient-bundle.json
tpt diff before.hl7 after.hl7 --ignore PID
```

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

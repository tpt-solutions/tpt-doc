# tpt-doc-cli

The `tpt` command line tool for the [tpt-doc](https://github.com/tpt-solutions/tpt-doc) workspace. Formats are detected from file *content*, not extension.

```text
tpt sniff <FILE>                 print the detected format
tpt inspect <FILE>               structural summary
tpt validate <FILE> [--json]     validate against the format's rules
tpt render <FILE.html> [-o OUT]  render HTML to PDF (default: stdout)
tpt text <FILE.docx>             extract plain text
tpt convert <FILE.hl7> [-o OUT]  HL7 v2 ADT/ORU to a FHIR R5 bundle (JSON)
```

`<FILE>` may be `-` for stdin.

| Format | sniff | inspect | validate |
|---|---|---|---|
| HL7 v2.x | ✓ | ✓ | segment tables (v2.5.1) |
| EDIFACT D.96A | ✓ | ✓ | element + envelope rules |
| X12 | ✓ | ✓ | element + ISA/GS/ST envelope rules |
| FHIR JSON | ✓ | ✓ | Observation / Encounter invariants |
| UBL / PEPPOL | ✓ | ✓ | PEPPOL BIS 3.0 rules |
| DOCX / XLSX | ✓ | ✓ | parses cleanly |

**Exit codes:** `0` success/valid, `1` document has validation errors, `2` usage or I/O error. With `--json`, `validate` prints `{format, valid, issues:[{severity, path, location, rule, message}]}`, suitable for CI.

```bash
cargo install --path crates/tpt-doc-cli
tpt validate invoice.xml --json
```

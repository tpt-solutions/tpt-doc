# tpt-doc-templates

Worked examples and golden tests for the tpt-doc workspace. Each template is a
realistic input with its expected output, and the tests fail when a change
alters that output unexpectedly. This crate is not published: it exists to
demonstrate and test the other crates together.

- **Status:** unpublished (`publish = false`). MSRV 1.88. Licence `MIT OR Apache-2.0`.

## Templates

| Folder | Shows |
|---|---|
| `01-hl7-adt-to-fhir` | HL7 v2 ADT message converted to FHIR |
| `02-x12-835-remittance` | X12 835 payment remittance |
| `03-ubl-invoice-facturx` | UBL invoice with a Factur-X attachment |
| `04-spreadsheet-report` | Spreadsheet report |
| `05-signed-contract` | PDF contract signed with PAdES |

See `templates/README.md` for the inputs and expected outputs of each.

## Running the tests

```sh
cargo test -p tpt-doc-templates
```

The golden tests compare generated output with the files in `tests/`. To
regenerate the expected files after an intended change, run:

```sh
TPT_UPDATE_GOLDEN=1 cargo test -p tpt-doc-templates
```

Review the diff before committing regenerated golden files.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

# Template gallery

Five worked examples. Each folder holds an **input** and the **expected output**
the library produces from it. The tests in `crates/tpt-doc-templates` regenerate
every output and compare it byte for byte.

| Folder | Input | Expected output | What it shows |
|---|---|---|---|
| `01-hl7-adt-to-fhir` | HL7 v2 `ADT^A01` admission | `expected.json` — FHIR R5 bundle | HL7 → FHIR conversion with `tpt-doc-convert` |
| `02-x12-835-remittance` | X12 835 remittance advice | `expected.txt` — payment summary | Reading claims and payments from an X12 remittance |
| `03-ubl-invoice-facturx` | PEPPOL BIS 3.0 invoice (UBL XML) | `expected.pdf` — Factur-X PDF | Validating an invoice, then embedding its XML in a PDF |
| `04-spreadsheet-report` | CSV data | `expected.xlsx` — spreadsheet | Numbers and text, header row, XLSX output |
| `05-signed-contract` | HTML agreement | `expected.pdf` — unsigned PDF | HTML rendering, then a PAdES signature |

## Running the gallery

```sh
cargo test -p tpt-doc-templates
```

Test 05 also signs its PDF. The signature is randomized, so it is checked
structurally: the rendered PDF survives as a prefix of the signed file and a
signature dictionary is present. The certificate and key used are the public
test fixtures from `tpt-doc-sign`. They are **not** a real identity, so do not
use them for anything that matters.

## Changing an output

When a change to the library is meant to change an output, regenerate the
expected files and review the diff before committing:

```sh
TPT_UPDATE_GOLDEN=1 cargo test -p tpt-doc-templates
git diff templates/
```

A golden file that changes without an intended reason is a regression.

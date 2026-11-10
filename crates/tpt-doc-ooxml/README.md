# tpt-doc-ooxml

The shared package layer for Office Open XML files. XLSX, DOCX and similar
formats are ZIP archives of XML parts, and this crate reads and writes that
archive structure safely. It is used by `tpt-doc-spreadsheet` and `tpt-doc-word`.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## What it does

- **`PackageWriter`** builds a ZIP package from named XML parts and returns the
  bytes with `finish`. Entries are written in a fixed order with a fixed
  timestamp and the DOS epoch, so the same parts always give the same bytes.
- **`open_archive`** opens a ZIP package and returns an archive handle. It
  rejects bytes that are not a ZIP file.
- **`read_part`** returns one part's bytes by name, for example
  `xl/workbook.xml`. It takes a `tpt_doc_core::Limits` and refuses any part
  larger than `max_entry_bytes`, so a zip bomb is not inflated without bound.
- **`XML_DECL`** is the standard XML declaration used for generated parts.

## Quick start

```rust,ignore
use tpt_doc_ooxml::{open_archive, read_part, PackageWriter};

let mut package = PackageWriter::new();
package.add_part("[Content_Types].xml", content_types_xml)?;
package.add_part("xl/workbook.xml", workbook_xml)?;
let bytes = package.finish()?;

let mut archive = open_archive(&bytes)?;
let workbook = read_part(&mut archive, "xl/workbook.xml", &Limits::default())?;
```

## Scope and known gaps

- The crate handles the package layer only. It does not interpret any
  WordprocessingML or SpreadsheetML content; that is the job of the format
  crates.
- Encrypted packages are not supported.

## Testing

```sh
cargo test -p tpt-doc-ooxml
```

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

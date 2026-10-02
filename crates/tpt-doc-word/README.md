# tpt-doc-word

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-word.svg)](https://crates.io/crates/tpt-doc-word)
[![Docs.rs](https://docs.rs/tpt-doc-word/badge.svg)](https://docs.rs/tpt-doc-word)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

Pure-Rust `.docx` (OOXML Word) document generation and parsing — **no COM, no
LibreOffice, no C-FFI**.

`.docx` files are ZIP archives of XML — the same structure as `.xlsx`. This
crate reuses the same `zip` + `quick-xml` approach as
[`tpt-doc-spreadsheet`](https://crates.io/crates/tpt-doc-spreadsheet), producing
spec-compliant Word documents entirely in Rust.

## Highlights

- Build documents in memory (`DocxDocument`), then serialize with `DocxWriter::write`.
- Read text back out of any `.docx` with `DocxReader::extract_text`.
- Paragraphs with multiple formatted runs and built-in styles.
- Tables with rows, cells, and border styles.
- Headers and footers.
- Deterministic output — identical input always produces identical bytes.
- Minimal dependency set: `zip`, `quick-xml`, `serde`.

## Installation

```toml
[dependencies]
tpt-doc-word = "0.1"
```

## Usage

### Build a document

```rust
use tpt_doc_word::prelude::*;

let mut doc = DocxDocument::new();
doc.push_paragraph(Paragraph::styled("Heading1", "Quarterly Report"));
doc.push_paragraph(Paragraph::new("Prepared by TPT Solutions."));

let bytes = DocxWriter::write(&doc)?;
let text = DocxReader::extract_text(&bytes)?;
assert!(text.contains("Quarterly Report"));
# Ok::<(), tpt_doc_core::DocError>(())
```

### Formatted runs and built-in styles

```rust
use tpt_doc_word::prelude::*;

let mut paragraph = Paragraph::styled(BuiltinStyle::Heading2.style_id(), "Findings");
paragraph.push_run(Run::styled(" (final).", RunStyle::default().italic()));
paragraph.push_run(Run::styled(" Reported.", RunStyle::default().bold()));

assert_eq!(paragraph.runs().len(), 3);
assert_eq!(paragraph.plain_text(), "Findings (final). Reported.");
```

### Built-in style IDs

`Paragraph::styled` takes a style *id* string. `BuiltinStyle` converts to and
from those ids so you never have to hard-code them:

```rust
use tpt_doc_word::BuiltinStyle;

assert_eq!(BuiltinStyle::Heading2.style_id(), "Heading2");
assert_eq!(BuiltinStyle::from_style_id("Heading2"), Some(BuiltinStyle::Heading2));
assert_eq!(BuiltinStyle::Heading2.name(), "heading 2");
assert_eq!(BuiltinStyle::from_style_id("Nonsense"), None);
```

### Tables

```rust
use tpt_doc_word::prelude::*;

let mut doc = DocxDocument::new();
let mut table = Table::new();
table.push_row(Row::new([
    Cell::new("Item"),
    Cell::new("Amount"),
]));
doc.push_table(table);

assert_eq!(doc.body().len(), 1);
```

### Header and footer

```rust
use tpt_doc_word::prelude::*;

let mut doc = DocxDocument::new();
doc.set_header("TPT Solutions");
doc.set_footer("Confidential");
assert_eq!(doc.header(), Some("TPT Solutions"));
assert_eq!(doc.footer(), Some("Confidential"));
```

## Modules

| Module | Contents |
|---|---|
| `document` | `DocxDocument`, `BodyElement`, `DocxWriter`, `DocxReader`. |
| `paragraph` | `Paragraph`, `Run`, `RunStyle`. |
| `styles` | `BuiltinStyle`, `StyleSet`. |
| `table` | `Table`, `Row`, `Cell`, `BorderStyle`. |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-word>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

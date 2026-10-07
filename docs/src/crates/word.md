# tpt-doc-word

Pure-Rust `.docx` (OOXML Word) document generation and parsing — no COM automation, no LibreOffice.

## Design

`.docx` files are ZIP archives of XML parts — the same structure as `.xlsx`. The crate reuses the `zip` + `quick-xml` infrastructure from `tpt-doc-spreadsheet`: an in-memory `DocxDocument` model (paragraphs, runs, tables) is serialized to `word/document.xml`, `word/styles.xml`, and supporting package parts, then assembled into a deterministic ZIP.

## Usage

```rust
use tpt_doc_word::prelude::*;

let mut doc = DocxDocument::new();
doc.push_paragraph(Paragraph::styled("Heading1", "Quarterly Report"));
doc.push_paragraph(Paragraph::new("Prepared by TPT Solutions."));

let mut table = Table::new();
table.push_row(Row::new([Cell::new("Region"), Cell::new("Revenue")]));
doc.push_table(table);

let bytes = DocxWriter::write(&doc)?;
```

## Limits

`DocxReader::extract_text_with_limits` takes explicit `tpt_doc_core::Limits`: the
decompressed `word/document.xml` part is size-bounded, and nesting is refused
past `max_xml_depth`. A document body is only a few elements deep, so a deeply
nested part is corrupt or an attempt to make the parser recurse without bound.

## Package Parts

| Part | Purpose |
|---|---|
| `[Content_Types].xml` | MIME type declarations for all parts |
| `_rels/.rels` | Package-level relationships |
| `word/document.xml` | The document body |
| `word/styles.xml` | Normal, Heading1–6, Title, TableGrid styles |
| `word/settings.xml` | Compatibility settings |

# tpt-doc-pdf

Pure-Rust PDF 1.7/2.0 generation and parsing. No pdfium, no C-FFI.

## Design

Documents are built as a declarative model (`Document → Page → ContentStream`) and serialized directly to PDF objects with precise byte-offset cross-reference tables, ensuring 100% spec compliance and minimal file sizes.

Font subsetting for TrueType/OTF is handled via `ttf-parser`. Content streams are compressed with `flate2`.

## Usage

```rust
use tpt_doc_pdf::{Document, Page, Font};

let mut doc = Document::new();
let font = doc.embed_builtin_font(Font::Helvetica);
let mut page = Page::a4();
page.text("Hello, world!", font, 12.0, (50.0, 750.0));
doc.add_page(page);

let pdf_bytes = doc.write()?;
std::fs::write("output.pdf", pdf_bytes)?;
```

## Key Types

- `Document` — root PDF object; owns pages, fonts, and resources
- `Page` — a single page with a content stream
- `ContentStream` — PDF graphics operators (text, paths, images)
- `Font` — embedded or built-in font reference
- `XrefTable` — byte-exact cross-reference table written during serialization

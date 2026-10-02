# tpt-doc-pdf

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-pdf.svg)](https://crates.io/crates/tpt-doc-pdf)
[![Docs.rs](https://docs.rs/tpt-doc-pdf/badge.svg)](https://docs.rs/tpt-doc-pdf)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

Pure-Rust PDF 1.7 / 2.0 generation and parsing.

**No pdfium. No C-FFI. No shelling out.** Documents are built as a declarative
model and serialized with precise byte-offset cross-reference tables.

## Highlights

- Declarative document model (`Document`, `Page`) — assemble, then `write()`.
- Built-in Type1 font programs: Helvetica, Helvetica-Bold/Oblique,
  Helvetica-BoldOblique, Times-Roman, Times-Bold, Times-Italic.
- Image embedding: JPEG passthrough and PNG transcoding to `/FlateDecode`.
- File attachments via PDF associated files (the Factur-X / ZUGFeRD mechanism).
- Deterministic output — fixed object numbering, no timestamps, fixed
  compression level, so identical documents produce byte-identical PDFs.
- Correct xref construction in [`xref`], exposed for advanced use.

## Installation

```toml
[dependencies]
tpt-doc-pdf = "0.1"
```

## Usage

### Build a document

```rust
use tpt_doc_pdf::{Document, Font, Page};

let mut doc = Document::new();
let font = doc.embed_builtin_font(Font::Helvetica);

let mut page = Page::a4();
page.text("Hello, PDF!", font, 12.0, (72.0, 700.0));
page.rect(72.0, 680.0, 451.0, 40.0);
doc.add_page(page);

let bytes = doc.write()?;
assert!(bytes.starts_with(b"%PDF-1.7"));
# Ok::<(), tpt_doc_core::DocError>(())
```

### Embed an image

```rust
use tpt_doc_core::DocError;
use tpt_doc_pdf::{Document, Image, Page};

// Build a tiny in-memory PNG (1x1, 8-bit greyscale) so the example is
// self-contained and reproducible.
let png = Image::from_png(&[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR: length + type
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // IHDR width 1, height 1
    0x08, 0x00, 0x00, 0x00, 0x00, // depth 8, greyscale, deflate, adaptive, no interlace
    0x3A, 0x7E, 0x9B, 0x55, // IHDR CRC
    0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, // IDAT: length + type
    0x78, 0x9C, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, // zlib stream
    0x00, 0x01, 0x48, 0xAF, 0xA4, 0x71, 0x00, 0x00, // adler32 + CRC
    0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, // IEND: length + type
    0x60, 0x82, // IEND CRC
])?;

let mut doc = Document::new();
let image_id = doc.embed_image(png);

let mut page = Page::letter();
page.image(image_id, 72.0, 400.0, 200.0, 150.0);
doc.add_page(page);

let bytes = doc.write()?;
assert!(bytes.starts_with(b"%PDF-1.7"));
# Ok::<(), DocError>(())
```

### Attach a file (Factur-X / ZUGFeRD)

```rust
use tpt_doc_core::DocError;
use tpt_doc_pdf::Document;

// An embedded file travels with the PDF as an associated file; this is the
// mechanism Factur-X / ZUGFeRD uses to attach invoice XML to a PDF invoice.
let invoice_xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice><ID>INV-001</ID><IssueDate>2026-01-15</IssueDate></Invoice>"#;

let mut doc = Document::new();
doc.attach_file("factur-x.xml", "xml", invoice_xml.to_vec());
let bytes = doc.write()?;

assert!(bytes.starts_with(b"%PDF-1.7"));
# Ok::<(), DocError>(())
```

## Modules

| Module | Contents |
|---|---|
| `document` | `Document`, `Attachment` — model and serialization. |
| `page` | `Page` — geometry and content stream operations. |
| `font` | `Font` — built-in Type1 fonts. |
| `image` | `Image`, `ImageFilter` — JPEG/PNG embedding. |
| `xref` | `XrefTable`, `XrefEntry` — cross-reference table construction. |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-pdf>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

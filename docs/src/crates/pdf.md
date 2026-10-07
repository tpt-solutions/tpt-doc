# tpt-doc-pdf

Pure-Rust PDF 1.7/2.0 generation and parsing. No pdfium, no C-FFI.

## Design

Documents are built as a declarative model (`Document → Page → ContentStream`) and serialized directly to PDF objects with precise byte-offset cross-reference tables, ensuring 100% spec compliance and minimal file sizes.

Built-in Type1 fonts need no embedding. Content streams are compressed with `flate2`.
(TrueType subsetting is not implemented; see the crate README.)

## Images

`Image::from_jpeg` embeds the stream verbatim under `/DCTDecode`, and reads the SOF
component count so a grayscale or CMYK JPEG is declared `/DeviceGray` or
`/DeviceCMYK` rather than being mislabelled `/DeviceRGB`. A component count no
device colour space covers is rejected.

`Image::from_png` decodes non-interlaced 8-bit images (grayscale, RGB, palette,
RGBA) and re-deflates them under `/FlateDecode`. RGBA alpha is deliberately
dropped rather than folded into the colour, since `/DeviceRGB` has no alpha
channel; use a `/SMask` for real transparency. The IDAT inflate is bounded by
`tpt_doc_core::Limits`, and the declared pixel count is checked before anything
is allocated.

## Validation on write

`Document::write` fails rather than emitting a file a conforming reader would
reject:

- a page referencing a font or image id that was never embedded
- a non-finite coordinate (NaN/inf from layout arithmetic), which would
  otherwise be written as a bare word into the content stream

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
- `Font` — built-in Type1 font reference
- `Image` — an embedded raster image, with its `ColorSpace` and `ImageFilter`
- `Attachment` — a file attached via PDF 2.0 associated files (`/AF`, `/AFRelationship /Data`)
- `XrefTable` — byte-exact cross-reference table written during serialization

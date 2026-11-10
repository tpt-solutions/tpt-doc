# tpt-doc-pdf

Build PDF documents in pure Rust, and read the text of PDF files. Output is
deterministic: the same document model always gives the same bytes. No
C library and no external process is involved.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## What it does

### Writing

- **`Document`** holds pages, fonts, images and file attachments, and writes
  them with a byte-exact cross-reference table.
- **`Page`** provides `a4()` and `letter()`, with text, images and rectangles.
  Content streams are Flate-compressed.
- **Built-in fonts** (`Font::Helvetica`, `Times`, `Courier` and their bold and
  italic faces) use WinAnsi encoding.
- **Images**: JPEG is embedded unchanged, and PNG is decoded and re-encoded with
  Flate.
- **Attachments** are embedded files, the mechanism Factur-X uses.
- `Document` implements `tpt_doc_core::DocWriter`.

### Reading

- **`extract::extract_pages`** and **`extract_text`** return the text of each
  page. Classic cross-reference tables and PDF 1.5 cross-reference streams are
  both read, along with objects stored in object streams and the `/Prev` chains
  of incremental updates. Signed files read correctly.
- Encrypted files and TIFF predictors are refused with an error.

## Quick start

```rust,ignore
use tpt_doc_pdf::{Document, Font, Page};
use tpt_doc_pdf::extract::extract_text;

let mut doc = Document::new();
let font = doc.embed_builtin_font(Font::Helvetica);

let mut page = Page::a4();
page.text("Invoice 5001", font, 14.0, (72.0, 760.0));
page.rect(72.0, 740.0, 468.0, 1.0);
doc.add_page(page);

let bytes = doc.write()?;
let text = extract_text(&bytes)?;
assert!(text.contains("Invoice 5001"));
```

## Main types

| Item | Purpose |
|---|---|
| `Document` | Pages, fonts, images and attachments; `write`, `attach_file` |
| `Page` | `a4`, `letter`, `text`, `image`, `rect` |
| `Font` | Built-in font faces |
| `image::Image` | `from_jpeg`, `from_png` |
| `extract::extract_pages`, `extract::extract_text` | Text extraction |

## Features

| Feature | Default | Effect |
|---|---|---|
| `serde` | no | `Serialize`/`Deserialize` for the document model. A deserialized document is checked by `write`. |

## Limits and safety

Stream decompression is bounded by `Limits::max_entry_bytes`. Predictor rows
and object-stream sizes are checked before allocation. The extractor refuses
malformed structure with an error rather than guessing.

## Scope and known gaps

- Only the built-in fonts are available for writing. Embedded TrueType fonts
  are not supported yet, and subsetting is not implemented.
- The writer does not produce PDF/A or PDF/A-3 conformance. Factur-X PDF/A-3
  metadata is left to the caller.
- The reader does not handle encryption, or TIFF predictors.
- Text extraction assumes WinAnsi-encoded fonts. Other encodings may come out
  wrong.

## Testing

```sh
cargo test -p tpt-doc-pdf
```

The tests include a deterministic structure snapshot, byte-exact xref checks,
a miniature spec-walking reader, and extraction tests that build PDFs with
object streams and compressed cross-reference streams.

## Related crates

- `tpt-doc-sign` signs PDFs with PAdES.
- `tpt-doc-layout` renders HTML into this crate's document model.
- `tpt-doc-ubl` embeds Factur-X invoices through `attach_file`.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

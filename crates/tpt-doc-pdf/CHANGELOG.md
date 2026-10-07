# Changelog

All notable changes to `tpt-doc-pdf` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `image::ColorSpace` on `Image`, and the JPEG SOF component count is read
  during `from_jpeg` so a grayscale or CMYK JPEG is declared `/DeviceGray` or
  `/DeviceCMYK` instead of being mislabelled `/DeviceRGB`. Component counts no
  device colour space covers are rejected.
- `Document::write` rejects a page whose font or image id was never embedded,
  and refuses non-finite coordinates rather than writing `NaN`/`inf` into a
  content stream.

### Fixed

- An attachment MIME subtype is escaped as a PDF name token: every delimiter
  (`/ ( ) < > [ ] { } % #` and whitespace) is escaped, not just `/`, so an
  untrusted subtype cannot inject dictionary syntax.
- A PNG palette index past the end of `PLTE` is reported instead of being
  rendered as black, which silently showed the wrong pixels.
- The PNG IDAT inflate is bounded by `tpt_doc_core::Limits`, so a small file
  cannot expand into gigabytes before the pixel count is checked.

## [0.1.0] - 2026-02-10

### Added

- `document` module with the declarative `Document` model, `Attachment`
  support (PDF associated files), and deterministic `write()` serialization.
- `page` module with `Page::a4()` / `Page::letter()` presets and `text()`,
  `image()`, and `rect()` content-stream operations.
- `font` module with built-in Type1 fonts: Helvetica, Helvetica-Bold,
  Helvetica-Oblique, Helvetica-BoldOblique, Times-Roman, Times-Bold,
  Times-Italic.
- `image` module: `Image::from_jpeg` (passthrough) and `Image::from_png`
  (transcode to `/FlateDecode`), plus `ImageFilter`.
  `from_png` now validates the IHDR chunk against the full 13-byte layout —
  previously a 10-, 11-, or 12-byte IHDR read `data[12]` out of bounds — checks
  the compression and filter methods, rejects unknown colour types, bounds the
  declared pixel count before allocating, and converts the chunk length with
  `try_from` so a hostile `0xFFFFFFFF` length cannot overflow the slicing.
  Each chunk's CRC-32 is now verified, so a corrupt or non-PNG file is
  rejected rather than decoded into a silently wrong image.
- `xref` module exposing `XrefTable` / `XrefEntry` for precise byte-offset
  cross-reference construction.
- Deterministic serialization guarantees: fixed object numbering, no
  timestamps, fixed zlib compression level.
- Integration tests plus `insta` snapshots and `proptest` round-trip tests.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-pdf-v0.1.0

# Changelog

All notable changes to `tpt-doc-layout` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-10

### Added

- `Renderer` with `new()`, `letter()`, `with_margin()`,
  `with_base_font_size()`, `render_html()`, and `render_pages()`.
- `PageKind` page classification for downstream consumers.
- `html` module: minimal HTML document model (`HtmlNode`, `BlockElement`,
  `TableRow`), `parse()`, and `decode_entities()`.
- `css` module: inline CSS parsing into `ComputedStyle` with `TextAlign`
  support.
- `render` module: block layout, automatic pagination, and
  `approx_text_width()` for font-metric estimation.
- PDF output delegated to `tpt-doc-pdf`.
- Deterministic output guarantee, verified with `insta` snapshots.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-layout-v0.1.0

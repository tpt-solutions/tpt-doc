# Changelog

All notable changes to `tpt-doc-layout` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `template` module and `Renderer::render_template`: Mustache-style data merge (`{{field}}`, `#if`/`else`, `#each`) over JSON, HTML-escaped, strict about missing fields.
- `ComputedStyle::is_bold`, applying CSS's 600-and-above bold threshold.

### Fixed

- The HTML tokenizer tracks quoting, so a `>` inside a quoted attribute value
  no longer ends the tag. `style="font-weight: bold > 700"` used to leak
  `700">` into the rendered text.
- `<div>` is a container rather than a paragraph. It previously collected its
  children into an inline list the renderer never walked, so **nested blocks
  were silently dropped** — an `h1`, `p`, or table inside a `div` never reached
  the page.
- `<p>` and headings implicitly close an open paragraph, as HTML requires.
  `<p>one<p>two` was a single paragraph reading "one two".
- `margin` and `padding` are applied: top and bottom margin shift the block,
  and left padding narrows the content column for that block only (it is
  scoped, so it does not leak into the next block). Both were parsed and then
  discarded, so all spacing came from the engine's fixed gap.
- `font-weight: bold` selects the bold face. The property was parsed but never
  consulted, so only `<b>` produced bold text.
- Table cell text wraps inside its column. The row box grew to fit the wrapped
  height, but the cell was still drawn as a single text op that ran past the
  column edge.

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

# tpt-doc-layout

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-layout.svg)](https://crates.io/crates/tpt-doc-layout)
[![Docs.rs](https://docs.rs/tpt-doc-layout/badge.svg)](https://docs.rs/tpt-doc-layout)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

Minimal, deterministic, headless HTML/CSS-to-PDF layout engine for generating
compliant reports.

Targets report generation, not browser fidelity: a restricted, well-defined
HTML/CSS subset is parsed, block-laid-out, and handed to
[`tpt-doc-pdf`](https://crates.io/crates/tpt-doc-pdf) for output.

## Highlights

- Headless — no browser, no JavaScript runtime, no system web engine.
- Deterministic — the same HTML always produces byte-identical PDF output.
- Block layout engine with automatic pagination across pages.
- Inline CSS parsing with a computed-style model.
- HTML entity decoding for `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&nbsp;`.

## Installation

```toml
[dependencies]
tpt-doc-layout = "0.1"
```

## Usage

### Render HTML to PDF

```rust
use tpt_doc_layout::Renderer;

let html = "<h1>Annual Report</h1><p>FY2026 results.</p>";
let pdf = Renderer::new().render_html(html)?;
assert!(pdf.starts_with(b"%PDF-1.7"));
# Ok::<(), tpt_doc_core::DocError>(())
```

### Page geometry and typography

```rust
use tpt_doc_layout::Renderer;

let renderer = Renderer::new()
    .letter()
    .with_margin(54.0)             // 0.75 inch margins
    .with_base_font_size(11.0);

let pdf = renderer.render_html("<p>Indented report body.</p>")?;
# Ok::<(), tpt_doc_core::DocError>(())
```

### Inspect the intermediate pages

```rust
use tpt_doc_layout::{PageKind, Renderer};

let renderer = Renderer::new();
let pages = renderer.render_pages("<h1>Report</h1><p>One.</p><p>Two.</p>");
assert!(!pages.is_empty());
assert_ne!(PageKind::A4, PageKind::Letter);
```

## Modules

| Module | Contents |
|---|---|
| `html` | `HtmlNode`, `BlockElement`, `TableRow`, `parse`, `decode_entities`. |
| `css` | `ComputedStyle`, `TextAlign`, `parse_inline`. |
| `render` | `Renderer`, `PageKind`, `approx_text_width`. |

## Supported subset

| Feature | Support |
|---|---|
| Block elements (`h1`–`h6`, `p`, `div`, `ul`, `ol`, `li`, `table`) | Yes |
| Inline formatting (`b`, `i`, `span`) | Yes |
| HTML entities | Yes |
| Inline CSS (`text-align`, `font-size`, `font-weight`, margins) | Yes |
| JavaScript, external resources, CSS animations | No (by design) |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-layout>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

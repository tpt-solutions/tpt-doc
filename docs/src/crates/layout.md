# tpt-doc-layout

A minimal, deterministic, headless HTML/CSS-to-PDF layout engine for generating compliant reports.

## Design

Targets report-generation use cases, not full browser rendering. Parses a restricted HTML/CSS subset (no JavaScript, no external network resources) and delegates final PDF output to `tpt-doc-pdf`.

## Supported HTML Subset

- Block elements: `<div>`, `<p>`, `<h1>`–`<h6>`, `<table>`, `<tr>`, `<td>`, `<th>`
- Inline elements: `<span>`, `<strong>`, `<em>`, `<a>`, `<img>` (local data URIs only)
- `<br>`, `<hr>`

## Supported CSS Properties

- `font-family`, `font-size`, `font-weight`, `font-style`, `color`
- `margin`, `padding`, `border`
- `page-break-before`, `page-break-after`
- `width`, `height` (on table cells)
- `text-align`

## Usage

```rust
use tpt_doc_layout::Renderer;

let html = r#"
    <h1>Annual Report</h1>
    <p>FY2026 results...</p>
"#;

let pdf_bytes = Renderer::new().render_html(html)?;
std::fs::write("report.pdf", pdf_bytes)?;
```

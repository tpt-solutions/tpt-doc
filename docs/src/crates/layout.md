# tpt-doc-layout

A minimal, deterministic, headless HTML/CSS-to-PDF layout engine for generating compliant reports.

## Design

Targets report-generation use cases, not full browser rendering. Parses a restricted HTML/CSS subset (no JavaScript, no external network resources) into a block tree, lays it out with a line-box model (word wrapping, page breaks), and delegates final PDF output to `tpt-doc-pdf`. Text measurement uses a deterministic per-character approximation of the Helvetica metrics, so identical input produces byte-identical output.

## Supported HTML Subset

- Block elements: `<div>`, `<p>`, `<h1>`-`<h6>`, `<table>`, `<tr>`, `<td>`, `<th>`
- Blocks nest: `<div>` is a container, so `<div><h1>…</h1><table>…</table></div>`
  lays out both children
- `<p>` and headings implicitly close an open paragraph, so `<p>one<p>two`
  produces two paragraphs rather than one
- Inline elements: `<b>`/`<strong>` (bold), `<i>`/`<em>` (italic)
- `html`/`head`/`body` wrappers are transparent; `head` content is dropped
- The five predefined entities plus numeric character references are decoded
- A `>` inside a quoted attribute value does not end the tag
- Table cell text wraps inside its column; the row box grows to fit
- Malformed input never panics - unclosed elements are closed implicitly

## Supported CSS Properties

- `font-size` (`pt`, `px`), `font-weight` (600 and above selects the bold face)
- `margin` (top and bottom), `padding` (left, scoped to the block)
- `page-break-before`, `page-break-after`
- `text-align` (left, right, center, justify)

## Usage

```rust
use tpt_doc_layout::Renderer;

let html = r#"
    <h1>Annual Report</h1>
    <p>FY2026 results &amp; commentary.</p>
    <table><tr><th>Region</th><th>Revenue</th></tr>
    <tr><td>Auckland</td><td>12000</td></tr></table>
"#;

let pdf_bytes = Renderer::new().render_html(html)?;
std::fs::write("report.pdf", pdf_bytes)?;
```

Tables render with fixed equal-width columns and stroked cell borders; rows that do not fit start a new page.

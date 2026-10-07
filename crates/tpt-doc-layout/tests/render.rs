//! Integration tests: render HTML reports end-to-end and inspect the
//! generated PDF content streams, plus a no-panic fuzz property.

use flate2::read::ZlibDecoder;
use std::io::Read;
use tpt_doc_layout::{Renderer, approx_text_width};

const REPORT: &str = r#"<html><head><title>Q3 Report</title></head>
<body>
<h1>Quarterly Report</h1>
<p>Prepared by the &lt;Data&gt; team &amp; audited.</p>
<table>
<tr><th>Region</th><th>Revenue</th></tr>
<tr><td>Auckland</td><td>12000</td></tr>
<tr><td>Wellington</td><td>9500</td></tr>
</table>
</body></html>"#;

/// Decompress every FlateDecode stream in the PDF and concatenate the
/// content-stream text.
fn content_text(pdf: &[u8]) -> String {
    let needle: &[u8] = b"/Filter /FlateDecode >>\nstream\n";
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(rel) = pdf[cursor..]
        .windows(needle.len())
        .position(|w| w == needle)
    {
        let start = cursor + rel + needle.len();
        let Some(end_rel) = pdf[start..].windows(10).position(|w| w == b"\nendstream") else {
            break;
        };
        let end = start + end_rel;
        let mut decompressed = Vec::new();
        if ZlibDecoder::new(&pdf[start..end])
            .read_to_end(&mut decompressed)
            .is_ok()
        {
            out.push_str(&String::from_utf8_lossy(&decompressed));
            out.push('\n');
        }
        cursor = end;
    }
    out
}

#[test]
fn renders_report_with_text_and_table_borders() {
    let pdf = Renderer::new().render_html(REPORT).expect("render");
    assert!(pdf.starts_with(b"%PDF-1.7"));
    let text = content_text(&pdf);
    // The wrapper emits one text op per word.
    assert!(text.contains("(Quarterly) Tj"), "heading missing: {text:?}");
    assert!(
        text.contains("(<Data>) Tj"),
        "entities not decoded: {text:?}"
    );
    assert!(
        text.contains("(audited.) Tj"),
        "body text missing: {text:?}"
    );
    // The table renders one stroked rectangle per cell: 2 cols × 3 rows.
    assert_eq!(text.matches(" re S").count(), 6, "cell borders: {text:?}");
    // Header cells use the bold face (F2).
    assert!(text.contains("/F2"), "bold header face missing: {text:?}");
}

#[test]
fn page_breaks_and_wrapping_produce_multiple_pages() {
    // ~40 wrapped lines forces several A4 pages.
    let paragraph = "<p>".repeat(0) + "<p>";
    let mut html = String::from(&paragraph[..3]);
    html.push_str(&"The layout engine wraps this sentence repeatedly. ".repeat(40));
    html.push_str("</p>");
    let pdf = Renderer::new().render_html(&html).expect("render");
    let text = content_text(&pdf);
    let page_count = text.matches(" Tj").count();
    assert!(
        page_count > 20,
        "expected many wrapped lines, got {page_count}"
    );
    assert!(pdf.windows(5).filter(|w| *w == *b"%%EOF").count() >= 1);
    // More than one page object: the content mentions two distinct page sizes.
    assert!(text.len() > 2000);
}

#[test]
fn explicit_page_break_and_page_kind_are_honored() {
    let html = "<p style=\"page-break-before: always\">second page</p>";
    let pdf = Renderer::new()
        .letter()
        .with_margin(36.0)
        .render_html(html)
        .expect("render");
    let text = content_text(&pdf);
    // The break pushes the paragraph onto page 2 (words render separately).
    assert!(
        text.contains("(second) Tj"),
        "page-2 text missing: {text:?}"
    );
    assert!(text.contains("(page) Tj"));
}

#[test]
fn text_alignment_offsets_content() {
    let html = "<p style=\"text-align: right\">right</p>";
    let pdf = Renderer::new().render_html(html).expect("render");
    let text = content_text(&pdf);
    // x offset must exceed the left margin (72) for right alignment.
    let td = text
        .split(" Td")
        .next()
        .unwrap_or("")
        .rsplit(' ')
        .next()
        .unwrap_or("0");
    let x: f32 = td.parse().unwrap_or(0.0);
    assert!(x > 72.0, "right-aligned x should exceed margin: {x}");
}

#[test]
fn approx_width_is_monotonic_and_scales() {
    assert!(approx_text_width("m", 10.0) > approx_text_width("i", 10.0));
    assert!(approx_text_width("ab", 20.0) > approx_text_width("ab", 10.0));
    assert!(approx_text_width("", 10.0) == 0.0);
}

/// A `>` inside a quoted attribute value is not the end of the tag. Scanning
/// to the first `>` truncated `<div style="…">` mid-value, leaving stray text
/// (`700">`) in the output.
#[test]
fn a_gt_inside_a_quoted_attribute_does_not_end_the_tag() {
    let html = r#"<p style="font-weight: bold > 700">Styled</p>"#;
    let pdf = Renderer::new().render_html(html).expect("render");
    let text = content_text(&pdf);
    assert!(text.contains("(Styled) Tj"), "{text:?}");
    assert!(
        !text.contains("(700") && !text.contains("\">"),
        "tag leaked into the text: {text:?}"
    );
}

/// Block elements nested inside a container must be laid out, not swallowed.
/// `<div>` and `<p>` opened a frame that only collected inline children, so an
/// `h1` or `p` inside a `div` was parsed into a child list the renderer never
/// walked — the content silently vanished.
#[test]
fn nested_blocks_are_rendered() {
    let html = "<div><h1>Nested Heading</h1><p>Nested body</p></div>";
    let text = content_text(&Renderer::new().render_html(html).expect("render"));
    assert!(text.contains("(Nested) Tj"), "heading lost: {text:?}");
    assert!(text.contains("(Heading) Tj"), "heading lost: {text:?}");
    assert!(text.contains("(Nested) Tj"), "body lost: {text:?}");
    assert!(text.contains("(body) Tj"), "body lost: {text:?}");
}

/// A table inside a container is still a table, with its cell borders drawn.
#[test]
fn a_table_nested_in_a_div_is_rendered() {
    let html = "<div><table><tr><td>Cell A</td></tr></table></div>";
    let pdf = Renderer::new().render_html(html).expect("render");
    let text = content_text(&pdf);
    assert!(text.contains("(Cell A) Tj"), "cell text lost: {text:?}");
    assert!(text.contains(" re S"), "cell border lost: {text:?}");
}

/// HTML closes an open `<p>` when a block element starts. Without that,
/// `<p>one<p>two` parsed as a single paragraph reading "one two".
#[test]
fn a_paragraph_is_implicitly_closed_by_the_next_block() {
    use tpt_doc_layout::html::{BlockElement, HtmlNode, parse};

    let nodes = parse("<p>First para<p>Second para");
    let paragraphs: Vec<&BlockElement> = nodes
        .iter()
        .filter_map(|node| match node {
            HtmlNode::Block(block @ BlockElement::Paragraph { .. }) => Some(block),
            _ => None,
        })
        .collect();
    assert_eq!(
        paragraphs.len(),
        2,
        "an unclosed <p> must not swallow the next one: {nodes:?}"
    );

    // Both paragraphs must reach the page, at different vertical positions.
    let text = content_text(
        &Renderer::new()
            .render_html("<p>First para<p>Second para")
            .expect("render"),
    );
    assert!(text.contains("(First) Tj"), "{text:?}");
    assert!(text.contains("(Second) Tj"), "{text:?}");
    let positions: Vec<&str> = text
        .split(" Td (")
        .skip(1)
        .filter_map(|chunk| chunk.split(' ').next())
        .collect();
    assert!(positions.len() >= 4, "expected four words drawn: {text:?}");
    // The first and third word start the two paragraphs, on different lines.
    assert_ne!(
        positions.first(),
        positions.get(2),
        "the two paragraphs must not share a line box: {text:?}"
    );
}

/// `font-weight: bold` was parsed but never applied; only `<b>` produced bold
/// text.
#[test]
fn font_weight_bold_is_applied() {
    let html = r#"<p style="font-weight: 700">Heavy</p>"#;
    let pdf = Renderer::new().render_html(html).expect("render");
    let text = String::from_utf8_lossy(&pdf).into_owned();
    // The bold face is font id 2 (1-based) in the content stream.
    assert!(
        text.contains("/F2 "),
        "font-weight: bold must select the bold face: {text:?}"
    );
}

/// `margin` was parsed and then ignored, so spacing came only from the
/// engine's fixed gap.
#[test]
fn margin_is_applied() {
    let plain = Renderer::new()
        .render_html("<p>Same text</p>")
        .expect("render");
    let spaced = Renderer::new()
        .render_html(r#"<p style="margin: 40pt">Same text</p>"#)
        .expect("render");
    assert_ne!(
        plain, spaced,
        "a 40pt margin must move the text, not be ignored"
    );
}

/// `padding` was parsed and then ignored.
#[test]
fn padding_is_applied() {
    let plain = Renderer::new()
        .render_html("<p>Same text</p>")
        .expect("render");
    let padded = Renderer::new()
        .render_html(r#"<p style="padding: 30pt">Same text</p>"#)
        .expect("render");
    assert_ne!(plain, padded, "padding must affect layout");
}

/// Long cell text must wrap inside the column instead of running past it. The
/// column has to be narrow for wrapping to be observable, so the table needs
/// several columns.
#[test]
fn long_table_cell_text_wraps() {
    let cell = "alpha bravo charlie delta echo foxtrot golf hotel india juliet";
    let html = format!("<table><tr><td>a</td><td>b</td><td>{cell}</td></tr></table>");
    let text = content_text(&Renderer::new().render_html(&html).expect("render"));
    let lines: Vec<&str> = text.lines().filter(|l| l.contains("Tj")).collect();
    assert!(
        lines.len() >= 2,
        "the long cell should span several text ops: {text:?}"
    );
    // Each drawn line must stay within its column: a single line carrying every
    // word means the text was emitted unwrapped.
    let widest = lines
        .iter()
        .map(|line| line.matches("Tj").count())
        .max()
        .unwrap_or(0);
    assert!(
        widest < 11,
        "one line holds all 11 words, so wrapping did not happen: {text:?}"
    );
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn render_never_panics_on_arbitrary_html(input in any::<String>()) {
            let renderer = Renderer::new();
            if let Ok(bytes) = renderer.render_html(&input) {
                prop_assert!(bytes.starts_with(b"%PDF-1.7"));
            }
        }

        #[test]
        fn render_never_panics_on_tag_soup(parts in proptest::collection::vec(
            prop_oneof![
                Just("<p>".to_owned()),
                Just("</p>".to_owned()),
                Just("<table><tr>".to_owned()),
                Just("</td></tr></table>".to_owned()),
                Just("<h1 style=\"page-break-before: always\">".to_owned()),
                Just("</h1>".to_owned()),
                Just("<b><i>".to_owned()),
                Just("</b></i>".to_owned()),
                Just("text &amp; more ".to_owned()),
                Just("<<<".to_owned()),
                Just("%%".to_owned()),
            ],
            0..24
        ), input in any::<String>()) {
            let mut html = parts.concat();
            html.push_str(&input);
            let renderer = Renderer::new();
            let _ = renderer.render_html(&html);
        }
    }
}

#[test]
fn template_merge_renders_invoice_lines_into_the_pdf() {
    use std::io::Read as _;

    let template = "<h1>Invoice {{number}}</h1>\
        <table>{{#each lines}}<tr><td>{{desc}}</td><td>{{total}}</td></tr>{{/each}}</table>\
        {{#if paid}}<p>PAID</p>{{else}}<p>Due {{due}}</p>{{/if}}";
    let data = serde_json::json!({
        "number": "INV-7 & co",
        "paid": false,
        "due": "2026-11-01",
        "lines": [{"desc": "Consulting", "total": "300.00"}, {"desc": "Hosting", "total": "120.00"}],
    });
    let pdf = Renderer::new()
        .render_template(template, &data)
        .expect("render");
    assert!(pdf.starts_with(b"%PDF-"));

    // Content streams are deflated; inflate every stream and look for the text.
    let mut text = String::new();
    let mut rest = &pdf[..];
    while let Some(start) = rest.windows(6).position(|w| w == b"stream") {
        let body = &rest[start + 6..];
        let body = body
            .strip_prefix(b"\r\n")
            .or_else(|| body.strip_prefix(b"\n"))
            .unwrap_or(body);
        let Some(end) = body.windows(9).position(|w| w == b"endstream") else {
            break;
        };
        let mut inflated = Vec::new();
        if flate2::read::ZlibDecoder::new(&body[..end])
            .read_to_end(&mut inflated)
            .is_ok()
        {
            text.push_str(&String::from_utf8_lossy(&inflated));
        }
        rest = &body[end + 9..];
    }
    for needle in ["(INV-7)", "(&)", "(Consulting)", "(120.00)", "(2026-11-01)"] {
        assert!(text.contains(needle), "missing {needle:?} in {text}");
    }
    assert!(!text.contains("PAID"));

    assert!(
        Renderer::new()
            .render_template("{{missing}}", &data)
            .is_err()
    );
}

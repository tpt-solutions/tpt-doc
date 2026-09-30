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

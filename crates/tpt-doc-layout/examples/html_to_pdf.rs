//! Render an HTML report to a PDF file.
//!
//! Run with `cargo run -p tpt-doc-layout --example html_to_pdf`.

use tpt_doc_layout::Renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = "<h1>Quarterly Report</h1><p>Revenue grew &amp; margins held.</p>\
                <table><tr><td>Q1</td><td>100</td></tr><tr><td>Q2</td><td>120</td></tr></table>";
    let pdf = Renderer::new().render_html(html)?;
    let path = std::env::temp_dir().join("tpt-doc-report.pdf");
    std::fs::write(&path, &pdf)?;
    println!("wrote {} ({} bytes)", path.display(), pdf.len());
    Ok(())
}

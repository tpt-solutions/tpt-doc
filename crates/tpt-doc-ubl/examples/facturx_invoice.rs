//! Parse a PEPPOL BIS 3.0 invoice, validate it, and embed it in a PDF as a
//! Factur-X associated file.
//!
//! Run with `cargo run -p tpt-doc-ubl --example facturx_invoice --features facturx`.

use tpt_doc_pdf::{Document, Font, Page};
use tpt_doc_ubl::{Invoice, invoice::facturx, validate};

const INVOICE: &[u8] = include_bytes!("../tests/fixtures/peppol_invoice.xml");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let invoice = Invoice::from_xml(INVOICE)?;
    if let Some(first) = validate(&invoice).first() {
        return Err(format!("invoice fails validation: {first:?}").into());
    }

    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text(
        format!("Invoice {} from {}", invoice.id, invoice.supplier.name),
        font,
        14.0,
        (72.0, 760.0),
    );
    doc.add_page(page);
    facturx::attach_xml(&invoice, &mut doc)?;

    let pdf = doc.write()?;
    let path = std::env::temp_dir().join("tpt-doc-facturx.pdf");
    std::fs::write(&path, &pdf)?;
    println!("wrote {} ({} bytes)", path.display(), pdf.len());
    Ok(())
}

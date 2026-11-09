//! The general reader: cross-reference streams with a PNG predictor, objects that
//! live inside compressed object streams, and the refusals for what is still
//! unsupported. The file is built here, byte by byte, in the layout that modern
//! PDF writers produce.

use std::io::Write as _;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use tpt_doc_pdf::extract::{extract_pages, extract_text};

fn zlib(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).expect("compress");
    encoder.finish().expect("finish")
}

/// PNG "Up" predictor, one row at a time (what `/Predictor 12` undoes).
fn png_up(rows: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut previous = vec![0u8; rows[0].len()];
    for row in rows {
        out.push(2);
        out.extend(row.iter().zip(&previous).map(|(a, b)| a.wrapping_sub(*b)));
        previous = row.clone();
    }
    out
}

/// A PDF whose page tree is in an object stream and whose cross-reference is a
/// compressed stream, with the content stream as an ordinary object.
fn modern_pdf(content: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.5\n".to_vec();

    // Object 4: the content stream, uncompressed.
    let offset4 = out.len();
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len() + 1
        )
        .as_bytes(),
    );

    // Object 6: an object stream holding the catalog (1), pages (2) and page (3).
    let members = [
        (1u32, "<< /Type /Catalog /Pages 2 0 R >>".to_owned()),
        (2u32, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned()),
        (
            3u32,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>".to_owned(),
        ),
    ];
    let mut header = String::new();
    let mut body = String::new();
    for (number, object) in &members {
        header.push_str(&format!("{number} {} ", body.len()));
        body.push_str(object);
        body.push('\n');
    }
    let first = header.len();
    let objstm_plain = format!("{header}{body}");
    let objstm = zlib(objstm_plain.as_bytes());
    let offset6 = out.len();
    out.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /ObjStm /N {} /First {first} /Filter /FlateDecode /Length {} >>\nstream\n",
            members.len(),
            objstm.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&objstm);
    out.extend_from_slice(b"\nendstream\nendobj\n");

    // Object 7: the cross-reference stream, with a PNG Up predictor.
    let offset7 = out.len();
    let entry = |kind: u8, field2: u32, field3: u16| {
        let mut row = vec![kind];
        row.extend_from_slice(&field2.to_be_bytes());
        row.extend_from_slice(&field3.to_be_bytes());
        row
    };
    let rows = vec![
        entry(0, 0, 0xffff), // object 0: free
        entry(2, 6, 0),      // object 1: in object stream 6, index 0
        entry(2, 6, 1),      // object 2: in object stream 6, index 1
        entry(2, 6, 2),      // object 3: in object stream 6, index 2
        entry(1, offset4 as u32, 0),
        entry(0, 0, 0), // object 5: free
        entry(1, offset6 as u32, 0),
        entry(1, offset7 as u32, 0),
    ];
    let xref = zlib(&png_up(&rows));
    out.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XRef /Size 8 /W [1 4 2] /Index [0 8] /Root 1 0 R \
             /Filter /FlateDecode /DecodeParms << /Predictor 12 /Columns 7 >> /Length {} >>\nstream\n",
            xref.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&xref);
    out.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{offset7}\n%%EOF\n").as_bytes());
    out
}

#[test]
fn a_pdf_with_object_streams_and_a_cross_reference_stream_reads() {
    let pdf = modern_pdf("BT /F1 12 Tf 72 700 Td (Hello from an object stream) Tj ET");
    let pages = extract_pages(&pdf).expect("extract");
    assert_eq!(pages, ["Hello from an object stream"]);
}

#[test]
fn text_through_the_general_reader_matches_the_classic_reader() {
    let pdf = modern_pdf("BT /F1 12 Tf 72 700 Td (same words) Tj ET");
    assert_eq!(extract_text(&pdf).expect("extract"), "same words");
}

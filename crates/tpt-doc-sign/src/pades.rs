use std::fmt::Write as _;

use ring::rand::SecureRandom;
use tpt_doc_core::DocError;

use crate::cades;
pub use crate::pem::PrivateKey;

/// Sign a PDF document with a `PAdES-B`-style detached signature.
///
/// An incremental update containing the signature dictionary is appended to
/// the PDF, so the original bytes remain untouched and the document stays a
/// valid, openable PDF. The `/Contents` placeholder covers exactly the CMS
/// bytes (its length is measured in a first pass), and `/ByteRange` excludes
/// that placeholder from the digest as required.
///
/// # Errors
/// Returns [`DocError`] if the PDF is malformed or the signing operation fails.
pub fn sign(
    pdf_bytes: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
) -> Result<Vec<u8>, DocError> {
    validate_pdf(pdf_bytes)?;
    let object_number = next_object_number(pdf_bytes);
    let prev_startxref = last_startxref(pdf_bytes)?;
    let root_ref = root_reference(pdf_bytes)?;

    // Pass 1: measure the exact CMS length. Every element of the CMS has a
    // fixed-width encoding, so the length is independent of signature values.
    let cms_len = cades::sign_detached(b"", key, cert_der, rng)?.len();
    let placeholder_len = 2 * cms_len; // lowercase hex, 2 chars per byte

    // Build the update with a zero placeholder to derive ByteRange offsets.
    // The final document is `original ++ update`; ByteRange offsets are
    // file-absolute.
    let (update_tail, contents_hex_start_rel) = build_update(
        pdf_bytes,
        object_number,
        placeholder_len,
        prev_startxref,
        &root_ref,
    );
    let base = pdf_bytes.len();
    let mut update = Vec::with_capacity(base + update_tail.len());
    update.extend_from_slice(pdf_bytes);
    update.extend_from_slice(&update_tail);

    let marker = "/ByteRange [PADDINGPLACEHOLDER]";
    let marker_start = find_bytes(&update[base..], marker.as_bytes())
        .map(|rel| base + rel)
        .ok_or_else(|| DocError::invalid_format("internal error: ByteRange marker missing"))?;

    // Offsets of the two digest ranges. The substituted ByteRange text is
    // padded to exactly the marker's width, so lengths never shift.
    let x1 = base + contents_hex_start_rel;
    let y1 = x1 + placeholder_len;
    // ByteRange is [start, length, start, length]: the second range runs
    // from the end of /Contents to the end of the file.
    let z1 = update.len() - y1;
    let byte_range_text = format!("/ByteRange [0 {x1} {y1} {z1}]");
    if byte_range_text.len() > marker.len() {
        return Err(DocError::invalid_format(
            "internal error: ByteRange text exceeds reserved placeholder width",
        ));
    }
    let padded = format!(
        "{byte_range_text}{}",
        " ".repeat(marker.len() - byte_range_text.len())
    );
    update.splice(
        marker_start..marker_start + marker.len(),
        padded.as_bytes().to_vec(),
    );

    // Digest both ranges and produce the CMS.
    let signed_data: Vec<u8> = update[..x1]
        .iter()
        .chain(update[y1..y1 + z1].iter())
        .copied()
        .collect();
    let cms = cades::sign_detached(&signed_data, key, cert_der, rng)?;
    if cms.len() != cms_len {
        return Err(DocError::invalid_format(
            "internal error: CMS length changed between passes",
        ));
    }
    let mut hex = String::with_capacity(cms.len() * 2);
    for byte in &cms {
        let _ = write!(hex, "{byte:02x}");
    }
    let zeros = "0".repeat(placeholder_len);
    let zeros_at = find_bytes(&update, zeros.as_bytes())
        .ok_or_else(|| DocError::invalid_format("internal error: placeholder not found"))?;
    update.splice(
        zeros_at..zeros_at + placeholder_len,
        hex.as_bytes().to_vec(),
    );
    Ok(update)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn validate_pdf(pdf_bytes: &[u8]) -> Result<(), DocError> {
    if !pdf_bytes.starts_with(b"%PDF-") {
        return Err(DocError::invalid_format("input is not a PDF file"));
    }
    let tail = &pdf_bytes[pdf_bytes.len().saturating_sub(8)..];
    if !tail.windows(5).any(|w| w == b"%%EOF") {
        return Err(DocError::invalid_format(
            "PDF does not end with %%EOF — incremental updates require a complete file",
        ));
    }
    Ok(())
}

/// The next free object number: one past the highest `N 0 obj` in the file.
///
/// Operates on raw bytes — content streams are compressed and need not be
/// valid UTF-8.
fn next_object_number(pdf_bytes: &[u8]) -> u32 {
    let needle = b" 0 obj";
    let mut max = 0u32;
    let mut cursor = 0usize;
    while let Some(rel) = find_bytes(&pdf_bytes[cursor..], needle) {
        let index = cursor + rel;
        let mut start = index;
        while start > 0 && pdf_bytes[start - 1].is_ascii_digit() && index - start < 10 {
            start -= 1;
        }
        if let Ok(n) = std::str::from_utf8(&pdf_bytes[start..index])
            .unwrap_or("")
            .parse::<u32>()
            && n > max
        {
            max = n;
        }
        cursor = index + needle.len();
    }
    max + 1
}

fn last_startxref(pdf_bytes: &[u8]) -> Result<u64, DocError> {
    // The trailer is the final structure in the file and is plain ASCII;
    // scan only the tail so compressed content cannot confuse the search.
    let tail_start = pdf_bytes.len().saturating_sub(64);
    let tail = &pdf_bytes[tail_start..];
    let marker = find_bytes(tail, b"startxref")
        .ok_or_else(|| DocError::invalid_format("PDF has no startxref"))?;
    let digits_start = marker + b"startxref".len();
    let digits_start = digits_start
        + tail[digits_start..]
            .iter()
            .take_while(|b| !b.is_ascii_digit())
            .count();
    let digits_len = tail[digits_start..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    let digits = std::str::from_utf8(&tail[digits_start..digits_start + digits_len])
        .unwrap_or("")
        .trim();
    digits
        .parse::<u64>()
        .map_err(|e| DocError::invalid_format(format!("startxref unparsable: {e}")))
}

fn root_reference(pdf_bytes: &[u8]) -> Result<String, DocError> {
    // The trailer is plain ASCII; scan only the tail of the file.
    let tail_start = pdf_bytes.len().saturating_sub(256);
    let tail = &pdf_bytes[tail_start..];
    let marker = find_bytes(tail, b"/Root ")
        .ok_or_else(|| DocError::invalid_format("PDF trailer has no /Root"))?;
    let rest = &tail[marker + b"/Root ".len()..];
    let end = find_bytes(rest, b" >>").unwrap_or(rest.len());
    let value = std::str::from_utf8(&rest[..end])
        .map_err(|_| DocError::invalid_format("PDF trailer is not ASCII"))?;
    Ok(value.trim().to_owned())
}

/// Build the incremental update with a `/ByteRange [PLACEHOLDER]` marker and
/// a zero-filled `/Contents` hex placeholder; return the update and the
/// offset of the first hex character.
fn build_update(
    pdf_bytes: &[u8],
    object_number: u32,
    placeholder_len: usize,
    prev_startxref: u64,
    root_ref: &str,
) -> (Vec<u8>, usize) {
    let zeros = "0".repeat(placeholder_len);
    let mut update = Vec::new();
    update.push(b'\n');

    // ByteRange is written with a fixed-width marker first so that the
    // Contents offset is stable; it is substituted afterwards.
    update.extend_from_slice(format!("{object_number} 0 obj\n").as_bytes());
    let dict_start = update.len();
    let dict_prefix = "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /M (D:20260101000000Z) /ByteRange [PADDINGPLACEHOLDER] /Contents <";
    update.extend_from_slice(dict_prefix.as_bytes());
    let contents_hex_start = dict_start + dict_prefix.len();
    update.extend_from_slice(zeros.as_bytes());
    update.extend_from_slice(b">>\nendobj\n");

    // New xref for the incremental update, chained via /Prev.
    let object_offset = pdf_bytes.len() + 1; // after the leading newline
    let xref_offset = update.len();
    update.extend_from_slice(b"xref\n");
    update.extend_from_slice(format!("{object_number} 1\n").as_bytes());
    update.extend_from_slice(format!("{object_offset:010} 00000 n\r\n").as_bytes());
    update.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root {root_ref} /Prev {prev_startxref} >>\nstartxref\n{xref_offset}\n%%EOF\n",
            object_number + 1
        )
        .as_bytes(),
    );
    (update, contents_hex_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_pdf_input_is_rejected() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        assert!(sign(b"not a pdf", &key, &cert, &rng).is_err());
        assert!(sign(b"%PDF-1.7 but no EOF", &key, &cert, &rng).is_err());
    }
}

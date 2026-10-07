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
/// `/M` (the signing time) and the certificate validity check both use the
/// system clock. Use [`sign_at`] to supply the time explicitly, which is what
/// reproducible builds and tests want.
///
/// # Errors
/// Returns [`DocError`] if the PDF is malformed, the certificate is not valid at
/// the signing time, or the signing operation fails.
pub fn sign(
    pdf_bytes: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
) -> Result<Vec<u8>, DocError> {
    sign_at(pdf_bytes, key, cert_der, rng, unix_now())
}

/// Sign a PDF, taking the signing time as an explicit number of seconds since
/// the Unix epoch.
///
/// A signature that claims a hardcoded time is worse than one with no time at
/// all: for an audit-trail format the recorded moment is part of what is being
/// attested, so making it injectable also makes it reproducible in tests.
///
/// The certificate's validity window is checked against `unix_seconds`, so an
/// expired or not-yet-valid certificate fails here rather than producing a
/// signature that no conforming verifier accepts.
///
/// # Errors
/// Returns [`DocError`] if the PDF is malformed, the certificate is not valid at
/// `unix_seconds`, or the signing operation fails.
pub fn sign_at(
    pdf_bytes: &[u8],
    key: &PrivateKey,
    cert_der: &[u8],
    rng: &dyn SecureRandom,
    unix_seconds: i64,
) -> Result<Vec<u8>, DocError> {
    validate_pdf(pdf_bytes)?;
    // The same instant decides both what /M claims and whether the signer
    // certificate was actually valid then.
    crate::pem::Certificate::from_der(cert_der)?.check_validity(unix_seconds)?;
    let object_number = next_object_number(pdf_bytes);
    let prev_startxref = last_startxref(pdf_bytes)?;
    let root_ref = root_reference(pdf_bytes)?;
    // An incremental update's trailer replaces the previous one wholesale, so
    // /Info and /ID have to be carried forward explicitly or they are lost.
    let preserved = PreservedTrailer::parse(pdf_bytes);

    // Pass 1: measure the exact CMS length. Every element of the CMS has a
    // fixed-width encoding, so the length is independent of signature values.
    let cms_len = cades::sign_detached_at(b"", key, cert_der, rng, unix_seconds)?.len();
    let placeholder_len = 2 * cms_len; // lowercase hex, 2 chars per byte

    // Build the update with a zero placeholder to derive ByteRange offsets.
    // The final document is `original ++ update`; ByteRange offsets are
    // file-absolute.
    let objects = SignatureObjects::allocate(pdf_bytes, &root_ref, object_number)?;
    let (update_tail, contents_hex_start_rel) = build_update(
        pdf_bytes,
        &objects,
        placeholder_len,
        prev_startxref,
        &root_ref,
        &preserved,
        &signing_date(unix_seconds),
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
    // The CMS must be checked against the same instant `/M` claims, not the system
    // clock, or a caller signing with an explicit past time would be contradicted
    // by an inner check against now.
    let cms = cades::sign_detached_at(&signed_data, key, cert_der, rng, unix_seconds)?;
    if cms.len() != cms_len {
        return Err(DocError::invalid_format(
            "internal error: CMS length changed between passes",
        ));
    }
    let mut hex = String::with_capacity(cms.len() * 2);
    for byte in &cms {
        let _ = write!(hex, "{byte:02x}");
    }
    // Search only the appended update, never the original PDF body: a long run
    // of '0' bytes in the original (an uncompressed stream, say) would otherwise
    // be matched first and the hex spliced in over it, corrupting the file.
    let zeros = "0".repeat(placeholder_len);
    let zeros_at = find_bytes(&update[base..], zeros.as_bytes())
        .map(|rel| base + rel)
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

/// The current time in seconds since the Unix epoch.
///
/// The clock reading a pre-epoch system time falls back to 0 (1970-01-01).
#[must_use]
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Format `unix_seconds` as a PDF date string, `D:YYYYMMDDHHmmSS+00'00'`.
///
/// PDF 32000 §7.9.4 defines `D:YYYYMMDDHHmmSSOHH'mm'`, where `O` is the sign of
/// the UTC offset from local time. Only UTC is emitted, so the offset is always
/// `+00'00'`. The conversion is done with a civil-from-days algorithm (days since
/// the epoch to year/month/day) so no date library is needed for a single
/// timestamp.
fn signing_date(unix_seconds: i64) -> String {
    let (days, seconds_of_day) = {
        let day = 86_400i64;
        let days = unix_seconds.div_euclid(day);
        let rest = unix_seconds.rem_euclid(day);
        (days, rest)
    };
    let (year, month, day) = civil_from_days(days);
    let (hour, minute, second) = (
        seconds_of_day / 3600,
        (seconds_of_day % 3600) / 60,
        seconds_of_day % 60,
    );
    format!("D:{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}+00'00'")
}

/// Convert days since 1970-01-01 to a `(year, month, day)` civil date.
///
/// All three are returned as `i64` so the caller can format them directly; the
/// algorithm only ever produces values in `1..=12` and `1..=31`.
///
/// Howard Hinnant's `civil_from_days`, which shifts the era to 0000-03-01 so
/// the leap day lands at the end of the cycle.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
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

/// The offset of the last `startxref` in the file.
///
/// The trailer may be preceded by any amount of whitespace or binary content,
/// so the search covers the whole file rather than a fixed-size window: a
/// stream containing the literal text `startxref` would otherwise be mistaken
/// for the real one, and truncating the window would miss a trailer that is not
/// near the very end.
fn last_startxref(pdf_bytes: &[u8]) -> Result<u64, DocError> {
    let marker = b"startxref";
    let Some(relative) = pdf_bytes.windows(marker.len()).rposition(|w| w == marker) else {
        return Err(DocError::invalid_format("PDF has no startxref"));
    };
    let after = &pdf_bytes[relative + marker.len()..];
    let start = after
        .iter()
        .position(u8::is_ascii_digit)
        .ok_or_else(|| DocError::invalid_format("startxref has no offset"))?;
    let len = after[start..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    std::str::from_utf8(&after[start..start + len])
        .unwrap_or("")
        .parse::<u64>()
        .map_err(|e| DocError::invalid_format(format!("startxref unparsable: {e}")))
}

/// Trailer entries preserved from the previous trailer.
///
/// An incremental update's trailer replaces the previous one, so anything not
/// carried forward is lost: `/Info` (document metadata) and `/ID` (the file
/// identity that revision-tracking relies on) must be copied across.
#[derive(Debug, Default, PartialEq, Eq)]
struct PreservedTrailer {
    info: Option<String>,
    id: Option<String>,
}

impl PreservedTrailer {
    /// Parse the previous trailer's `/Info` and `/ID` from the raw file.
    ///
    /// Entries are read from the whole file because a PDF may carry several
    /// trailers from earlier incremental updates; the last one wins.
    fn parse(pdf_bytes: &[u8]) -> Self {
        let Some(trailer) = last_trailer_slice(pdf_bytes) else {
            return Self::default();
        };
        Self {
            info: trailer_entry(trailer, b"/Info"),
            id: trailer_entry(trailer, b"/ID"),
        }
    }

    /// Render the entries as trailer dictionary text, or an empty string when
    /// there is nothing to carry forward.
    fn render(&self) -> String {
        let mut out = String::new();
        if let Some(info) = &self.info {
            let _ = write!(out, " /Info {info}");
        }
        if let Some(id) = &self.id {
            let _ = write!(out, " /ID {id}");
        }
        out
    }
}

/// The bytes of the final `trailer` dictionary, including both delimiters.
///
/// The closing `>>` is found by matching nesting depth rather than by taking
/// the first `>`: `/ID [<aabb> <ccdd>]` contains hex strings whose `<…>` pairs
/// would otherwise truncate the trailer before the entries are read.
fn last_trailer_slice(pdf_bytes: &[u8]) -> Option<&[u8]> {
    let marker = b"trailer";
    let relative = pdf_bytes.windows(marker.len()).rposition(|w| w == marker)?;
    let rest = &pdf_bytes[relative + marker.len()..];
    let open = rest.windows(2).position(|w| w == b"<<")?;
    let mut depth = 0usize;
    let mut index = open;
    while index + 1 < rest.len() {
        match &rest[index..index + 2] {
            b"<<" => {
                depth += 1;
                index += 2;
            }
            b">>" => {
                depth -= 1;
                index += 2;
                if depth == 0 {
                    return Some(&rest[open..index]);
                }
            }
            _ => index += 1,
        }
    }
    None
}

/// Length of an indirect reference (`N G R`) at the start of `data`.
///
/// Returns `None` unless the text really is `digits whitespace digits
/// whitespace R`, so a value like `1` followed by an unrelated token is not
/// mistaken for a reference.
fn indirect_reference_len(data: &[u8]) -> Option<usize> {
    let number = |slice: &[u8]| -> Option<usize> {
        let len = slice.iter().take_while(|b| b.is_ascii_digit()).count();
        (len > 0).then_some(len)
    };
    let skip_space = |slice: &[u8], from: usize| -> Option<usize> {
        let len = slice[from..]
            .iter()
            .take_while(|b| b.is_ascii_whitespace())
            .count();
        Some(from + len)
    };

    let first = number(data)?;
    let second_start = skip_space(data, first)?;
    let second = number(&data[second_start..])?;
    let r_start = skip_space(data, second_start + second)?;
    let r_start = skip_space(data, r_start)?;
    (data.get(r_start) == Some(&b'R')).then_some(r_start + 1)
}

/// Extract a trailer entry value such as `12 0 R` or `[<aa> <bb>]`.
///
/// The returned text is the complete value including any indirect reference or
/// array, ready to be spliced into the new trailer.
fn trailer_entry(trailer: &[u8], key: &[u8]) -> Option<String> {
    let position = find_bytes(trailer, key)?;
    let rest = &trailer[position + key.len()..];
    // Skip the `=` of a key/value pair, if present.
    let rest = rest.strip_prefix(b" ").unwrap_or(rest);
    let rest = rest.strip_prefix(b" ").unwrap_or(rest);
    let value = if rest.first() == Some(&b'[') {
        // An array runs to its closing `]`.
        let end = find_bytes(rest, b"]")? + 1;
        &rest[..end]
    } else if rest.first().is_some_and(u8::is_ascii_digit) {
        // An indirect reference is `N G R`. The generation and the `R` are part
        // of the value, so the whole triple must be taken rather than stopping
        // at the first space.
        let end = indirect_reference_len(rest)?;
        &rest[..end]
    } else {
        // A name or a bare value runs to the next delimiter.
        let end = rest
            .iter()
            .position(|b| b.is_ascii_whitespace() || *b == b'>')?;
        &rest[..end]
    };
    let text = std::str::from_utf8(value).ok()?;
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

/// The `/Root` reference from the previous trailer, as `N G R`.
fn root_reference(pdf_bytes: &[u8]) -> Result<String, DocError> {
    let trailer = last_trailer_slice(pdf_bytes)
        .ok_or_else(|| DocError::invalid_format("PDF has no trailer"))?;
    trailer_entry(trailer, b"/Root")
        .ok_or_else(|| DocError::invalid_format("PDF trailer has no /Root"))
}

/// Object numbers allocated by the incremental update.
///
/// A `PAdES` signature is only discoverable by a conforming viewer if the
/// signature dictionary is reachable: it must be referenced by a signature
/// *field*, and that field must be listed in the catalog's `/AcroForm`.
/// Writing the `/Sig` object alone leaves a cryptographically valid but
/// effectively invisible signature, so all four objects are written.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SignatureObjects {
    /// The `/Type /Sig` dictionary.
    signature: u32,
    /// The signature field (`/FT /Sig`) merged with its widget annotation.
    field: u32,
    /// The `/AcroForm` dictionary holding `/Fields` and `/SigFlags`.
    acro_form: u32,
    /// The catalog object number, from the previous trailer's `/Root`.
    catalog: u32,
    /// The catalog's existing dictionary body, with its closing `>>` removed.
    catalog_body: Vec<u8>,
}

impl SignatureObjects {
    /// Allocate object numbers and capture the catalog body.
    ///
    /// `first_free` is one past the highest object number already in the file.
    fn allocate(pdf_bytes: &[u8], root_ref: &str, first_free: u32) -> Result<Self, DocError> {
        let catalog = root_object_number(root_ref)?;
        let catalog_body = catalog_body(pdf_bytes, catalog)?;
        Ok(Self {
            signature: first_free,
            field: first_free + 1,
            catalog,
            catalog_body,
            acro_form: first_free + 2,
        })
    }
}

/// The object number in an indirect reference such as `12 0 R`.
fn root_object_number(reference: &str) -> Result<u32, DocError> {
    reference
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| DocError::invalid_format(format!("unparsable /Root `{reference}`")))
}

/// The body of a PDF object's dictionary, with the enclosing
/// `N 0 obj` / `endobj` and the closing `>>` removed.
///
/// The caller appends its own entries and re-closes the dictionary, which is
/// how the catalog gains an `/AcroForm` reference.
fn catalog_body(pdf_bytes: &[u8], number: u32) -> Result<Vec<u8>, DocError> {
    let header = format!("{number} 0 obj");
    let start = find_bytes(pdf_bytes, header.as_bytes())
        .ok_or_else(|| DocError::invalid_format("catalog object not found"))?;
    let body_start = start + header.len();
    let end = find_bytes(&pdf_bytes[body_start..], b"endobj")
        .map(|rel| body_start + rel)
        .ok_or_else(|| DocError::invalid_format("catalog object has no endobj"))?;

    let region = &pdf_bytes[body_start..end];
    let open = region
        .iter()
        .position(|b| *b == b'<')
        .ok_or_else(|| DocError::invalid_format("catalog object is not a dictionary"))?;
    // Trim the trailing `>>` so entries can be appended before it.
    let trimmed = region
        .iter()
        .rposition(|b| *b == b'>')
        .filter(|last| region.get(last.wrapping_sub(1)) == Some(&b'>'))
        .map(|last| &region[..last - 1])
        .ok_or_else(|| DocError::invalid_format("catalog dictionary is not closed"))?;
    Ok(trimmed[open..].to_vec())
}

/// Build the incremental update: the signature, field, catalog, and `/AcroForm`
/// objects, a `/ByteRange [PLACEHOLDER]` marker and a zero-filled `/Contents`
/// hex placeholder. Returns the update and the offset of the first hex
/// character within it.
#[allow(clippy::too_many_arguments)]
fn build_update(
    pdf_bytes: &[u8],
    objects: &SignatureObjects,
    placeholder_len: usize,
    prev_startxref: u64,
    root_ref: &str,
    preserved: &PreservedTrailer,
    signing_date: &str,
) -> (Vec<u8>, usize) {
    let zeros = "0".repeat(placeholder_len);
    let mut update = Vec::new();
    update.push(b'\n');

    // Offsets are recorded as each object is written, since the xref table at
    // the end needs them all.
    let mut offsets: Vec<(u32, usize)> = Vec::new();

    // 1. The signature dictionary itself.
    offsets.push((objects.signature, pdf_bytes.len() + update.len()));
    update.extend_from_slice(format!("{} 0 obj\n", objects.signature).as_bytes());
    let dict_start = update.len();
    let dict_prefix = format!(
        "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached /M ({signing_date}) /ByteRange [PADDINGPLACEHOLDER] /Contents <"
    );
    update.extend_from_slice(dict_prefix.as_bytes());
    let contents_hex_start = dict_start + dict_prefix.len();
    update.extend_from_slice(zeros.as_bytes());
    update.extend_from_slice(b">>\nendobj\n");

    // 2. The signature field. Merged with the widget, as PDF 32000 §12.7.3.2
    //    allows a field and its widget to share one dictionary; keeping them
    //    together means fewer objects and no /Kids array to resolve.
    offsets.push((objects.field, pdf_bytes.len() + update.len()));
    // `/P` (the page the annotation belongs to) is deliberately omitted: the
    // page tree is not parsed here, and naming a guessed object number would
    // leave a dangling reference. It is optional for a signature widget, which
    // is invisible by virtue of its zero-area `/Rect`.
    update.extend_from_slice(
        format!(
            "{} 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Sig /T (Signature1) /Rect [0 0 0 0] /F 132 /V {} 0 R >>\nendobj\n",
            objects.field, objects.signature
        )
        .as_bytes(),
    );

    // 3. The catalog, redefined to reference the /AcroForm. An incremental
    //    update may replace an existing object, so the catalog is re-emitted
    //    with /AcroForm added to the entries it already had. Its number comes
    //    from the previous trailer's /Root, and its body is copied verbatim
    //    apart from that one added entry.
    offsets.push((objects.catalog, pdf_bytes.len() + update.len()));
    update.extend_from_slice(format!("{} 0 obj\n", objects.catalog).as_bytes());
    update.extend_from_slice(&objects.catalog_body);
    update
        .extend_from_slice(format!(" /AcroForm {} 0 R >>\nendobj\n", objects.acro_form).as_bytes());

    // 4. /AcroForm. /SigFlags 3 = SignaturesExist (bit 1) + AppendOnly (bit 2).
    //    Without AppendOnly a later update could silently drop or alter the
    //    signature, so a viewer treats its absence as a red flag.
    offsets.push((objects.acro_form, pdf_bytes.len() + update.len()));
    update.extend_from_slice(
        format!(
            "{} 0 obj\n<< /SigFlags 3 /Fields [{} 0 R] >>\nendobj\n",
            objects.acro_form, objects.field
        )
        .as_bytes(),
    );

    // New xref for the incremental update, chained via /Prev.
    let xref_offset = update.len();
    update.extend_from_slice(b"xref\n");
    // A subsection header declares a contiguous run `first first+count`, so the
    // entries are sorted by object number and grouped. The catalog is normally
    // object 1 while the new objects are numbered after everything in the file,
    // which is two separate runs rather than one.
    offsets.sort_by_key(|(number, _)| *number);
    let mut run_start = 0usize;
    while run_start < offsets.len() {
        let first = offsets[run_start].0;
        let mut run_end = run_start + 1;
        while run_end < offsets.len() && offsets[run_end].0 == offsets[run_end - 1].0 + 1 {
            run_end += 1;
        }
        let count = run_end - run_start;
        update.extend_from_slice(format!("{first} {count}\n").as_bytes());
        for (_, offset) in &offsets[run_start..run_end] {
            // Exactly 20 bytes: 10-digit offset, space, 5-digit generation,
            // space, `n`, then the 2-byte EOL marker. A trailing space here
            // would make every entry 21 bytes and shift the whole table.
            update.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
        }
        run_start = run_end;
    }
    // /Size is one past the highest object number in the file, including the
    // catalog that was redefined above.
    let size = offsets.iter().map(|(number, _)| *number).max().unwrap_or(0) + 1;
    // The new trailer replaces the previous one, so the preserved entries are
    // spliced in before `/Prev`.
    update.extend_from_slice(
        format!(
            "trailer\n<< /Size {size} /Root {root_ref}{} /Prev {prev_startxref} >>\nstartxref\n{xref_offset}\n%%EOF\n",
            preserved.render(),
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

    /// `/M` must be the actual signing time. A hardcoded date would make every
    /// signature claim the same instant, which for an audit-trail format is the one
    /// field a verifier most relies on.
    #[test]
    fn the_m_entry_records_the_requested_signing_time() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        // 2026-10-01T00:00:00Z and 2026-12-15T12:34:56Z. Both are inside the test
        // certificate's validity window, which starts 2026-09-30.
        let first = sign_at(
            &pdf_with_trailer_entries(None, None),
            &key,
            &cert,
            &rng,
            1_790_812_800,
        )
        .expect("sign");
        let second = sign_at(
            &pdf_with_trailer_entries(None, None),
            &key,
            &cert,
            &rng,
            1_797_338_096,
        )
        .expect("sign");

        let text = String::from_utf8_lossy(&first);
        assert!(
            text.contains("/M (D:20261001000000+00'00')"),
            "expected the requested date, got: {:?}",
            text.find("/M (").map(|i| &text[i..i + 30])
        );
        let text = String::from_utf8_lossy(&second);
        assert!(
            text.contains("/M (D:20261215123456+00'00')"),
            "expected the requested date, got: {:?}",
            text.find("/M (").map(|i| &text[i..i + 30])
        );
    }

    /// The date conversion is hand-rolled, so it is checked against known epochs
    /// including a leap day and the pre-2000 era.
    #[test]
    fn signing_date_converts_known_instants() {
        assert_eq!(signing_date(0), "D:19700101000000+00'00'");
        // 2000-02-29T12:00:00Z — a leap day.
        assert_eq!(signing_date(951_825_600), "D:20000229120000+00'00'");
        // 2026-01-01T00:00:00Z.
        assert_eq!(signing_date(1_767_225_600), "D:20260101000000+00'00'");
        // 2026-06-15T12:34:56Z.
        assert_eq!(signing_date(1_781_526_896), "D:20260615123456+00'00'");
        // 2038-01-19T03:14:07Z, the classic 32-bit time_t boundary.
        assert_eq!(signing_date(2_147_483_647), "D:20380119031407+00'00'");
    }

    #[test]
    fn civil_from_days_matches_the_epoch_and_a_leap_day() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        // 2024-02-29 is day 59 of a leap year.
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }

    /// The certificate's validity window is enforced at signing time, so an expired
    /// certificate cannot produce a signature that no verifier will accept.
    #[test]
    fn an_expired_certificate_fails_at_signing_time() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let certificate = crate::pem::Certificate::from_der(&cert).expect("cert");
        let rng = ring::rand::SystemRandom::new();

        let err = sign_at(
            &pdf_with_trailer_entries(None, None),
            &key,
            &cert,
            &rng,
            certificate.not_after() + 1,
        )
        .expect_err("expired certificate must fail");
        assert!(err.to_string().contains("expired"), "{err}");

        let err = sign_at(
            &pdf_with_trailer_entries(None, None),
            &key,
            &cert,
            &rng,
            certificate.not_before() - 1,
        )
        .expect_err("not-yet-valid certificate must fail");
        assert!(err.to_string().contains("not valid until"), "{err}");

        // Inside the window it still signs.
        assert!(
            sign_at(
                &pdf_with_trailer_entries(None, None),
                &key,
                &cert,
                &rng,
                certificate.not_before(),
            )
            .is_ok(),
            "notBefore is inclusive"
        );
    }

    /// `sign_at` at a fixed time must be byte-identical to `sign` at that time,
    /// which is what makes signatures reproducible in tests and builds.
    #[test]
    fn sign_and_sign_at_agree_for_the_same_instant() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        let instant = 1_800_000_000;
        let a = sign_at(
            &pdf_with_trailer_entries(None, None),
            &key,
            &cert,
            &rng,
            instant,
        )
        .expect("a");
        let b = sign(&pdf_with_trailer_entries(None, None), &key, &cert, &rng).expect("b");
        // `sign` uses the clock, so the bytes may differ, but the structure and
        // the certificate check must behave identically.
        assert_eq!(a.len(), b.len(), "same structure");
        assert!(String::from_utf8_lossy(&b).contains("/SubFilter /ETSI.CAdES.detached"));
    }

    /// A `/Sig` dictionary alone is cryptographically valid but effectively
    /// invisible: a viewer discovers signatures by walking `/AcroForm /Fields`.
    /// All four objects must be present and cross-referenced.
    #[test]
    fn the_signature_is_reachable_through_acroform() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        let signed = sign(&pdf_with_trailer_entries(None, None), &key, &cert, &rng).expect("sign");
        let text = String::from_utf8_lossy(&signed);

        // The signature dictionary.
        assert!(text.contains("/Type /Sig"), "signature dictionary");
        assert!(
            text.contains("/SubFilter /ETSI.CAdES.detached"),
            "subfilter"
        );

        // The field, merged with its widget, pointing back at the signature.
        assert!(text.contains("/FT /Sig"), "signature field type");
        assert!(text.contains("/Subtype /Widget"), "widget annotation");
        assert!(text.contains("/V "), "field must reference /V");

        // /AcroForm with /SigFlags and the field listed in /Fields.
        assert!(
            text.contains("/AcroForm"),
            "catalog must reference AcroForm"
        );
        assert!(
            text.contains("/SigFlags 3"),
            "SigFlags: exists + append-only"
        );
        assert!(text.contains("/Fields ["), "AcroForm must list fields");
    }

    /// `/Size` must exceed every object number in the file, including the
    /// catalog redefined by the update. Too small and readers reject the file.
    #[test]
    fn size_covers_every_written_object() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        let signed = sign(&pdf_with_trailer_entries(None, None), &key, &cert, &rng).expect("sign");
        let text = String::from_utf8_lossy(&signed);
        let last_trailer = text.rfind("trailer").expect("trailer");
        let trailer = &text[last_trailer..];
        let size: u32 = trailer
            .split("/Size ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|n| n.parse().ok())
            .expect("/Size");

        let highest = text
            .match_indices(" 0 obj")
            .map(|(index, _)| {
                let mut start = index;
                while start > 0 && text.as_bytes()[start - 1].is_ascii_digit() && index - start < 10
                {
                    start -= 1;
                }
                text[start..index].parse::<u32>().unwrap_or(0)
            })
            .max()
            .expect("objects present");
        assert!(
            size > highest,
            "/Size {size} must exceed the highest object number {highest}"
        );
    }

    /// Every xref entry must point at the real start of its object, or readers
    /// resolve garbage.
    #[test]
    fn every_xref_entry_points_at_its_object() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        let signed = sign(&pdf_with_trailer_entries(None, None), &key, &cert, &rng).expect("sign");

        // Walk the final xref table, following each run.
        let text = String::from_utf8_lossy(&signed);
        let start = text.rfind("\nxref\n").map(|i| i + 6).expect("xref");
        let body = &signed[start..];
        let mut checked = 0;
        let mut cursor = 0usize;
        while let Some(newline) = body[cursor..].iter().position(|b| *b == b'\n') {
            let line = &body[cursor..cursor + newline];
            cursor += newline + 1;
            let line = std::str::from_utf8(line).unwrap_or("").trim();
            let mut fields = line.split_whitespace();
            let (Some(first), Some(count)) = (fields.next(), fields.next()) else {
                continue;
            };
            if fields.next().is_some() {
                continue; // an entry, not a run header
            }
            let first: u32 = first.parse().expect("run start");
            let count: u32 = count.parse().expect("run count");
            for index in 0..count {
                let entry_start = cursor + (index as usize) * 20;
                let entry = &body[entry_start..entry_start + 20];
                assert_eq!(entry.len(), 20, "xref entries are 20 bytes");
                let offset: usize = std::str::from_utf8(&entry[..10])
                    .expect("ascii")
                    .trim()
                    .parse()
                    .expect("offset");
                let number = first + index;
                let expected = format!("{number} 0 obj");
                let actual = signed
                    .get(offset..offset + expected.len())
                    .unwrap_or_default();
                assert_eq!(
                    actual,
                    expected.as_bytes(),
                    "xref entry for object {number} points at the wrong offset"
                );
                checked += 1;
            }
            cursor += (count as usize) * 20;
        }
        assert!(checked >= 3, "expected several xref entries, saw {checked}");
    }

    /// A minimal but valid PDF with an `/Info` and `/ID` in its trailer.
    fn pdf_with_trailer_entries(info: Option<&str>, id: Option<&str>) -> Vec<u8> {
        let mut extra = String::new();
        if let Some(info) = info {
            let _ = write!(extra, " /Info {info}");
        }
        if let Some(id) = id {
            let _ = write!(extra, " /ID {id}");
        }
        format!(
            "%PDF-1.7\n\
             1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n\
             xref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n\
             trailer\n<< /Size 3 /Root 1 0 R{extra} >>\nstartxref\n109\n%%EOF\n"
        )
        .into_bytes()
    }

    /// An incremental update replaces the trailer wholesale, so `/Info` and
    /// `/ID` must be carried forward or document metadata is lost on signing.
    #[test]
    fn signing_preserves_info_and_id_from_the_previous_trailer() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        let original = pdf_with_trailer_entries(Some("7 0 R"), Some("[<aabb> <ccdd>]"));

        let signed = sign(&original, &key, &cert, &rng).expect("sign");

        let text = String::from_utf8_lossy(&signed);
        let last_trailer = text.rfind("trailer").expect("trailer written");
        let trailer = &text[last_trailer..];
        assert!(trailer.contains("/Info 7 0 R"), "{trailer}");
        assert!(trailer.contains("/ID [<aabb> <ccdd>]"), "{trailer}");
    }

    /// A PDF with neither entry must not gain spurious ones.
    #[test]
    fn absent_trailer_entries_are_not_invented() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();
        let signed = sign(&pdf_with_trailer_entries(None, None), &key, &cert, &rng).expect("sign");

        let text = String::from_utf8_lossy(&signed);
        let last_trailer = text.rfind("trailer").expect("trailer written");
        let trailer = &text[last_trailer..];
        assert!(!trailer.contains("/Info"), "{trailer}");
        assert!(!trailer.contains("/ID"), "{trailer}");
    }

    /// A hex string inside the trailer contains its own `>` (`/ID [<aabb> <ccdd>]`),
    /// so closing the dictionary on the first `>` truncates it and the entries are
    /// lost.
    #[test]
    fn a_hex_string_in_the_trailer_does_not_truncate_it() {
        let pdf = pdf_with_trailer_entries(Some("7 0 R"), Some("[<aabb> <ccdd>]"));
        let trailer = last_trailer_slice(&pdf).expect("trailer");
        let text = std::str::from_utf8(trailer).expect("ascii");
        assert_eq!(
            text,
            "<< /Size 3 /Root 1 0 R /Info 7 0 R /ID [<aabb> <ccdd>] >>"
        );
        assert_eq!(
            trailer_entry(trailer, b"/ID").as_deref(),
            Some("[<aabb> <ccdd>]")
        );
    }

    #[test]
    fn trailer_entry_parsing_handles_references_and_arrays() {
        let trailer = b"<< /Size 3 /Root 1 0 R /Info 7 0 R /ID [<aabb> <ccdd>] >>";
        assert_eq!(
            trailer_entry(trailer, b"/Root").as_deref(),
            Some("1 0 R"),
            "indirect reference"
        );
        assert_eq!(
            trailer_entry(trailer, b"/ID").as_deref(),
            Some("[<aabb> <ccdd>]"),
            "array"
        );
        assert_eq!(trailer_entry(trailer, b"/Encrypt"), None, "absent");
    }

    /// The literal text `startxref` can appear inside a stream. Only the last
    /// occurrence is the real one.
    #[test]
    fn last_startxref_ignores_earlier_decoys() {
        let mut pdf = pdf_with_trailer_entries(None, None);
        // Insert a decoy `startxref\n99` well before the real trailer.
        let decoy = b"1 0 obj\n<< /Note (startxref 99) >>\nendobj\n";
        let insert_at = find_bytes(&pdf, b"xref\n").expect("xref");
        pdf.splice(insert_at..insert_at, decoy.iter().copied());
        assert_eq!(
            last_startxref(&pdf).expect("real startxref"),
            109,
            "must pick the final startxref, not the decoy"
        );
    }

    /// The `/Contents` placeholder must be matched inside the appended update.
    /// A run of '0' bytes in the original body used to be found first, and the
    /// CMS hex spliced over it, corrupting the signed file.
    #[test]
    fn a_zero_run_in_the_original_body_is_not_the_placeholder() {
        let key = crate::test_support::test_key();
        let cert = crate::test_support::test_cert_der();
        let rng = ring::rand::SystemRandom::new();

        // A long zero run, comfortably longer than the CMS placeholder.
        let mut original = pdf_with_trailer_entries(None, None);
        let zeros = "0".repeat(8192);
        let insert_at = find_bytes(&original, b"xref\n").expect("xref");
        original.splice(
            insert_at..insert_at,
            format!("1 0 obj\n<< /Note ({zeros}) >>\nendobj\n").into_bytes(),
        );

        let signed = sign(&original, &key, &cert, &rng).expect("sign");
        assert!(signed.starts_with(&original), "original must be a prefix");
        // The note object must survive byte-for-byte.
        assert!(
            signed.windows(8192).any(|w| w == zeros.as_bytes()),
            "the original zero run must be untouched"
        );
    }
}

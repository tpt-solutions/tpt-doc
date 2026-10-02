//! Integration tests: byte-exact xref verification and deterministic output.
//!
//! A miniature PDF reader in these tests walks the generated file exactly as
//! a spec-compliant viewer would: from `startxref` to the xref table, from
//! each entry to its object, and through the trailer.

use flate2::read::ZlibDecoder;
use std::io::{Read, Write};
use tpt_doc_pdf::{Document, Font, Image, Page};

const HEADER: &[u8] = b"%PDF-1.7\n";

/// CRC-32 (IEEE) for building PNG chunks in tests.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Build a minimal 1×1 white RGB PNG with a real zlib-compressed IDAT.
fn tiny_png() -> Vec<u8> {
    // One scanline: filter None + RGB white.
    let raw = [0u8, 0xFF, 0xFF, 0xFF];
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&raw).expect("deflate raw pixel");
    let idat = encoder.finish().expect("finish deflate");

    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let ihdr = {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&1u32.to_be_bytes());
        chunk.extend_from_slice(&1u32.to_be_bytes());
        chunk.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB, no interlace
        chunk
    };
    for (kind, data) in [(&b"IHDR"[..], &ihdr[..]), (&b"IDAT"[..], &idat[..])] {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        png.extend_from_slice(kind);
        png.extend_from_slice(data);
        let mut crc_input = kind.to_vec();
        crc_input.extend_from_slice(data);
        png.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&crc32(b"IEND").to_be_bytes());
    png
}

fn sample_document() -> Document {
    let mut doc = Document::new();
    let helvetica = doc.embed_builtin_font(Font::Helvetica);
    let times = doc.embed_builtin_font(Font::TimesRoman);

    let mut page = Page::a4();
    page.text("Quarterly Report", helvetica, 18.0, (72.0, 770.0));
    page.text(
        "Prepared (carefully) by TPT \\ Solutions",
        helvetica,
        11.0,
        (72.0, 740.0),
    );
    doc.add_page(page);

    let mut page2 = Page::letter();
    page2.text("Times on page two", times, 12.0, (72.0, 700.0));
    let image = Image::from_png(&tiny_png()).expect("tiny png");
    let image_id = doc.embed_image(image);
    page2.image(image_id, 100.0, 400.0, 144.0, 144.0);
    doc.add_page(page2);
    doc
}

/// Locate `needle` and return the byte offset.
fn find(bytes: &[u8], needle: &[u8]) -> usize {
    bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .unwrap_or_else(|| panic!("pattern {needle:?} not found"))
}

#[test]
fn generated_pdf_is_spec_walkable() {
    let bytes = sample_document().write().expect("write");

    // 1. Header and EOF marker.
    assert!(bytes.starts_with(HEADER));
    assert!(bytes.windows(5).any(|w| w == b"%%EOF"));

    // 2. startxref points at the xref table. It is read from the END of the
    // file (the trailer is last); compressed streams can coincidentally
    // contain the byte sequence, so the first occurrence is not reliable.
    let trailer_marker = bytes
        .windows(b"startxref\n".len())
        .rposition(|w| w == b"startxref\n")
        .expect("startxref present");
    let startxref = trailer_marker + b"startxref\n".len();
    let offset_str: String = bytes[startxref..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .map(|b| char::from(*b))
        .collect();
    let xref_offset: usize = offset_str.parse().expect("startxref offset");
    assert_eq!(&bytes[xref_offset..xref_offset + 5], b"xref\n");

    // 3. Every xref entry points at "<id> 0 obj".
    let count_line_start = xref_offset + 5;
    let count_line_end = count_line_start
        + bytes[count_line_start..]
            .iter()
            .position(|b| *b == b'\n')
            .expect("count line");
    let count_str = std::str::from_utf8(&bytes[count_line_start..count_line_end])
        .expect("ASCII count")
        .trim();
    let entry_count: usize = count_str
        .split_whitespace()
        .last()
        .expect("entry count")
        .parse()
        .expect("numeric count");
    let entries_start = count_line_end + 1;
    for id in 1..entry_count as u32 {
        let entry =
            &bytes[entries_start + id as usize * 20..entries_start + (id as usize + 1) * 20];
        let entry_str = std::str::from_utf8(entry).expect("ASCII entry");
        let obj_offset: usize = entry_str[..10].trim().parse().expect("entry offset");
        let prefix = format!("{id} 0 obj");
        assert_eq!(
            &bytes[obj_offset..obj_offset + prefix.len()],
            prefix.as_bytes(),
            "xref entry for object {id} is not byte-exact"
        );
    }

    // 4. The trailer references the catalog.
    let trailer = find(&bytes, b"/Root 1 0 R");
    assert!(trailer > 0);
}

#[test]
fn page_one_text_survives_content_stream_compression() {
    let bytes = sample_document().write().expect("write");
    // Walk every FlateDecode stream dictionary and decompress each stream;
    // page 1's content is the one containing its text operators. (Marker
    // bytes can appear inside compressed data, so position alone is not
    // trusted — a stream is only accepted if it decompresses cleanly.)
    let needle: &[u8] = b"/Filter /FlateDecode >>\nstream\n";
    let mut found_text: Option<String> = None;
    let mut cursor = 0usize;
    while let Some(rel) = bytes[cursor..]
        .windows(needle.len())
        .position(|w| w == needle)
    {
        let marker = cursor + rel + needle.len();
        let end = marker
            + bytes[marker..]
                .windows(10)
                .position(|w| w == b"\nendstream")
                .expect("endstream");
        let mut decompressed = Vec::new();
        if ZlibDecoder::new(&bytes[marker..end])
            .read_to_end(&mut decompressed)
            .is_ok()
            && let Ok(text) = String::from_utf8(decompressed)
            && text.contains("Quarterly Report")
        {
            found_text = Some(text);
            break;
        }
        cursor = marker;
    }
    let text = found_text.expect("page 1 content stream found");
    assert!(text.contains("(Quarterly Report) Tj"), "{text:?}");
    assert!(
        text.contains("Prepared \\(carefully\\) by TPT \\\\ Solutions"),
        "{text:?}"
    );
}

#[test]
fn attached_file_is_embedded_and_registered() {
    let mut doc = Document::new();
    let font = doc.embed_builtin_font(Font::Helvetica);
    let mut page = Page::a4();
    page.text("with attachment", font, 12.0, (72.0, 700.0));
    doc.add_page(page);
    doc.attach_file("factur-x.xml", "text/xml", b"<Invoice/>".to_vec());
    let bytes = doc.write().expect("write");

    let text = String::from_utf8_lossy(&bytes).into_owned();
    // Catalog registers /AF and the EmbeddedFiles name tree.
    assert!(text.contains("/AF ["), "catalog /AF missing");
    assert!(text.contains("/EmbeddedFiles"), "name tree missing");
    assert!(text.contains("(factur-x.xml)"), "filespec name missing");
    assert!(text.contains("/AFRelationship /Data"));
    // The payload bytes are embedded verbatim.
    assert!(bytes.windows(10).any(|w| w == b"<Invoice/>"));
}

/// Parse the xref table and return each object's body, keyed by object number.
/// Walks the file the way a viewer does: `startxref` -> xref -> objects.
fn object_bodies(bytes: &[u8]) -> std::collections::BTreeMap<u32, String> {
    let startxref = find(bytes, b"startxref") + "startxref".len();
    let mut cursor = startxref;
    while cursor < bytes.len() && (bytes[cursor] as char).is_whitespace() {
        cursor += 1;
    }
    let mut xref_offset = 0usize;
    for ch in bytes[cursor..].iter().take_while(|b| b.is_ascii_digit()) {
        xref_offset = xref_offset * 10 + usize::from(ch - b'0');
    }

    let mut objects = std::collections::BTreeMap::new();
    let mut pos = xref_offset + "xref".len();
    // Skip the `xref` keyword line, then read the subsection header
    // `first count` — the per-line entries carry no object id of their own.
    pos += bytes[pos..]
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or_else(|| panic!("malformed xref keyword line"));
    pos += 1;
    let header_end = find(&bytes[pos..], b"\n") + pos;
    let header = std::str::from_utf8(&bytes[pos..header_end]).expect("xref subsection header");
    let mut header_fields = header.split_whitespace();
    let next_id: u32 = header_fields.next().expect("first object").parse().expect("id");
    let count: u32 = header_fields.next().expect("count").parse().expect("count");
    pos = header_end + 1;

    for index in 0..count {
        let line_end = find(&bytes[pos..], b"\n") + pos;
        let entry = std::str::from_utf8(&bytes[pos..line_end]).expect("xref entry");
        // Each fixed-width entry is `oooooooooo ggggg n`.
        let mut fields = entry.split_whitespace();
        let offset: usize = fields.next().expect("offset").parse().expect("numeric offset");
        let _generation = fields.next().expect("generation");
        let in_use = fields.next().expect("in-use flag") == "n";
        pos = line_end + 1;

        // Object 0 is the free-list head and points at no object body.
        if in_use {
            let id = next_id + index;
            let header_text = format!("{id} 0 obj");
            let header_at = find(&bytes[offset..], header_text.as_bytes()) + offset;
            let body_start = header_at + header_text.len();
            let body_end = find(&bytes[body_start..], b"endobj") + body_start;
            objects.insert(
                id,
                String::from_utf8_lossy(&bytes[body_start..body_end]).into_owned(),
            );
        }
    }
    objects
}

/// Resolve every `N 0 R` indirect reference in `body`, in order of appearance.
fn indirect_refs(body: &str) -> Vec<u32> {
    let bytes = body.as_bytes();
    let mut refs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let mut j = i;
        while j < bytes.len() && bytes[j] == b' ' {
            j += 1;
        }
        if bytes[j..].starts_with(b"0 R") {
            if let Ok(id) = std::str::from_utf8(&bytes[start..i]).unwrap().parse::<u32>() {
                refs.push(id);
            }
            i = j + 3;
            continue;
        }
        i = j;
    }
    refs
}

/// Extract the `/EF << ... >>` dictionary body from a filespec.
fn ef_dictionary(filespec: &str) -> String {
    filespec
        .split("/EF <<")
        .nth(1)
        .and_then(|rest| rest.split(">>").next())
        .unwrap_or_else(|| panic!("/EF dictionary missing from {filespec:?}"))
        .to_owned()
}

/// Object ids whose body contains `marker`.
fn ids_containing(objects: &std::collections::BTreeMap<u32, String>, marker: &str) -> Vec<u32> {
    objects
        .iter()
        .filter(|(_, body)| body.contains(marker))
        .map(|(id, _)| *id)
        .collect()
}

#[test]
fn filespec_ef_points_at_the_embedded_file_stream_not_itself() {
    let mut doc = Document::new();
    doc.attach_file("factur-x.xml", "text/xml", b"<Invoice/>".to_vec());
    let bytes = doc.write().expect("write");

    let objects = object_bodies(&bytes);
    let filespecs = ids_containing(&objects, "/Type /Filespec");
    let streams = ids_containing(&objects, "/Type /EmbeddedFile");
    assert_eq!(filespecs.len(), 1, "one filespec");
    assert_eq!(streams.len(), 1, "one embedded file stream");
    let (filespec_id, stream_id) = (filespecs[0], streams[0]);

    assert_ne!(
        filespec_id, stream_id,
        "the embedded file stream must be its own object"
    );

    // /F and /UF must both resolve to the stream object.
    let ef = ef_dictionary(&objects[&filespec_id]);
    let ef_refs = indirect_refs(&ef);
    assert_eq!(
        ef_refs,
        vec![stream_id, stream_id],
        "/F and /UF must both reference stream {stream_id}, got {ef_refs:?} in {ef:?}"
    );
    assert!(
        !ef_refs.contains(&filespec_id),
        "/EF self-references the filespec object {filespec_id}: {ef:?}"
    );
    assert!(
        objects[&stream_id].contains("<Invoice/>"),
        "object {stream_id} should hold the payload"
    );
}

#[test]
fn multiple_attachments_each_reference_their_own_stream() {
    let mut doc = Document::new();
    doc.attach_file("first.xml", "text/xml", b"<First/>".to_vec());
    doc.attach_file("second.json", "application/json", b"{\"second\":1}".to_vec());
    let bytes = doc.write().expect("write");

    let objects = object_bodies(&bytes);
    let filespecs = ids_containing(&objects, "/Type /Filespec");
    let streams = ids_containing(&objects, "/Type /EmbeddedFile");
    assert_eq!(filespecs.len(), 2);
    assert_eq!(streams.len(), 2);

    let mut referenced = Vec::new();
    for filespec_id in &filespecs {
        let ef = ef_dictionary(&objects[filespec_id]);
        let mut refs = indirect_refs(&ef);
        assert_eq!(refs.len(), 2, "/F and /UF both present in {ef:?}");
        refs.dedup();
        assert_eq!(refs.len(), 1, "/F and /UF must agree: {ef:?}");
        let target = refs[0];
        assert!(streams.contains(&target), "/EF target {target} is not a stream");
        assert_ne!(
            target, *filespec_id,
            "filespec {filespec_id} must not reference itself"
        );
        referenced.push(target);
    }
    referenced.sort_unstable();
    referenced.dedup();
    assert_eq!(
        referenced.len(),
        2,
        "two filespecs must reference two different streams, got {referenced:?}"
    );
}

#[test]
fn catalog_af_and_name_tree_reference_the_filespecs() {
    let mut doc = Document::new();
    doc.attach_file("a.xml", "text/xml", b"<A/>".to_vec());
    doc.attach_file("b.xml", "text/xml", b"<B/>".to_vec());
    let bytes = doc.write().expect("write");

    let objects = object_bodies(&bytes);
    let catalog = objects
        .get(&1)
        .unwrap_or_else(|| panic!("object 1 missing; got {:?}", objects.keys()));
    assert!(catalog.contains("/Type /Catalog"));

    let mut expected = ids_containing(&objects, "/Type /Filespec");
    expected.sort_unstable();
    assert_eq!(expected.len(), 2);

    let af = catalog
        .split("/AF [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .unwrap_or_else(|| panic!("/AF missing from {catalog:?}"));
    let mut af_ids = indirect_refs(af);
    af_ids.sort_unstable();
    assert_eq!(af_ids, expected, "/AF must list both filespecs");

    let names = catalog
        .split("/EmbeddedFiles << /Names [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .unwrap_or_else(|| panic!("/EmbeddedFiles name tree missing from {catalog:?}"));
    let mut name_ids = indirect_refs(names);
    name_ids.sort_unstable();
    assert_eq!(name_ids, expected, "name tree must list both filespecs");

    assert!(catalog.contains("(a.xml)"), "missing name: {catalog:?}");
    assert!(catalog.contains("(b.xml)"), "missing name: {catalog:?}");
}

#[test]
fn deterministic_output_is_byte_identical() {
    let a = sample_document().write().expect("write a");
    let b = sample_document().write().expect("write b");
    assert_eq!(a, b, "identical documents must serialize identically");
}

#[test]
fn deterministic_structure_snapshot() {
    let bytes = sample_document().write().expect("write");
    // Snapshot the document skeleton: object header lines and stream
    // dictionaries — deterministic and human-reviewable, unlike the raw
    // bytes with their compressed streams.
    let mut skeleton = String::new();
    for (index, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
        if line.ends_with(" 0 obj")
            || line.starts_with("<< /Type")
            || line.starts_with("<< /Length")
        {
            skeleton.push_str(&format!("{index}: {line}\n"));
        }
    }
    insta::assert_snapshot!(skeleton);
}

#[test]
fn image_data_is_embedded_verbatim_or_transcoded() {
    let mut doc = Document::new();
    let jpeg = Image::from_jpeg(vec![
        0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0E, 0x08, 0x00, 0x02, 0x00, 0x03, 0x03,
    ])
    .expect("jpeg dims");
    assert_eq!(jpeg.filter, tpt_doc_pdf::image::ImageFilter::DctDecode);
    let png = Image::from_png(&tiny_png()).expect("png");
    assert_eq!(png.width, 1);
    assert_eq!(png.height, 1);
    let _image_id = doc.embed_image(jpeg);
    let _ = doc.embed_image(png);
    let bytes = doc.write().expect("write");
    // The JPEG SOI bytes appear in the output (passthrough).
    assert!(bytes.windows(2).any(|w| w == [0xFF, 0xD8]));
}

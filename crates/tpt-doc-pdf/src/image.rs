//! Raster image embedding: JPEG passthrough and PNG transcoding.
//!
//! JPEG streams are embedded verbatim (`/DCTDecode`). PNG streams are
//! decoded (inflate + unfilter) and re-deflated into a raw RGB stream
//! (`/FlateDecode`) so no PNG-specific filters leak into the PDF.

use std::io::{Read, Write as _};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use tpt_doc_core::DocError;

/// The stream filter of an embedded image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFilter {
    /// JPEG data, embedded verbatim.
    DctDecode,
    /// zlib-deflated raw RGB samples.
    FlateDecode,
}

/// An embeddable raster image.
#[derive(Debug, Clone)]
pub struct Image {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// The encoded stream (already in the layout given by `filter`).
    pub data: Vec<u8>,
    /// How `data` is compressed.
    pub filter: ImageFilter,
}

impl Image {
    /// Embed a JPEG image, parsing its dimensions from the SOF marker.
    ///
    /// The bytes are not re-encoded: the JPEG stream is embedded directly
    /// with a `/DCTDecode` filter.
    ///
    /// # Errors
    /// Returns [`DocError`] if the bytes are not a JPEG stream or no
    /// SOF (frame header) marker is present.
    pub fn from_jpeg(bytes: Vec<u8>) -> Result<Self, DocError> {
        if bytes.first() != Some(&0xFF) || bytes.get(1) != Some(&0xD8) {
            return Err(DocError::invalid_format("not a JPEG stream (bad SOI)"));
        }
        let (width, height) = parse_jpeg_dimensions(&bytes)
            .ok_or_else(|| DocError::invalid_format("no JPEG SOF marker found"))?;
        Ok(Self {
            width,
            height,
            data: bytes,
            filter: ImageFilter::DctDecode,
        })
    }

    /// Decode a PNG image into a raw RGB stream.
///
/// Supports non-interlaced 8-bit images: grayscale, RGB, palette, and
/// RGBA. The decoded samples are re-deflated for a `/FlateDecode`
/// `/XObject`.
///
/// # Errors
/// Returns [`DocError`] if the bytes are not a supported PNG stream.
pub fn from_png(bytes: &[u8]) -> Result<Self, DocError> {
        const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        if bytes.len() < 8 || bytes[..8] != SIGNATURE {
            return Err(DocError::invalid_format("not a PNG stream (bad signature)"));
        }

        let mut width = 0u32;
        let mut height = 0u32;
        let mut color_type = 0u8;
        let mut palette: Vec<[u8; 3]> = Vec::new();
        let mut idat: Vec<u8> = Vec::new();

        let mut cursor = &bytes[8..];
        while cursor.len() >= 8 {
            // `8 + len + 4` overflows on a hostile 32-bit length field, so the
            // chunk size is checked before it is used for slicing.
            let len = u32::from_be_bytes([cursor[0], cursor[1], cursor[2], cursor[3]]);
            let len = usize::try_from(len).unwrap_or(usize::MAX);
            let kind = &cursor[4..8];
            // The 4-byte CRC follows the chunk data.
            let end = 8usize.checked_add(len).and_then(|v| v.checked_add(4));
            let Some(end) = end else {
                return Err(DocError::invalid_format("PNG chunk length overflows"));
            };
            let chunk = cursor
                .get(..end)
                .ok_or_else(|| DocError::invalid_format("truncated PNG chunk"))?;
            let data = &chunk[8..8 + len];
            // Verify the CRC over the chunk type and data. A mismatch means
            // the file is corrupt or was not a PNG to begin with, and decoding
            // it would silently produce a wrong image.
            let declared = u32::from_be_bytes([chunk[8 + len], chunk[9 + len], chunk[10 + len], chunk[11 + len]]);
            let actual = png_crc32(&chunk[4..8 + len]);
            if declared != actual {
                return Err(DocError::invalid_format(format!(
                    "PNG chunk {} has CRC {declared:#010x}, expected {actual:#010x}",
                    String::from_utf8_lossy(kind)
                )));
            }
            match kind {
                b"IHDR" => {
                    let (w, h, c) = parse_ihdr(data)?;
                    width = w;
                    height = h;
                    color_type = c;
                }
                b"PLTE" => {
                    palette = data.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                }
                b"IDAT" => idat.extend_from_slice(data),
                b"IEND" => break,
                _ => {}
            }
            cursor = &cursor[end..];
        }

        // `parse_ihdr` already validated the dimensions and the declared size.
        let channels = match color_type {
            0 | 3 => 1usize,
            2 => 3,
            6 => 4,
            other => {
                return Err(DocError::invalid_format(format!(
                    "unsupported PNG color type {other}"
                )));
            }
        };
        if idat.is_empty() {
            return Err(DocError::invalid_format("PNG has no IDAT data"));
        }

        let mut raw = Vec::new();
        ZlibDecoder::new(idat.as_slice())
            .read_to_end(&mut raw)
            .map_err(|e| DocError::invalid_format(format!("PNG IDAT decompression failed: {e}")))?;

        let stride = width as usize * channels;
        let filtered = unfilter(&raw, height as usize, stride, channels)?;
        #[allow(clippy::cast_possible_truncation)] // dimensions come from 32-bit PNG fields
        let (w, h) = (width as usize, height as usize);
        let rgb = to_rgb(&filtered, w, h, channels, &palette);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&rgb)
            .map_err(|e| DocError::invalid_format(format!("PNG re-deflate failed: {e}")))?;
        let data = encoder
            .finish()
            .map_err(|e| DocError::invalid_format(format!("PNG re-deflate failed: {e}")))?;
        Ok(Self {
            width,
            height,
            data,
            filter: ImageFilter::FlateDecode,
        })
    }
}

/// CRC-32 (IEEE) as specified for PNG chunks.
fn png_crc32(data: &[u8]) -> u32 {
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

/// Bounds the decoded image size.
///
/// A PNG declares its dimensions in the header, so a few bytes on the wire can
/// claim a multi-gigabyte decoded image. The limit is checked against the
/// declaration, before anything is allocated or inflated.
const MAX_PIXELS: u64 = 50_000_000;

/// Parse an IHDR chunk payload into `(width, height, colour_type)`.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] for a short chunk, a bit depth other
/// than 8, an unknown colour type, a non-zero compression or filter method, an
/// interlaced image, zero dimensions, or a declared size above [`MAX_PIXELS`].
fn parse_ihdr(data: &[u8]) -> Result<(u32, u32, u8), DocError> {
    // IHDR is exactly 13 bytes: width, height, bit depth, colour type,
    // compression method, filter method, interlace method.
    if data.len() < 13 {
        return Err(DocError::invalid_format(format!(
            "IHDR must be 13 bytes, got {}",
            data.len()
        )));
    }
    let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if data[8] != 8 {
        return Err(DocError::invalid_format(
            "only 8-bit PNG images are supported",
        ));
    }
    let color_type = data[9];
    if !matches!(color_type, 0 | 2 | 3 | 6) {
        return Err(DocError::invalid_format(format!(
            "unsupported PNG color type {color_type}"
        )));
    }
    if data[10] != 0 {
        return Err(DocError::invalid_format(
            "unsupported PNG compression method",
        ));
    }
    if data[11] != 0 {
        return Err(DocError::invalid_format(
            "unsupported PNG filter method",
        ));
    }
    if data[12] != 0 {
        return Err(DocError::invalid_format(
            "interlaced PNG images are not supported",
        ));
    }
    if width == 0 || height == 0 {
        return Err(DocError::invalid_format("PNG has zero dimensions"));
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_PIXELS {
        return Err(DocError::invalid_format(format!(
            "PNG is too large: {width}x{height} ({pixels} pixels, limit {MAX_PIXELS})"
        )));
    }
    Ok((width, height, color_type))
}

/// Reverse PNG per-row filters into unfiltered scanlines.
fn unfilter(raw: &[u8], height: usize, stride: usize, bpp: usize) -> Result<Vec<u8>, DocError> {
    let expected = height * (stride + 1);
    if raw.len() < expected {
        return Err(DocError::invalid_format("PNG pixel data truncated"));
    }
    let mut out = vec![0u8; height * stride];
    let mut previous = vec![0u8; stride];
    for row in 0..height {
        let row_start = row * (stride + 1);
        let filter = raw[row_start];
        let line = &raw[row_start + 1..row_start + 1 + stride];
        let out_start = row * stride;
        for x in 0..stride {
            let left = if x >= bpp {
                out[out_start + x - bpp]
            } else {
                0
            };
            let up = if row > 0 { previous[x] } else { 0 };
            let up_left = if row > 0 && x >= bpp {
                previous[x - bpp]
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => {
                    let avg = i32::from(left) + i32::from(up);
                    u8::try_from(avg / 2).unwrap_or(255)
                }
                4 => paeth(left, up, up_left),
                other => {
                    return Err(DocError::invalid_format(format!(
                        "unknown PNG row filter {other}"
                    )));
                }
            };
            out[out_start + x] = line[x].wrapping_add(predictor);
        }
        previous.copy_from_slice(&out[out_start..out_start + stride]);
    }
    Ok(out)
}

/// Paeth predictor from the PNG specification.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let p = i32::from(left) + i32::from(up) - i32::from(up_left);
    let pa = p.abs();
    let pb = (p - i32::from(left)).abs();
    let pc = (p - i32::from(up)).abs();
    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        up
    } else {
        up_left
    }
}

/// Expand unfiltered samples to packed RGB rows.
fn to_rgb(
    samples: &[u8],
    width: usize,
    height: usize,
    channels: usize,
    palette: &[[u8; 3]],
) -> Vec<u8> {
    // Palette images carry their index in the single sample channel.
    if channels == 1 && !palette.is_empty() {
        let mut indexed = Vec::with_capacity(width * height * 3);
        for i in 0..width * height {
            let index = usize::from(samples.get(i).copied().unwrap_or(0));
            let entry = palette.get(index).copied().unwrap_or([0, 0, 0]);
            indexed.extend_from_slice(&entry);
        }
        return indexed;
    }
    let mut rgb = Vec::with_capacity(width * height * 3);
    for i in 0..width * height {
        if channels == 1 {
            let g = samples.get(i).copied().unwrap_or(0);
            rgb.extend_from_slice(&[g, g, g]);
        } else {
            let pixel = samples
                .get(i * channels..i * channels + 3)
                .unwrap_or(&[0; 3]);
            rgb.extend_from_slice(pixel);
        }
    }
    rgb
}

/// Walk JPEG markers to the SOF frame header for width/height.
fn parse_jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2usize;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        match marker {
            // Standalone markers carry no length.
            0xD8 | 0x01 | 0xD9 => {
                i += 2;
            }
            0xFF => {
                i += 1;
            }
            _ => {
                let len_hi = usize::from(*bytes.get(i + 2)?);
                let len_lo = usize::from(*bytes.get(i + 3)?);
                let seg_len = len_hi << 8 | len_lo;
                let is_sof =
                    (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
                if is_sof {
                    let dim = |offset: usize| -> u32 {
                        let hi = u32::from(bytes.get(offset).copied().unwrap_or(0));
                        let lo = u32::from(bytes.get(offset + 1).copied().unwrap_or(0));
                        hi << 8 | lo
                    };
                    return Some((dim(i + 7), dim(i + 5)));
                }
                i += 2 + seg_len;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_dimensions_parsed_from_sof() {
        // Minimal SOF0: FFC0 len(11) precision(8) height(0x0010) width(0x0020) channels(3)
        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x10, 0x00, 0x20, 0x03]);
        let image = Image::from_jpeg(jpeg).expect("parse");
        assert_eq!(image.width, 32);
        assert_eq!(image.height, 16);
        assert_eq!(image.filter, ImageFilter::DctDecode);
    }

    #[test]
    fn non_jpeg_input_is_rejected() {
        assert!(Image::from_jpeg(b"nope".to_vec()).is_err());
    }

    #[test]
    fn png_unfilters_all_filter_types() {
        // Two rows of 4 pixels, bpp=1: filter None then filter Sub.
        let raw = [
            0, 10, 20, 30, 40, // row 0: no filter
            1, 5, 5, 5, 5, // row 1: sub -> 5,10,15,20
        ];
        let out = unfilter(&raw, 2, 4, 1).expect("unfilter");
        assert_eq!(out, [10, 20, 30, 40, 5, 10, 15, 20]);
    }

    #[test]
    fn png_unfilter_paeth_matches_reference() {
        // filter Paeth with zero neighbours: predictor 0
        let raw = [4, 0, 0, 0];
        let out = unfilter(&raw, 1, 3, 3).expect("unfilter");
        assert_eq!(out, [0, 0, 0]);
    }

    /// Build a PNG with the given IHDR payload, so malformed headers can be
/// exercised without hand-assembling every chunk.
fn png_with_ihdr(ihdr: &[u8]) -> Vec<u8> {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&u32::try_from(ihdr.len()).unwrap_or(u32::MAX).to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(ihdr);
    let mut crc_input = b"IHDR".to_vec();
    crc_input.extend_from_slice(ihdr);
    png.extend_from_slice(&png_crc32(&crc_input).to_be_bytes());
    png
}

#[test]
fn short_ihdr_is_rejected_without_panicking() {
    // Regression: the parser checked `len() < 10` then read `data[12]`, which
    // is an out-of-bounds panic for a 10-, 11-, or 12-byte IHDR.
    for len in 0..13 {
        let png = png_with_ihdr(&vec![0u8; len]);
        let result = Image::from_png(&png);
        assert!(
            result.is_err(),
            "a {len}-byte IHDR must be rejected, got {result:?}"
        );
    }
}

#[test]
fn oversized_declared_dimensions_are_rejected() {
    // A 30000x30000 PNG is tiny on the wire but ~2.7 GB decoded.
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&30_000u32.to_be_bytes());
    ihdr.extend_from_slice(&30_000u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let png = png_with_ihdr(&ihdr);

    let err = Image::from_png(&png).expect_err("oversized image must be rejected");
    assert!(err.to_string().contains("too large"), "{err}");
}

#[test]
fn hostile_chunk_length_does_not_overflow() {
    // A 0xFFFFFFFF length field must be reported as a truncated chunk, not
    // wrap around when computing `8 + len + 4`.
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&u32::MAX.to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(b"short");

    let err = Image::from_png(&png).expect_err("must be rejected");
    assert!(
        err.to_string().contains("truncated") || err.to_string().contains("overflows"),
        "{err}"
    );
}

/// A chunk whose CRC does not match its contents must be rejected rather than
/// decoded into a silently wrong image.
#[test]
fn chunk_crc_mismatch_is_rejected() {
    let mut ihdr = vec![0u8; 13];
    ihdr[0..4].copy_from_slice(&1u32.to_be_bytes());
    ihdr[4..8].copy_from_slice(&1u32.to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = 2;
    let mut png = png_with_ihdr(&ihdr);

    // Corrupt one byte of the IHDR data, leaving the declared CRC stale.
    let target = png.len() - 4 - 1;
    png[target] ^= 0xFF;

    let err = Image::from_png(&png).expect_err("bad CRC must be rejected");
    assert!(err.to_string().contains("CRC"), "{err}");
}

/// The standard CRC-32/IEEE check value guards against a broken CRC table.
#[test]
fn png_crc_matches_the_reference_test_vector() {
    assert_eq!(png_crc32(b"123456789"), 0xCBF4_3926);
}

#[test]
fn nonzero_compression_or_filter_method_is_rejected() {
    let mut base = vec![0u8; 8];
    base.extend_from_slice(&1u32.to_be_bytes());
    base.extend_from_slice(&1u32.to_be_bytes());
    base.extend_from_slice(&[8, 2, 0, 0, 0]);

    let mut bad_compression = base.clone();
    bad_compression[14] = 1; // compression method
    assert!(Image::from_png(&png_with_ihdr(&bad_compression)).is_err());

    let mut bad_filter = base;
    bad_filter[15] = 1; // filter method
    assert!(Image::from_png(&png_with_ihdr(&bad_filter)).is_err());
}

#[test]
fn interlaced_image_is_rejected() {
    let mut ihdr = vec![0u8; 13];
    ihdr[0..4].copy_from_slice(&1u32.to_be_bytes());
    ihdr[4..8].copy_from_slice(&1u32.to_be_bytes());
    ihdr[8] = 8; // bit depth
    ihdr[9] = 2; // colour type: truecolour
    ihdr[12] = 1; // interlace
    let err = Image::from_png(&png_with_ihdr(&ihdr)).expect_err("must be rejected");
    assert!(err.to_string().contains("interlaced"), "{err}");
}

#[test]
fn truncated_pixel_data_is_rejected() {
        assert!(unfilter(&[0, 1, 2], 2, 4, 1).is_err());
    }
}

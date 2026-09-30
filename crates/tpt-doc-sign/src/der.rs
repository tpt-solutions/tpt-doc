//! Minimal DER (TLV) encoder for CMS structures.
//!
//! Only the subset needed to build `SignedData` and friends: definite-length
//! encoding with long-form lengths where required.

/// Append a DER tag-length-value with definite length.
///
/// The `u8` casts below cannot truncate: the short form is only used for
/// lengths below 0x80, and the long-form count is the number of following
/// length bytes (at most 4 for the sizes this crate encodes).
#[allow(clippy::cast_possible_truncation)]
pub fn tlv(out: &mut Vec<u8>, tag: u8, content: &[u8]) {
    out.push(tag);
    let len = content.len();
    if len < 0x80 {
        let Ok(byte_len) = u8::try_from(len) else {
            return;
        };
        out.push(byte_len);
    } else {
        let bytes = len.to_be_bytes();
        let first_significant = bytes
            .iter()
            .position(|b| *b != 0)
            .unwrap_or(bytes.len() - 1);
        let n = bytes.len() - first_significant;
        out.push(0x80 | n as u8);
        out.extend_from_slice(&bytes[first_significant..]);
    }
    out.extend_from_slice(content);
}

/// DER SEQUENCE.
#[must_use]
pub fn sequence(content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0x30, content);
    out
}

/// DER SET OF (ordered as given; callers must sort when required).
#[must_use]
pub fn set_of(content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0x31, content);
    out
}

/// DER INTEGER from a raw big-endian magnitude (a leading zero byte is added
/// when the high bit is set, per DER).
#[must_use]
pub fn integer(magnitude: &[u8]) -> Vec<u8> {
    let mut content = Vec::new();
    if magnitude.first().is_some_and(|b| *b & 0x80 != 0) {
        content.push(0);
    }
    let start = magnitude
        .iter()
        .position(|b| *b != 0)
        .unwrap_or(magnitude.len().saturating_sub(1));
    content.extend_from_slice(&magnitude[start..]);
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0x02, &content);
    out
}

/// DER OBJECT IDENTIFIER from the dotted form.
#[must_use]
pub fn oid(dotted: &str) -> Vec<u8> {
    let parts: Vec<u32> = dotted.split('.').filter_map(|p| p.parse().ok()).collect();
    let mut content = Vec::new();
    if let Some((first, rest)) = parts.split_first() {
        let second = rest.first().copied().unwrap_or(0);
        encode_arc(first * 40 + second, &mut content);
        for arc in rest.iter().skip(1) {
            encode_arc(*arc, &mut content);
        }
    }
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0x06, &content);
    out
}

#[allow(clippy::cast_possible_truncation)] // each byte keeps only its 7 low bits
fn encode_arc(mut value: u32, out: &mut Vec<u8>) {
    let mut temp = vec![(value & 0x7F) as u8];
    value >>= 7;
    while value > 0 {
        temp.push((0x80 | (value & 0x7F)) as u8);
        value >>= 7;
    }
    temp.reverse();
    out.extend_from_slice(&temp);
}

/// DER OCTET STRING.
#[must_use]
pub fn octet_string(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 5);
    tlv(&mut out, 0x04, bytes);
    out
}

/// DER NULL.
#[must_use]
pub fn null() -> Vec<u8> {
    let mut out = Vec::new();
    tlv(&mut out, 0x05, &[]);
    out
}

/// DER context-specific primitive tag (constructed = false).
#[must_use]
pub fn context_primitive(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0x80 | tag, content);
    out
}

/// DER context-specific constructed tag.
#[must_use]
pub fn context_constructed(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0xA0 | tag, content);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_and_long_lengths() {
        let mut out = Vec::new();
        tlv(&mut out, 0x04, b"abc");
        assert_eq!(out, [0x04, 0x03, b'a', b'b', b'c']);

        let big = vec![0u8; 300];
        let mut long = Vec::new();
        tlv(&mut long, 0x04, &big);
        assert_eq!(&long[..4], &[0x04, 0x82, 0x01, 0x2C]);
        assert_eq!(long.len(), 4 + 300);
    }

    #[test]
    fn integer_adds_leading_zero_and_strips_padding() {
        assert_eq!(integer(&[0x80]), [0x02, 0x02, 0x00, 0x80]);
        assert_eq!(integer(&[0x00, 0x00, 0x7F]), [0x02, 0x01, 0x7F]);
        assert_eq!(integer(&[0x01]), [0x02, 0x01, 0x01]);
    }

    #[test]
    fn oid_encodes_known_algorithm() {
        // SHA-256: 2.16.840.1.101.3.4.2.1
        let sha256 = oid("2.16.840.1.101.3.4.2.1");
        assert_eq!(
            sha256,
            [
                0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01
            ]
        );
        // RSA-PSS: 1.2.840.113549.1.1.10
        let pss = oid("1.2.840.113549.1.1.10");
        assert_eq!(pss[0], 0x06);
    }
}

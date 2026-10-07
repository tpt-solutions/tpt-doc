//! Minimal DER (TLV) encoder for CMS structures.
//!
//! Only the subset needed to build `SignedData` and friends: definite-length
//! encoding with long-form lengths where required.

use tpt_doc_core::DocError;

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

/// DER INTEGER from a non-negative big-endian magnitude.
///
/// DER requires the content octets to be minimal (no redundant leading zero)
/// and, because INTEGER is signed, requires a `0x00` pad when the first
/// *content* octet has its high bit set — otherwise the value decodes as
/// negative. Stripping the padding has to happen first: deciding on the pad
/// from the unstripped input encodes `[0x00, 0x80]` as `-128` rather than `128`.
#[must_use]
pub fn integer(magnitude: &[u8]) -> Vec<u8> {
    let start = magnitude.iter().position(|b| *b != 0);
    let significant: &[u8] = match start {
        Some(index) => &magnitude[index..],
        // All zeros (or empty) is the value 0, one content octet.
        None => &[0x00],
    };
    let mut content = Vec::with_capacity(significant.len() + 1);
    if significant[0] & 0x80 != 0 {
        content.push(0);
    }
    content.extend_from_slice(significant);
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0x02, &content);
    out
}

/// DER OBJECT IDENTIFIER from the dotted decimal form.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if an arc is missing, non-numeric, or
/// out of range, or if the leading arcs violate X.660 (`first` is 0-2, and
/// `second` is below 40 unless `first` is 2).
///
/// Silently dropping an unparsable arc would produce a *valid but different*
/// OID, which is worse than failing: the signature would be structurally
/// correct and cryptographically wrong.
pub fn oid(dotted: &str) -> Result<Vec<u8>, DocError> {
    let mut arcs = dotted.split('.');
    let bad = || DocError::invalid_format(format!("malformed OID `{dotted}`"));
    let first: u32 = arcs.next().ok_or_else(bad)?.parse().map_err(|_| bad())?;
    let second: u32 = arcs.next().ok_or_else(bad)?.parse().map_err(|_| bad())?;
    if first > 2 {
        return Err(bad());
    }
    if first < 2 && second >= 40 {
        return Err(bad());
    }

    let mut content = Vec::new();
    // The first two arcs share a byte: 40 * first + second, which cannot
    // overflow because `first` is at most 2.
    encode_arc(first * 40 + second, &mut content);
    for arc in arcs {
        let value: u32 = arc.parse().map_err(|_| bad())?;
        encode_arc(value, &mut content);
    }
    let mut out = Vec::with_capacity(content.len() + 5);
    tlv(&mut out, 0x06, &content);
    Ok(out)
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

    /// The pad decision must be made on the *stripped* content. Reading the
    /// high bit off the raw input encoded `[0x00, 0x80]` as `0x80`, which DER
    /// decodes as **-128** rather than 128.
    #[test]
    fn integer_pad_is_decided_after_stripping() {
        // 128 is the minimal two-octet encoding `00 80`; the redundant input
        // zeros are dropped rather than preserved.
        assert_eq!(integer(&[0x00, 0x80]), [0x02, 0x02, 0x00, 0x80]);
        assert_eq!(integer(&[0x00, 0x00, 0x80]), [0x02, 0x02, 0x00, 0x80]);
    }

    /// A serial number is the case that reaches `integer` from untrusted input:
    /// X.509 serials are non-negative, and a high-bit serial needs the pad.
    #[test]
    fn serial_numbers_stay_non_negative() {
        // 20 octets of 0xFF, padded to 21 content octets so the value does
        // not decode as negative.
        let encoded = integer(&[0xFF; 20]);
        assert_eq!(&encoded[..3], &[0x02, 0x15, 0x00]);
        assert_eq!(encoded.len(), 2 + 21);
        assert!(encoded[2] == 0x00, "pad octet");
        assert_eq!(encoded[3..].len(), 20, "magnitude preserved");
    }

    #[test]
    fn integer_of_zero_is_one_octet() {
        assert_eq!(integer(&[0x00]), [0x02, 0x01, 0x00]);
        assert_eq!(integer(&[0x00, 0x00]), [0x02, 0x01, 0x00]);
        // An empty magnitude is 0, not a zero-length INTEGER (invalid DER).
        assert_eq!(integer(&[]), [0x02, 0x01, 0x00]);
    }

    #[test]
    fn oid_encodes_known_algorithm() {
        // SHA-256: 2.16.840.1.101.3.4.2.1
        let sha256 = oid("2.16.840.1.101.3.4.2.1").expect("sha256 oid");
        assert_eq!(
            sha256,
            [
                0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01
            ]
        );
        // RSA-PSS: 1.2.840.113549.1.1.10
        let pss = oid("1.2.840.113549.1.1.10").expect("pss oid");
        assert_eq!(pss[0], 0x06);
    }

    /// A non-numeric arc used to be filtered out, so `2.16.x.840` silently
    /// encoded as `2.16.840` — a valid OID for a different algorithm.
    #[test]
    fn a_non_numeric_arc_is_an_error_not_a_silent_drop() {
        let err = oid("2.16.x.840").expect_err("non-numeric arc must be rejected");
        assert!(err.to_string().contains("malformed OID"), "{err}");
        assert!(oid("2.16.840.113549.1.9.16.2.47.").is_err(), "trailing dot");
        assert!(oid("2").is_err(), "needs two leading arcs");
        assert!(oid("").is_err(), "empty");
        assert!(oid("3.1").is_err(), "first arc above 2");
        assert!(oid("1.40.1").is_err(), "second arc must be below 40");
        // first == 2 allows any second arc (ISO/IEC arc space).
        assert!(oid("2.999.1").is_ok());
    }
}


//! Shared XML text helpers: legality checking and entity escaping.
//!
//! Every XML-producing crate in this workspace (spreadsheet, word, ubl, fhir,
//! sign) needs the same two guarantees, so they live here rather than being
//! reimplemented per crate.

use alloc::string::String;

use crate::error::DocError;

/// Returns `true` if `ch` may appear in an XML 1.0 document.
///
/// Implements the `Char` production from the XML 1.0 specification:
/// `#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`.
/// Notably excluded are the C0 controls other than tab, newline and carriage
/// return, the surrogate range, and `U+FFFE`/`U+FFFF`.
#[must_use]
pub fn is_xml_char(ch: char) -> bool {
    let code = ch as u32;
    code == 0x09
        || code == 0x0A
        || code == 0x0D
        || (0x20..=0xD7FF).contains(&code)
        || (0xE000..=0xFFFD).contains(&code)
        || (0x10000..=0x10FFFF).contains(&code)
}


/// Escape text for inclusion in XML, rejecting characters XML 1.0 forbids.
///
/// Control characters other than tab, newline and carriage return cannot be
/// represented in a document at all; emitting them raw produces a file that
/// conforming parsers reject, so they are reported instead.
///
/// The five predefined entities are escaped, matching
/// `quick_xml::escape::escape`, so output is byte-identical to what the
/// previous code produced for legal input.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] naming the offending code point.
pub fn escape_xml_text(text: &str) -> Result<String, DocError> {
    validate_xml_chars(text)?;
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    Ok(out)
}

/// Check that `text` contains only characters XML 1.0 permits, without
/// escaping.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] naming the offending code point.
pub fn validate_xml_chars(text: &str) -> Result<(), DocError> {
    for ch in text.chars() {
        if !is_xml_char(ch) {
            return Err(DocError::InvalidFormat(format!(
                "U+{:04X} is not a legal XML 1.0 character",
                ch as u32
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_newline_and_return_are_legal() {
        assert!(is_xml_char('\t'));
        assert!(is_xml_char('\n'));
        assert!(is_xml_char('\r'));
    }

    #[test]
    fn other_c0_controls_are_illegal() {
        for code in 0u32..0x20 {
            if code != 0x09 && code != 0x0A && code != 0x0D {
                let ch = char::from_u32(code).expect("c0 control");
                assert!(!is_xml_char(ch), "U+{code:04X} must be rejected");
            }
        }
    }

    #[test]
    fn del_and_escape_are_legal() {
        assert!(is_xml_char('\u{7F}'));
    }

    #[test]
    fn noncharacters_are_illegal() {
        assert!(!is_xml_char('\u{FFFE}'));
        assert!(!is_xml_char('\u{FFFF}'));
    }

    #[test]
    fn supplementary_planes_are_legal() {
        assert!(is_xml_char('\u{1F600}'));
        assert!(is_xml_char('\u{10FFFF}'));
    }

    #[test]
    fn escaping_matches_predefined_entities() {
        assert_eq!(
            escape_xml_text("a & b < c > d \" e ' f").expect("legal"),
            "a &amp; b &lt; c &gt; d &quot; e &apos; f"
        );
    }

    #[test]
    fn spacing_is_preserved() {
        assert_eq!(
            escape_xml_text(" <b> ").expect("legal"),
            " &lt;b&gt; "
        );
    }

    #[test]
    fn illegal_control_characters_are_rejected() {
        let err = escape_xml_text("bad\u{1}value").expect_err("must reject");
        assert!(
            err.to_string().contains("U+0001"),
            "error should name the code point: {err}"
        );
    }

    #[test]
    fn validate_matches_escape_on_acceptance() {
        for input in ["", "plain", "a\tb\nc\rd", "caf\u{e9}", "\u{1F600}"] {
            assert_eq!(
                validate_xml_chars(input).is_ok(),
                escape_xml_text(input).is_ok(),
                "disagreement on {input:?}"
            );
        }
    }
}

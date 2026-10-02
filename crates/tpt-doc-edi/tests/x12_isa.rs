//! X12 ISA envelope validation.
//!
//! The delimiters that drive all segment splitting come from fixed positions in
//! the ISA header, so a malformed envelope silently produces garbage segments
//! unless it is rejected up front.

use tpt_doc_edi::x12::X12Parser;

/// A well-formed 106-byte ISA, `*` element / `:` component / `~` terminator.
const VALID_ISA: &[u8] =
    b"ISA*00*          *00*          *ZZ*SENDERID       *ZZ*RECEIVERID     *260101*0900*^*00501*000000001*0*P*:~";

fn with_byte(index: usize, byte: u8) -> Vec<u8> {
    let mut bytes = VALID_ISA.to_vec();
    bytes[index] = byte;
    bytes
}

#[test]
fn valid_isa_is_accepted() {
    assert!(X12Parser::new(VALID_ISA).is_ok());
}

#[test]
fn non_printable_element_separator_is_rejected() {
    // ISA01 = CR would make every segment boundary ambiguous.
    assert!(X12Parser::new(&with_byte(3, b'\r')).is_err());
    assert!(X12Parser::new(&with_byte(3, b'\n')).is_err());
    assert!(X12Parser::new(&with_byte(3, 0x00)).is_err());
}

#[test]
fn element_separator_equal_to_terminator_is_rejected() {
    // ISA01 == ISA terminator means nothing can be split.
    assert!(X12Parser::new(&with_byte(105, b'*')).is_err());
}

#[test]
fn component_separator_equal_to_element_separator_is_rejected() {
    // ISA16 == ISA01 is ambiguous.
    assert!(X12Parser::new(&with_byte(104, b'*')).is_err());
}

#[test]
fn non_printable_component_separator_is_rejected() {
    assert!(X12Parser::new(&with_byte(104, b'\r')).is_err());
}

#[test]
fn short_input_is_rejected() {
    assert!(X12Parser::new(b"ISA*too short").is_err());
    assert!(X12Parser::new(b"").is_err());
}

#[test]
fn wrong_prefix_is_rejected() {
    let mut bytes = VALID_ISA.to_vec();
    bytes[..3].copy_from_slice(b"NSA");
    assert!(X12Parser::new(&bytes).is_err());
}

//! Inputs that once panicked the parser. Each must now return an error or a
//! result, never panic. The first came from the fuzzer.

use tpt_doc_hl7v2::Hl7Parser;

#[test]
fn a_bare_msh_with_no_field_separator_is_an_error_not_a_panic() {
    let results: Vec<_> = Hl7Parser::new(b"MSH").collect();
    assert!(results.iter().any(Result::is_err), "{results:?}");
}

#[test]
fn the_fuzzer_input_that_panicked_now_parses_or_errors() {
    let input: &[u8] = b"MSHT1MSHT1MH\nTSS\x1aS\nMSH\nT1\x1aS\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\nMSHT1S\x1a\x10\x00\x00\x00\x00\x00\x00\x00\n";
    let _ = Hl7Parser::new(input).count();
}

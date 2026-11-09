#![no_main]
use libfuzzer_sys::fuzz_target;

// Any byte string must parse or fail with an error, never panic.
fuzz_target!(|data: &[u8]| {
    for segment in tpt_doc_hl7v2::Hl7Parser::new(data) {
        let _ = segment;
    }
});

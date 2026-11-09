#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for segment in tpt_doc_edi::EdifactParser::new(data) {
        let _ = segment;
    }
    let _ = tpt_doc_edi::edifact_docs::parse_document(data);
});

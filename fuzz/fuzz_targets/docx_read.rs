#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_doc_word::DocxDocument::read(data);
    let _ = tpt_doc_word::DocxReader::extract_text(data);
});

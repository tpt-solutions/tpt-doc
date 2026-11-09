#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(parser) = tpt_doc_edi::x12::X12Parser::new(data) {
        for segment in parser {
            let _ = segment;
        }
    }
    let _ = tpt_doc_edi::x12_850::parse_850(data);
    let _ = tpt_doc_edi::x12_856::parse_856(data);
});

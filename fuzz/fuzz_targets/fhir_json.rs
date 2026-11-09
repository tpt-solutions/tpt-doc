#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_doc_fhir::Patient::from_json(data);
    let _ = tpt_doc_fhir::Observation::from_json(data);
    let _ = tpt_doc_fhir::Bundle::from_json(data);
});

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = tpt_doc_fhir::Patient::from_xml(data);
    let _ = tpt_doc_fhir::Observation::from_xml(data);
    let _ = tpt_doc_fhir::Encounter::from_xml(data);
    let _ = tpt_doc_fhir::Organization::from_xml(data);
});

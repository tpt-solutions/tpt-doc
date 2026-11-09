#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut reader) = tpt_doc_spreadsheet::xlsx::XlsxReader::new(data) {
        for row in reader.rows() {
            let _ = row;
        }
    }
});

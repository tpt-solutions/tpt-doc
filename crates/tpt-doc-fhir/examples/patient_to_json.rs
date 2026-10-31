//! Build a typed FHIR R5 Patient and serialize it to JSON.
//!
//! Run with `cargo run -p tpt-doc-fhir --example patient_to_json`.

use tpt_doc_fhir::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `build()` only exists once every mandatory field is set.
    let patient = Patient::builder()
        .id("nz-12345")
        .name(HumanName::new("Smith", "John"))
        .build();

    let json = patient.to_json()?;
    println!("{}", String::from_utf8_lossy(&json));
    Ok(())
}

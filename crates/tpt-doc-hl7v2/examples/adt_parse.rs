//! Parse an ADT^A01 message and print the patient identifiers.
//!
//! Run with `cargo run -p tpt-doc-hl7v2 --example adt_parse`.

use tpt_doc_hl7v2::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = b"MSH|^~\\&|HIS|ACME|RIS|ACME|||ADT^A01|MSG1|P|2.5.1\rPID|1||12345^^^ACME||Doe^Jane";
    for segment in Hl7Parser::new(message) {
        let segment = segment?;
        println!("{}: {} fields", segment.tag(), segment.raw_fields().len());
    }
    let segments: Vec<_> = Hl7Parser::new(message).collect::<Result<_, _>>()?;
    println!("message type: {:?}", segments[0].component(9, 1));
    println!("patient id:   {:?}", segments[1].component(3, 1));
    Ok(())
}

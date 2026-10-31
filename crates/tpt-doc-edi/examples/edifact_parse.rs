//! Stream the segments of an EDIFACT interchange.
//!
//! Run with `cargo run -p tpt-doc-edi --example edifact_parse`.

use tpt_doc_edi::edifact::EdifactParser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message =
        b"UNB+UNOA:1+SENDER+RECEIVER+260101:0900+1'UNH+1+ORDERS:D:96A:UN'UNT+2+1'UNZ+1+1'";
    for segment in EdifactParser::new(message) {
        let segment = segment?;
        println!("{}", segment.tag());
    }
    Ok(())
}

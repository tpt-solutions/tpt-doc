//! Read the payments and claims from an X12 835 remittance advice.
//!
//! Run with `cargo run -p tpt-doc-edi --example x12_835_parse`.

use tpt_doc_edi::x12::X12Parser;

const REMITTANCE: &[u8] = include_bytes!("../tests/fixtures/sample_835.x12");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for segment in X12Parser::new(REMITTANCE)? {
        let segment = segment?;
        match segment.tag() {
            // BPR02 is the total payment amount for the remittance.
            "BPR" => println!("total payment {}", segment.elements().get(1).unwrap_or(&"")),
            // CLP: claim id, status code, billed amount, paid amount.
            "CLP" => {
                let e = segment.elements();
                println!(
                    "claim {} status {} billed {} paid {}",
                    e[0], e[1], e[2], e[3]
                );
            }
            "SVC" => println!("  service {}", segment.elements().first().unwrap_or(&"")),
            _ => {}
        }
    }
    Ok(())
}

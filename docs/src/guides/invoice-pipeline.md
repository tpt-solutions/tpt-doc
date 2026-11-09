# Tutorial: from invoice JSON to a signed Factur-X PDF

This tutorial runs the e-invoice pipeline end to end: an invoice as JSON goes
in, and a PDF comes out that carries the invoice as Factur-X and is signed
with PAdES.

The pipeline lives behind the `pipeline` feature of the `tpt-doc` crate:

```toml
[dependencies]
tpt-doc = { version = "0.1", features = ["pipeline"] }
ring = "0.17"
```

## The steps

1. **Read** the invoice from JSON. The field names are the serde names of
   `tpt_doc_ubl::Invoice`, so an invoice that has been parsed from UBL XML can
   be serialized with `serde_json` and read back.
2. **Check** it against the PEPPOL BIS Billing 3.0 rules. An invoice that breaks
   a rule is refused, and nothing is rendered. The error lists the first few
   violations.
3. **Render** a summary page and attach the UBL XML to it as Factur-X
   (`factur-x.xml`, PDF/A-3 metadata left to the caller).
4. **Sign** the PDF with a PAdES signature over the caller's key and
   certificate, at a time the caller supplies.

## Running it

```rust,ignore
use ring::rand::SystemRandom;
use tpt_doc::pipeline::invoice_json_to_signed_pdf;
use tpt_doc::sign::{PrivateKey, pem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read("invoice.json")?;
    let key = PrivateKey::from_pem(&std::fs::read("signer-key.pem")?)?;
    let cert = pem::pem_to_der(&std::fs::read("signer-cert.pem")?)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    let pdf = invoice_json_to_signed_pdf(&json, &key, &cert, &SystemRandom::new(), now)?;
    std::fs::write("invoice-signed.pdf", pdf)?;
    Ok(())
}
```

The signing time is passed in rather than read inside the library, so a test
can pin it. The certificate must be valid at that time, and it must belong to
the key; both are checked before anything is signed.

## Checking the result

The output is an ordinary PDF. Its text can be read back with
`tpt_doc::pdf::extract::extract_text`, and its signature can be checked with
`tpt_doc::sign::verify::verify_detached` over the signed byte ranges, or with
any PDF signature validator.

## Test material

The repository ships a test key and certificate under
`crates/tpt-doc-sign/tests/fixtures`. They are public and must not be used to
sign real invoices. Use your own key and a certificate issued for it.

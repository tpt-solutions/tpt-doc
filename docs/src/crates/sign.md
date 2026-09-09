# tpt-doc-sign

Pure-Rust cryptographic document signing (PAdES, XAdES, CAdES) leveraging `ring` primitives.

## Standards

| Standard | Description |
|---|---|
| **CAdES** | CMS Advanced Electronic Signatures (detached, over arbitrary bytes) |
| **XAdES** | XML Advanced Electronic Signatures (enveloped, over XML documents) |
| **PAdES** | PDF Advanced Electronic Signatures (integrated into PDF `/ByteRange`) |

HIPC 2020 compliance requires PAdES or CAdES signatures on health data exchanges.

## Usage

```rust
use tpt_doc_sign::pades;

let pdf_bytes = std::fs::read("document.pdf")?;
let key = pades::load_private_key(std::fs::read("key.pem")?)?;
let signed = pades::sign(&pdf_bytes, &key)?;
std::fs::write("document_signed.pdf", signed)?;
```

## Key Types

- `PrivateKey` — loaded from PEM; wraps `ring::signature::RsaKeyPair`
- `SignedDocument` — the output of a signing operation
- `CertChain` — X.509 certificate chain embedded in `SignedData`

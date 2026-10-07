# tpt-doc-sign

Pure-Rust cryptographic document signing (PAdES, XAdES, CAdES) leveraging `ring` primitives.

## Standards

| Standard | Description |
|---|---|
All three signatures are RSA with SHA-256 (CAdES/PAdES use RSA-PSS, XAdES uses
RSASSA-PKCS1-v1_5), produced by `ring` with no C-FFI. ECDSA is not yet supported.

| **CAdES** | CMS Advanced Electronic Signatures (detached, over arbitrary bytes) |
| **XAdES** | XML Advanced Electronic Signatures (enveloped, over XML documents) |
| **PAdES** | PDF Advanced Electronic Signatures (integrated into PDF `/ByteRange`) |

HIPC 2020 compliance requires PAdES or CAdES signatures on health data exchanges.

## Usage

Signatures take a private key, the signer's certificate DER, and an RNG. `cades`
covers arbitrary bytes; `pades` produces a signed PDF and `xades` an enveloped
XML signature.

```rust
use ring::rand::SystemRandom;
use tpt_doc_sign::{PrivateKey, cades, pades, pem};

let key = PrivateKey::from_pem(std::fs::read("key.pem")?.as_slice())?;
let cert_der = pem::pem_to_der(std::fs::read("cert.pem")?.as_slice())?;
let rng = SystemRandom::new();

// Detached CAdES over arbitrary bytes.
let cms = cades::sign_detached(b"document bytes", &key, &cert_der, &rng)?;

// PAdES: an incremental update, so the original PDF stays a valid prefix.
let pdf = std::fs::read("document.pdf")?;
let signed = pades::sign(&pdf, &key, &cert_der, &rng)?;
std::fs::write("document_signed.pdf", signed)?;
# Ok::<(), tpt_doc_core::DocError>(())
```

## Keys and certificates

`PrivateKey::from_pem` accepts both PKCS#8 (`BEGIN PRIVATE KEY`) and PKCS#1
(`BEGIN RSA PRIVATE KEY`) armor — `openssl genrsa` emits the latter by default.
PKCS#1 is wrapped into PKCS#8 internally after checking the tag, length form,
`RSAPrivateKey` version, and rejecting trailing bytes.

`Certificate` wraps the parsed X.509 data and provides two checks:

```rust
let cert = Certificate::from_pem(cert_pem_bytes)?;
cert.check_validity(unix_seconds)?; // inclusive at both ends
cert.check_key_matches(&key)?;     // private half of the certificate's public key
```

`cades::sign_detached` and `xades::sign` call `check_key_matches` for you. A
mismatch would otherwise produce a well-formed signature that no verifier can
attribute to the embedded certificate, so it fails at signing time instead.

## Key Types

- `PrivateKey` — loaded from PEM (PKCS#8 or PKCS#1); wraps `ring::signature::RsaKeyPair`
- `Certificate` — parsed X.509 certificate with validity and key-match checks
- `cades::sign_detached` — DER CMS `SignedData` over arbitrary bytes
- `pades::sign` — PDF signed via a `/ByteRange` incremental update
- `xades::sign` — `<ds:Signature>` enveloped inside the document root

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
// `/M` records the signing time and the certificate's validity window is
// checked against it.
let pdf = std::fs::read("document.pdf")?;
let signed = pades::sign(&pdf, &key, &cert_der, &rng)?;
std::fs::write("document_signed.pdf", signed)?;

// `sign_at` takes the time explicitly, which is what reproducible builds and
// tests want. `unix_now()` is available if you need the same clock `sign` uses.
let signed = pades::sign_at(&pdf, &key, &cert_der, &rng, 1_800_000_000)?;
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

All three signature functions enforce `check_validity` against the signing time,
so an expired certificate cannot produce a signature that no conforming verifier
will accept. Each has a `_at` variant taking the time explicitly
(`cades::sign_detached_at`, `xades::sign_at`, `pades::sign_at`); the plain
functions delegate using the system clock.

## Key Types

- `PrivateKey` — loaded from PEM (PKCS#8 or PKCS#1); wraps `ring::signature::RsaKeyPair`
- `Certificate` — parsed X.509 certificate with validity and key-match checks
- `cades::sign_detached` — DER CMS `SignedData` over arbitrary bytes
- `pades::sign` / `pades::sign_at` — PDF signed via a `/ByteRange` incremental update
- `xades::sign` — `<ds:Signature>` enveloped inside the document root

## XAdES structure

`xades::sign` emits an `XAdES` `QualifyingProperties` block, which is what
separates an `XAdES` signature from plain `XML-DSig`:

```text
<ds:Signature Id="tptdoc-signature">
  <ds:SignedInfo>
    <ds:Reference URI="">            → the document
    <ds:Reference URI="#tptdoc-signed-properties"
                   Type="…#SignedProperties">   → the signed properties
  </ds:SignedInfo>
  <ds:SignatureValue/>
  <ds:KeyInfo/>
  <ds:Object>
    <xades:QualifyingProperties Target="#tptdoc-signature">
      <xades:SignedProperties Id="tptdoc-signed-properties">
        <xades:SigningCertificate>
          <xades:Cert>
            <xades:CertDigest>      → SHA-256 of the embedded certificate
            <xades:IssuerSerial>    → the certificate serial number
```

The second `Reference` matters: without it the properties would be present but
unsigned, so a substituted certificate would go undetected. `SignedInfo`
declares `xmlns:ds` on itself and is emitted without incidental whitespace, so
the signature covers the element in its own namespace context in a form C14N
reproduces.

Not implemented: a C14N pass over the *document* reference (the digest is taken
over the input bytes as provided), and `IssuerName` (the `x509-parser` 0.16 API
cannot render issuer RDNs back to XML).

## How PAdES output is structured

The incremental update writes four objects, because a signature nobody can find
is the same as no signature:

| Object | Purpose |
|---|---|
| `/Type /Sig` | the CMS signature, `/ByteRange`, `/Contents`, `/M` |
| `/FT /Sig` + `/Subtype /Widget` | the signature field, merged with its invisible widget annotation |
| `/AcroForm` | `/Fields` plus `/SigFlags 3` (SignaturesExist + AppendOnly) |
| redefined catalog | references the `/AcroForm`, with its existing entries preserved |

`/SigFlags` bit 2 (AppendOnly) matters: without it, a later incremental update
could silently alter or drop the signature.

The original PDF bytes are never touched — the update is appended, so the
document stays valid and openable.

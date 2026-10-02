# tpt-doc-sign

[![Crates.io](https://img.shields.io/crates/v/tpt-doc-sign.svg)](https://crates.io/crates/tpt-doc-sign)
[![Docs.rs](https://docs.rs/tpt-doc-sign/badge.svg)](https://docs.rs/tpt-doc-sign)
[![CI](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-doc/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/tpt-solutions/tpt-doc#license)

Pure-Rust cryptographic document signing — **PAdES**, **XAdES**, and **CAdES**.

Leverages [`ring`](https://crates.io/crates/ring) for cryptographic primitives.
No C-FFI, no OpenSSL, no shelling out.

## Highlights

- **CAdES-BES** detached CMS `SignedData` signatures (RSA-PSS, SHA-256, MGF1-SHA-256).
- **PAdES** PDF signatures via incremental update — the original PDF bytes are
  never modified, so the document stays valid and openable.
- **XAdES** enveloped XML signatures using `rsa-sha256`.
- Minimal, self-contained DER (TLV) encoder for CMS structures — no
  heavyweight ASN.1 dependency.
- PEM key loading with PKCS#8 and PKCS#1 support.
- Certificate parsing via `x509-parser`.

## Installation

```toml
[dependencies]
tpt-doc-sign = "0.1"
ring = "0.17"
```

`ring` is a direct dependency of this crate; it appears in the tree either way,
but you will typically reference `SecureRandom` yourself to supply randomness.

## Usage

### CAdES detached signature

```rust
use ring::rand::SystemRandom;
use tpt_doc_sign::{cades, pem, PrivateKey};

let key = PrivateKey::from_pem(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/test_key.pem")))?;
let cert_der = pem::pem_to_der(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/test_cert.pem")))?;
let rng = SystemRandom::new();

let cms = cades::sign_detached(b"document bytes", &key, &cert_der, &rng)?;
assert!(!cms.is_empty());
# Ok::<(), tpt_doc_core::DocError>(())
```

### PAdES PDF signature

The signature is appended as an **incremental update**, so the original bytes
are preserved byte-for-byte at the front of the output. `/ByteRange` correctly
excludes the `/Contents` placeholder from the digest.

```rust
use ring::rand::SystemRandom;
use tpt_doc_pdf::{Document, Font, Page};
use tpt_doc_sign::{pades, pem, PrivateKey};

let key = PrivateKey::from_pem(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/test_key.pem")))?;
let cert_der = pem::pem_to_der(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/test_cert.pem")))?;

let mut doc = Document::new();
let font = doc.embed_builtin_font(Font::Helvetica);
let mut page = Page::a4();
page.text("Report to sign", font, 12.0, (72.0, 700.0));
doc.add_page(page);
let pdf = doc.write()?;

let signed = pades::sign(&pdf, &key, &cert_der, &SystemRandom::new())?;
assert!(signed.starts_with(&pdf));
# Ok::<(), tpt_doc_core::DocError>(())
```

### XAdES enveloped XML signature

A `<ds:Signature>` element is inserted inside the document's root element.

```rust
use ring::rand::SystemRandom;
use tpt_doc_sign::{pem, xades, PrivateKey};

let key = PrivateKey::from_pem(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/test_key.pem")))?;
let cert_der = pem::pem_to_der(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/test_cert.pem")))?;

let xml = br#"<Invoice><ID>INV-001</ID></Invoice>"#;
let signed = xades::sign(xml, &key, &cert_der, &SystemRandom::new())?;
assert!(String::from_utf8_lossy(&signed).contains("ds:Signature"));
# Ok::<(), tpt_doc_core::DocError>(())
```

## Modules

| Module | Contents |
|---|---|
| `pades` | `sign` — PDF signature via incremental update and `/ByteRange`. |
| `xades` | `sign` — enveloped XML signature (`rsa-sha256`). |
| `cades` | `sign_detached` — CMS `SignedData`, plus OID constants. |
| `der` | Minimal DER TLV encoder: `tlv`, `sequence`, `set_of`, `integer`, `oid`, `octet_string`, `null`, `context_primitive`, `context_constructed`. |
| `pem` | `PrivateKey`, `pem_to_der`, `from_pem`, `sign_pss_sha256`, `sign_pkcs1_sha256`. |

## Algorithm parameters

| Parameter | Value |
|---|---|
| Signature algorithm | RSA-PSS (`1.2.840.113549.1.1.10`) |
| Digest | SHA-256 (`2.16.840.1.101.3.4.2.1`) |
| Mask generation | MGF1-SHA-256 (`1.2.840.113549.1.1.8`) |
| PSS salt length | 32 bytes |
| XML signature algorithm | RSASSA-PKCS1-v1_5 (`rsa-sha256`) |

## Documentation

- Crate docs: <https://docs.rs/tpt-doc-sign>
- Repository: <https://github.com/tpt-solutions/tpt-doc>

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache License, Version 2.0](../../LICENSE-APACHE), at your option.

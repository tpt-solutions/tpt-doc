# Changelog

All notable changes to `tpt-doc-sign` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- PKCS#1 private keys (`-----BEGIN RSA PRIVATE KEY-----`) now load, which is
  what `openssl genrsa` emits by default. The PKCS#1 `RSAPrivateKey` is wrapped
  into a PKCS#8 `PrivateKeyInfo`, validating the tag, the length form, the
  `RSAPrivateKey` version, and rejecting trailing bytes.
- `Certificate` (in `pem`) with `from_der`/`from_pem`, `public_key_algorithm`,
  `not_before`, `not_after`, `check_key_matches`, and `check_validity`.
### Added

- `xades`: the XAdES `QualifyingProperties` block, containing
  `SignedProperties` → `SignedSignatureProperties` → `SigningCertificate` →
  `Cert` → `CertDigest` (SHA-256 of the embedded certificate) and
  `IssuerSerial`. `SignedInfo` now carries a second `Reference` of type
  `http://uri.etsi.org/01903#SignedProperties` pointing at the properties by
  `Id`, so they are *signed* rather than merely present. `<ds:Signature>`
  gained the `Id` that `QualifyingProperties/@Target` names.
- `pades::sign_at`, `cades::sign_detached_at`, and `xades::sign_at` take the
  signing time as an explicit number of seconds since the Unix epoch; the
  corresponding functions without `_at` delegate using the system clock. All
  three enforce the certificate's validity window against that instant, so the
  time is injectable and signatures are reproducible in tests and builds.
- `Certificate::not_before`/`not_after` accessors, so callers can choose a
  signing time inside the validity window.

### Fixed

- `der::integer` strips leading zeros *before* deciding on the `0x00` sign pad.
  Reading the high bit off the unstripped input encoded `[0x00, 0x80]` as `-128`
  rather than `128`, which corrupts the CMS `serialNumber`. Zero now encodes as
  a single `0x00` octet rather than a zero-length (invalid) INTEGER.
- `der::oid` returns `Result` instead of silently dropping arcs that fail to
  parse: `2.16.x.840` used to encode as `2.16.840`, a valid OID for a different
  algorithm. The X.660 leading-arc rules are now checked too.
- `cades::sign_detached` and `xades::sign` verify that the private key is the
  private half of the certificate's public key. A mismatch previously produced a
  well-formed signature that no verifier could attribute to the embedded
  certificate.
- `signingCertificateV2` omits its `hashAlgorithm` field, which is `DEFAULT
  sha256`; encoding a present default is a DER violation that strict validators
  reject.
- `pades`: an incremental update's trailer now preserves `/Info` and `/ID` from
  the previous trailer. The trailer replaces the previous one wholesale, so
  document metadata and the file identity used for revision tracking were
  silently lost on signing.
- `pades`: the trailer dictionary is delimited by matching `<<`/`>>` depth
  rather than the first `>`. A hex string in `/ID [<aabb> <ccdd>]` contains its
  own `>`, which truncated the trailer and lost the entries.
- `xades`: `SignedInfo` declares `xmlns:ds` on itself and is emitted without
  incidental whitespace, so the signature is computed over the element in its
  own namespace context in a form a C14N implementation reproduces exactly.
- `pades`: the last `startxref` is located across the whole file. A literal
  `startxref` inside a stream could be matched instead, and a fixed-size tail
  window could miss a trailer that is not near the end.
- `pades`: the `/Contents` placeholder is searched from the start of the
  appended update. A run of `0` bytes in the original PDF body was matched
  first, and the CMS hex was spliced over it, corrupting the signed file.
- `pades`: `/M` records the actual signing time. It was hardcoded to
  `D:20260101000000Z`, so every signature claimed the same instant — for an
  audit-trail format the recorded moment is part of what is being attested.
  Formatted as `D:YYYYMMDDHHmmSS+00'00'` per PDF 32000 §7.9.4.
- `pades`: `/SubFilter` is now `ETSI.CAdES.detached` rather than
  `adbe.pkcs7.detached`. The CAdES subfilter is what ETSI EN 319 142 PAdES
  profiles specify, and the CMS produced by `cades` is a CAdES `SignedData`.
- `pades`: the signer certificate's validity window is checked against the
  signing time, so an expired or not-yet-valid certificate fails at signing
  rather than producing a signature no conforming verifier accepts. `cades` and
  `xades` now do the same through their `_at` variants.
- `pades`: the signature is discoverable. Previously only the `/Sig` dictionary
  was written, which is cryptographically valid but unreachable — no conforming
  viewer finds a signature that is not listed in the catalog's `/AcroForm`.
  The update now also writes a signature field merged with its widget
  (`/FT /Sig`, `/Subtype /Widget`), an `/AcroForm` with `/SigFlags 3`
  (SignaturesExists + AppendOnly), and redefines the catalog to reference it.
- `pades`: the incremental update's xref table covers every object it writes,
  sorted by object number and split into contiguous runs, with exact 20-byte
  entries. `/Size` is derived from the highest object number written.

## [0.1.0] - 2026-02-10

### Added

- `cades` module: `sign_detached` producing DER-encoded CMS `SignedData`
  (CAdES-BES) with the signer certificate embedded, RSA-PSS / SHA-256 /
  MGF1-SHA-256 with a 32-byte salt, plus public OID constants.
- `pades` module: `sign` appending a signature dictionary as a PDF incremental
  update, with exact `/Contents` placeholder sizing, file-absolute
  `/ByteRange` computation, and original-bytes preservation.
- `xades` module: `sign` inserting an enveloped `<ds:Signature>` element using
  the XML-DSig canonical form and `rsa-sha256`.
- `der` module: a minimal DER TLV encoder (`tlv`, `sequence`, `set_of`,
  `integer`, `oid`, `octet_string`, `null`, `context_primitive`,
  `context_constructed`).
- `pem` module: `PrivateKey` with `from_pem`, `sign_pss_sha256`,
  `sign_pkcs1_sha256`, `signature_len`, and `pem_to_der`.
- Top-level re-exports: `sign` (PAdES), `sign_xml` (XAdES), `PrivateKey`.
- Test fixtures (`test_key.pem`, `test_cert.pem`) and integration tests. PKCS#1
  support and the key/certificate match check were added after this release.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-sign-v0.1.0

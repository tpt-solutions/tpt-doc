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
  `check_key_matches`, and `check_validity`.

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

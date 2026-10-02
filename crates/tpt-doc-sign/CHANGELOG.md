# Changelog

All notable changes to `tpt-doc-sign` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- Test fixtures (`test_key.pem`, `test_cert.pem`) and integration tests.

[0.1.0]: https://github.com/tpt-solutions/tpt-doc/releases/tag/tpt-doc-sign-v0.1.0

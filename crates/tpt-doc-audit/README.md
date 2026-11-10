# tpt-doc-audit

A tamper-evident audit log for document processing. Each entry is chained to
the one before it with SHA-256, so editing, removing or reordering any entry
breaks verification. The log can be sealed with a CAdES signature, attached to
a PDF, and exported as a FHIR `Provenance` resource.

- **Status:** 0.1.0, not yet published. MSRV 1.88. Licence `MIT OR Apache-2.0`.

## What it does

- **`AuditLog`** appends entries, returns the current head hash, and verifies
  the whole chain.
- **JSON Lines** storage: `to_jsonl` and `from_jsonl` round-trip the log.
- **Seal:** `seal` signs the head hash with a CAdES signature, and `verify_seal`
  checks it against the chain.
- **PDF export:** `attach_to_pdf` embeds the log as `audit-log.jsonl` in a PDF,
  using the `x-ndjson` media type.
- **FHIR export:** `provenance` builds a `Provenance` resource that points at a
  given entry and target.

The genesis hash is exported as `GENESIS`, and every chain starts from it.

## Quick start

```rust,ignore
use tpt_doc_audit::AuditLog;

let mut log = AuditLog::new();
log.append("2026-11-10T09:00:00Z", "invoice.created", b"{\"id\":\"5001\"}")?;
log.append("2026-11-10T09:01:00Z", "invoice.signed", b"{\"id\":\"5001\"}")?;
log.verify()?;

let jsonl = log.to_jsonl()?;
let restored = AuditLog::from_jsonl(&jsonl)?;
```

`seal` takes a private key, the signer's certificate (DER), a random source and
a Unix time, and returns a detached CAdES signature over the log's head hash.

## Scope and known gaps

- The chain proves order and integrity. It does not prove who made an entry;
  use `seal` with a signing certificate for that.
- Timestamps inside entries are recorded as given. An RFC 3161 trusted
  timestamp is not yet supported, because it needs a timestamp authority.

## Testing

```sh
cargo test -p tpt-doc-audit
```

The tests cover tampering with each entry position, round trips through JSON
Lines, and seal verification.

## Related crates

- `tpt-doc-sign` provides the CAdES signing used by `seal`.
- `tpt-doc-pdf` provides the attachment mechanism.

## Licence

Licensed under either of Apache-2.0 or MIT, at your option.

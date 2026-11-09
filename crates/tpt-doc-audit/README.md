# tpt-doc-audit

A tamper-evident audit log. Each event is a SHA-256 hash-chain entry that
commits to the one before it, so editing, deleting, reordering or inserting an
entry breaks every hash after it. The head of the chain can be sealed with a
CAdES detached signature.

```rust,ignore
use tpt_doc_audit::AuditLog;

let mut log = AuditLog::new();
log.append("2026-11-09T10:00:00Z", "invoice.approved", invoice_bytes)?;

let text = log.to_jsonl()?;                 // persist one entry per line
let restored = AuditLog::from_jsonl(&text)?; // refuses an edited or truncated file

let seal = log.seal(&key, &cert_der, &rng, unix_now)?; // CAdES over the head
```

The payload is hashed rather than stored, so the log can be kept without the
data it describes. The timestamp is supplied by the caller, so the same events
always produce the same chain.

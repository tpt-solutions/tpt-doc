//! Exporting an audit log: into a PDF, as an attachment, and entry by entry as
//! FHIR `Provenance` resources.
//!
//! Both exports verify the chain first. A broken log is never exported, so a
//! file that carries an audit trail carries one that checks out.

use tpt_doc_core::DocError;
use tpt_doc_pdf::Document;

use crate::{AuditLog, Entry};

/// The name the log is attached under.
pub const ATTACHMENT_NAME: &str = "audit-log.jsonl";

/// Extension URL that carries an entry's hash in a `Provenance` resource.
pub const HASH_EXTENSION_URL: &str = "urn:tpt-doc:audit-hash";

impl AuditLog {
    /// Attach the log to a PDF as `audit-log.jsonl`.
    ///
    /// The attachment is the JSON Lines form, so it can be checked later with
    /// [`AuditLog::from_jsonl`].
    ///
    /// # Errors
    /// Returns [`DocError`] if the log does not verify or cannot be serialized.
    pub fn attach_to_pdf(&self, document: &mut Document) -> Result<(), DocError> {
        self.verify()?;
        let text = self.to_jsonl()?;
        document.attach_file(ATTACHMENT_NAME, "x-ndjson", text.into_bytes());
        Ok(())
    }

    /// Describe one entry as a FHIR `Provenance` resource, about the resource
    /// at `target` (for example `Invoice/inv-5001`).
    ///
    /// The event becomes the activity, the entry's timestamp becomes `recorded`,
    /// the payload digest becomes the source entity, and the entry's hash is kept
    /// in an extension so the link back to the chain is not lost.
    ///
    /// # Errors
    /// Returns [`DocError`] if the log does not verify, `seq` is out of range,
    /// or the entry's timestamp is not an RFC 3339 instant (FHIR `recorded`
    /// requires one).
    pub fn provenance(&self, seq: usize, target: &str) -> Result<serde_json::Value, DocError> {
        self.verify()?;
        let entry: &Entry = self
            .entries()
            .get(seq)
            .ok_or_else(|| DocError::invalid_format(format!("no audit entry {seq}")))?;
        if !looks_like_instant(&entry.at) {
            return Err(DocError::invalid_format(format!(
                "entry {seq} timestamp `{}` is not an RFC 3339 instant",
                entry.at
            )));
        }
        Ok(serde_json::json!({
            "resourceType": "Provenance",
            "target": [{ "reference": target }],
            "recorded": entry.at,
            "activity": { "text": entry.event },
            "entity": [{
                "role": "source",
                "what": { "display": format!("payload sha256 {}", entry.payload_sha256) }
            }],
            "extension": [{ "url": HASH_EXTENSION_URL, "valueString": entry.hash }],
        }))
    }
}

/// A cheap check for an RFC 3339 date-time: `YYYY-MM-DDThh:mm:ss` with an
/// offset or `Z`. It rejects the obvious non-instants; the full grammar is
/// the consumer's to enforce.
fn looks_like_instant(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && (bytes.ends_with(b"Z") || bytes.contains(&b'+') || bytes[19..].contains(&b'-'))
        && bytes[..4].iter().all(u8::is_ascii_digit)
}

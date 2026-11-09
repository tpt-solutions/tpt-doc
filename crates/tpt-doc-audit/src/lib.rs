//! A tamper-evident audit log: a SHA-256 hash chain over events, with the head
//! sealed by a CAdES signature.
//!
//! Each entry commits to the hash of the entry before it, so changing,
//! deleting, reordering or inserting any entry breaks every hash after it.
//! [`AuditLog::verify`] recomputes the chain, and [`AuditLog::from_jsonl`]
//! refuses a file whose chain does not check out.
//!
//! The head is the hash of the last entry (the genesis value for an empty
//! log). [`AuditLog::seal`] signs the head with a CAdES detached signature, so
//! anyone holding the head, the signature and the certificate can confirm that
//! the log has not changed since the seal.
//!
//! The caller supplies each entry's timestamp rather than the log reading a
//! clock, so the same events always produce the same chain.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt::Write as _;

use ring::digest::{SHA256, digest};
use ring::rand::SecureRandom;
use serde::{Deserialize, Serialize};
use tpt_doc_core::{DocError, Location};
use tpt_doc_sign::PrivateKey;

pub mod export;

/// The previous-hash value of the first entry: 64 hex zeros.
pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One recorded event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Position in the log, starting at 0.
    pub seq: u64,
    /// Caller-supplied timestamp, for example RFC 3339.
    pub at: String,
    /// What happened, for example `invoice.approved`.
    pub event: String,
    /// Hex SHA-256 of the event's payload. The payload itself is not stored,
    /// so the log can be kept without the data it describes.
    pub payload_sha256: String,
    /// The hash of the previous entry, or [`GENESIS`] for the first.
    pub prev: String,
    /// Hex SHA-256 over this entry's fields and `prev`.
    pub hash: String,
}

/// An append-only, hash-chained log of events.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditLog {
    entries: Vec<Entry>,
}

impl AuditLog {
    /// An empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The entries, in order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The hash of the last entry, or [`GENESIS`] for an empty log.
    #[must_use]
    pub fn head(&self) -> &str {
        self.entries
            .last()
            .map_or(GENESIS, |entry| entry.hash.as_str())
    }

    /// Record an event. The payload is hashed, not stored.
    ///
    /// # Errors
    /// Returns [`DocError`] if `event` is empty.
    pub fn append(&mut self, at: &str, event: &str, payload: &[u8]) -> Result<&Entry, DocError> {
        if event.is_empty() {
            return Err(DocError::invalid_format(
                "audit event name must not be empty",
            ));
        }
        let seq = self.entries.len() as u64;
        let prev = self.head().to_owned();
        let payload_sha256 = hex(digest(&SHA256, payload).as_ref());
        let hash = entry_hash(seq, at, event, &payload_sha256, &prev);
        self.entries.push(Entry {
            seq,
            at: at.to_owned(),
            event: event.to_owned(),
            payload_sha256,
            prev,
            hash,
        });
        Ok(self.entries.last().expect("an entry was just pushed"))
    }

    /// Recompute the chain and confirm every link.
    ///
    /// # Errors
    /// Returns [`DocError`] at the index of the first entry that does not
    /// check out: a wrong sequence number, a broken link to the previous hash,
    /// or a hash that does not match the entry's contents.
    pub fn verify(&self) -> Result<(), DocError> {
        let mut prev = GENESIS.to_owned();
        for (index, entry) in self.entries.iter().enumerate() {
            let fail = |message: String| {
                DocError::validation_failed(message).with_location(Location::Index(index))
            };
            if entry.seq != index as u64 {
                return Err(fail(format!(
                    "sequence number {} where {index} was expected",
                    entry.seq
                )));
            }
            if entry.prev != prev {
                return Err(fail("the link to the previous entry is broken".to_owned()));
            }
            let expected = entry_hash(
                entry.seq,
                &entry.at,
                &entry.event,
                &entry.payload_sha256,
                &entry.prev,
            );
            if entry.hash != expected {
                return Err(fail("the entry does not match its hash".to_owned()));
            }
            prev.clone_from(&entry.hash);
        }
        Ok(())
    }

    /// Serialize as JSON Lines: one entry per line.
    ///
    /// # Errors
    /// Returns [`DocError`] if an entry cannot be serialized.
    pub fn to_jsonl(&self) -> Result<String, DocError> {
        let mut out = String::new();
        for entry in &self.entries {
            let line = serde_json::to_string(entry)
                .map_err(|e| DocError::invalid_format(format!("cannot serialize entry: {e}")))?;
            out.push_str(&line);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parse JSON Lines and verify the chain.
    ///
    /// # Errors
    /// Returns [`DocError`] naming the line that is not a valid entry, or the
    /// entry where the chain is broken. A file that was edited, truncated, or
    /// reordered is refused.
    pub fn from_jsonl(text: &str) -> Result<Self, DocError> {
        let mut entries = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: Entry = serde_json::from_str(line).map_err(|e| {
                DocError::invalid_format(format!("line {} is not an entry: {e}", index + 1))
                    .with_location(Location::Index(index))
            })?;
            entries.push(entry);
        }
        let log = Self { entries };
        log.verify()?;
        Ok(log)
    }

    /// Sign the head with a CAdES detached signature.
    ///
    /// The log is verified first, so a broken log is never sealed. The seal
    /// covers the head's hex text; the signature is DER-encoded CMS.
    ///
    /// # Errors
    /// Returns [`DocError`] if the log does not verify, the certificate is not
    /// valid at `unix_seconds` or does not match `key`, or signing fails.
    pub fn seal(
        &self,
        key: &PrivateKey,
        cert_der: &[u8],
        rng: &dyn SecureRandom,
        unix_seconds: i64,
    ) -> Result<Vec<u8>, DocError> {
        self.verify()?;
        tpt_doc_sign::cades::sign_detached_at(
            self.head().as_bytes(),
            key,
            cert_der,
            rng,
            unix_seconds,
        )
    }
}

/// Hash one entry. Each field is length-prefixed, so two different entries
/// can never produce the same byte string.
fn entry_hash(seq: u64, at: &str, event: &str, payload_sha256: &str, prev: &str) -> String {
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&seq.to_be_bytes());
    for field in [at, event, payload_sha256, prev] {
        buffer.extend_from_slice(&(field.len() as u64).to_be_bytes());
        buffer.extend_from_slice(field.as_bytes());
    }
    hex(digest(&SHA256, &buffer).as_ref())
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

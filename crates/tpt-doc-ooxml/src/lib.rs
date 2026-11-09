//! Shared OOXML package handling for `tpt-doc-spreadsheet` and `tpt-doc-word`.
//!
//! Both XLSX and DOCX are ZIP containers of XML parts. This crate owns the two
//! things they have in common: writing a package with deflated entries, and
//! reading a part with the bounds from [`tpt_doc_core::Limits`].
//!
//! The XML declaration every part starts with is shared too, so the two
//! writers emit byte-identical headers.
//!
//! This crate is internal plumbing; the format crates re-export what users
//! need.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::io::{Cursor, Write};

use tpt_doc_core::{DocError, Limits};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The XML declaration that opens every OOXML part.
pub const XML_DECL: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

/// Open a ZIP container for reading.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if `bytes` is not a ZIP archive.
pub fn open_archive(bytes: &[u8]) -> Result<ZipArchive<Cursor<&[u8]>>, DocError> {
    ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| DocError::invalid_format(format!("not a valid ZIP archive: {e}")))
}

/// Read one named part, bounded by `limits.max_entry_bytes`.
///
/// # Errors
/// Returns [`DocError::InvalidFormat`] if the part is missing, or an error if
/// it exceeds the size limit.
pub fn read_part(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    limits: &Limits,
) -> Result<Vec<u8>, DocError> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| DocError::invalid_format(format!("missing ZIP entry `{name}`")))?;
    limits.read_to_end(&mut file)
}

/// Builds a ZIP package from named text parts.
///
/// Entries are written in the order they are added, deflate-compressed, so a
/// package built from the same parts is byte-identical every time.
pub struct PackageWriter {
    zip: ZipWriter<Cursor<Vec<u8>>>,
    options: SimpleFileOptions,
}

impl PackageWriter {
    /// Start an empty package.
    #[must_use]
    pub fn new() -> Self {
        Self {
            zip: ZipWriter::new(Cursor::new(Vec::new())),
            options: SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        }
    }

    /// Add one part at `name` (for example `word/document.xml`).
    ///
    /// # Errors
    /// Returns [`DocError`] if the entry cannot be started or written.
    pub fn add_part(&mut self, name: &str, content: &str) -> Result<(), DocError> {
        self.zip
            .start_file(name, self.options)
            .map_err(|e| DocError::invalid_format(format!("ZIP entry `{name}` failed: {e}")))?;
        self.zip.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Finish the package and return its bytes.
    ///
    /// # Errors
    /// Returns [`DocError`] if the central directory cannot be written.
    pub fn finish(self) -> Result<Vec<u8>, DocError> {
        let cursor = self
            .zip
            .finish()
            .map_err(|e| DocError::invalid_format(format!("ZIP finalization failed: {e}")))?;
        Ok(cursor.into_inner())
    }
}

impl Default for PackageWriter {
    fn default() -> Self {
        Self::new()
    }
}

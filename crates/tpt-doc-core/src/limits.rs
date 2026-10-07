//! Resource limits applied when parsing untrusted documents.

use alloc::format;

use crate::error::DocError;

/// Bounds applied when reading untrusted documents.
///
/// Every parser in this workspace decompresses or buffers data whose size is
/// declared by the input rather than chosen by the caller. A small file can
/// declare an enormous payload, so these limits stop a malicious document from
/// exhausting memory before it is rejected.
///
/// The defaults are generous enough for real documents (a 200 MB uncompressed
/// worksheet, a 100 MB ZIP entry) while keeping the worst case bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum size of a single decompressed entry (ZIP member, PNG IDAT, XML
    /// document). Defaults to 200 MB.
    pub max_entry_bytes: usize,
    /// Maximum number of rows a tabular reader will yield before erroring.
    /// Defaults to 1,000,000.
    pub max_rows: usize,
    /// Maximum depth of nested XML elements before erroring. Defaults to 256.
    pub max_xml_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entry_bytes: 200 * 1024 * 1024,
            max_rows: 1_000_000,
            max_xml_depth: 256,
        }
    }
}

impl Limits {
    /// Read a stream into a `Vec`, erroring once it exceeds
    /// [`Limits::max_entry_bytes`].
    ///
    /// This is a bounded replacement for `Read::read_to_end`, which allocates
    /// whatever the stream claims it contains.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if the limit is exceeded, or the
    /// underlying I/O error.
    #[cfg(feature = "std")]
    pub fn read_to_end<R: std::io::Read>(&self, reader: &mut R) -> Result<Vec<u8>, DocError> {
        use std::io::Read as _;

        // Read one byte past the limit so an exactly-at-limit stream succeeds
        // while an over-limit one is detected without unbounded growth.
        let mut limited = reader.take(self.max_entry_bytes as u64 + 1);
        let mut bytes = Vec::new();
        limited.read_to_end(&mut bytes).map_err(DocError::Io)?;
        if bytes.len() > self.max_entry_bytes {
            return Err(DocError::invalid_format(format!(
                "decompressed data exceeds the {} byte limit",
                self.max_entry_bytes
            )));
        }
        Ok(bytes)
    }

    /// Error if `rows` exceeds [`Limits::max_rows`].
    ///
    /// Callers pass the running row count so the check is a single comparison
    /// per row rather than a second accumulator.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if the limit is exceeded.
    pub fn check_rows(&self, rows: usize) -> Result<(), DocError> {
        if rows > self.max_rows {
            return Err(DocError::invalid_format(format!(
                "row count exceeds the {} row limit",
                self.max_rows
            )));
        }
        Ok(())
    }

    /// Error if `depth` exceeds [`Limits::max_xml_depth`].
    ///
    /// `depth` is the number of currently open elements. Deeply nested XML is
    /// cheap to write and expensive to recurse over, so a parser must stop
    /// before its own stack is the limit.
    ///
    /// # Errors
    /// Returns [`DocError::InvalidFormat`] if the limit is exceeded.
    pub fn check_xml_depth(&self, depth: usize) -> Result<(), DocError> {
        if depth > self.max_xml_depth {
            return Err(DocError::invalid_format(format!(
                "XML nesting depth exceeds the {} level limit",
                self.max_xml_depth
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_generous_but_bounded() {
        let limits = Limits::default();
        assert!(limits.max_entry_bytes > 0);
        assert!(limits.max_rows > 0);
        assert!(limits.max_xml_depth > 0);
    }

    #[test]
    fn reads_a_stream_within_the_limit() {
        let limits = Limits {
            max_entry_bytes: 16,
            ..Limits::default()
        };
        let mut input = std::io::Cursor::new(b"0123456789".to_vec());
        assert_eq!(
            limits.read_to_end(&mut input).expect("under limit"),
            b"0123456789"
        );
    }

    /// A stream exactly at the limit must succeed; the sentinel byte only
    /// exists to detect overflow.
    #[test]
    fn a_stream_exactly_at_the_limit_succeeds() {
        let limits = Limits {
            max_entry_bytes: 8,
            ..Limits::default()
        };
        let mut input = std::io::Cursor::new(b"12345678".to_vec());
        assert_eq!(limits.read_to_end(&mut input).expect("at limit").len(), 8);
    }

    /// This is the zip-bomb case: a stream that claims far more than the limit.
    #[test]
    fn a_stream_over_the_limit_is_rejected() {
        let limits = Limits {
            max_entry_bytes: 8,
            ..Limits::default()
        };
        let mut input = std::io::Cursor::new(vec![b'x'; 64]);
        let err = limits.read_to_end(&mut input).expect_err("over limit");
        assert!(err.to_string().contains("exceeds"), "{err}");
    }

    struct Endless;

    impl std::io::Read for Endless {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            buf.fill(b'z');
            Ok(buf.len())
        }
    }

    /// Only the declared limit is read, so an endless stream cannot exhaust
    /// memory before the check runs.
    #[test]
    fn an_endless_stream_is_bounded() {
        let limits = Limits {
            max_entry_bytes: 1024,
            ..Limits::default()
        };
        let mut reader = Endless;
        let err = limits
            .read_to_end(&mut reader)
            .expect_err("must stop at the limit");
        assert!(err.to_string().contains("exceeds"), "{err}");
    }

    #[test]
    fn empty_input_is_fine() {
        let mut input = std::io::Cursor::new(Vec::new());
        assert!(
            Limits::default()
                .read_to_end(&mut input)
                .expect("empty")
                .is_empty()
        );
    }

    /// A row count at the limit is the last acceptable value; the next one is
    /// refused before another row is materialised.
    #[test]
    fn rows_up_to_the_limit_are_accepted() {
        let limits = Limits {
            max_rows: 10,
            ..Limits::default()
        };
        assert!(limits.check_rows(10).is_ok());
        let err = limits.check_rows(11).expect_err("over the row limit");
        assert!(err.to_string().contains("10 row limit"), "{err}");
    }

    /// The depth check is what stops a nesting bomb before the parser recurses.
    #[test]
    fn xml_depth_up_to_the_limit_is_accepted() {
        let limits = Limits {
            max_xml_depth: 4,
            ..Limits::default()
        };
        assert!(limits.check_xml_depth(4).is_ok());
        let err = limits.check_xml_depth(5).expect_err("over the depth limit");
        assert!(err.to_string().contains("4 level limit"), "{err}");
    }
}

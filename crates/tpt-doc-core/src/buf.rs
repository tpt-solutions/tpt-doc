/// A zero-copy borrowed byte slice.
///
/// `BufSlice<'a>` is a newtype over `&'a [u8]` that makes the "no allocation
/// was made to produce these bytes" guarantee explicit in the type system.
/// Parsers that return `BufSlice` promise the data is a view into the original
/// input, not a copy.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufSlice<'a>(&'a [u8]);

impl<'a> BufSlice<'a> {
    /// Create a new `BufSlice` from a byte slice.
    #[inline]
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    /// Return the underlying byte slice.
    #[inline]
    #[must_use]
    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }

    /// Length of the slice in bytes.
    #[inline]
    #[must_use]
    pub const fn len(self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the slice is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    /// Split at `mid`, returning two `BufSlice`s.
    ///
    /// # Panics
    /// Panics if `mid > self.len()`.
    #[inline]
    #[must_use]
    pub fn split_at(self, mid: usize) -> (Self, Self) {
        let (a, b) = self.0.split_at(mid);
        (Self(a), Self(b))
    }
}

impl<'a> From<&'a [u8]> for BufSlice<'a> {
    fn from(b: &'a [u8]) -> Self {
        Self(b)
    }
}

impl<'a> From<&'a str> for BufSlice<'a> {
    fn from(s: &'a str) -> Self {
        Self(s.as_bytes())
    }
}

impl core::fmt::Debug for BufSlice<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BufSlice({} bytes)", self.0.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let data = b"hello";
        let buf = BufSlice::new(data);
        assert_eq!(buf.as_bytes(), data);
        assert_eq!(buf.len(), 5);
        assert!(!buf.is_empty());
    }

    #[test]
    fn split() {
        let buf = BufSlice::new(b"abcdef");
        let (a, b) = buf.split_at(3);
        assert_eq!(a.as_bytes(), b"abc");
        assert_eq!(b.as_bytes(), b"def");
    }

    #[test]
    fn from_str() {
        let buf: BufSlice<'_> = "hello".into();
        assert_eq!(buf.as_bytes(), b"hello");
    }

    #[test]
    fn empty_slice() {
        let buf = BufSlice::new(b"");
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    #[should_panic(expected = "mid > len")]
    fn split_at_beyond_len_panics() {
        let _ = BufSlice::new(b"abc").split_at(4);
    }

    #[test]
    fn is_copy_and_reference_semantics_preserve_identity() {
        let data = b"shared";
        let a = BufSlice::new(data);
        let b = a;
        assert_eq!(a.as_bytes().as_ptr(), b.as_bytes().as_ptr());
        assert_eq!(a, b);
    }

    #[test]
    fn debug_reports_byte_count_only() {
        let buf = BufSlice::new(b"secret-contents");
        assert_eq!(std::format!("{buf:?}"), "BufSlice(15 bytes)");
    }

    #[test]
    fn split_covers_whole_input() {
        let buf = BufSlice::new(b"abcdef");
        let (a, b) = buf.split_at(0);
        assert_eq!(a.as_bytes(), b"");
        assert_eq!(b.as_bytes(), b"abcdef");
        let (a, b) = buf.split_at(6);
        assert_eq!(a.as_bytes(), b"abcdef");
        assert_eq!(b.as_bytes(), b"");
    }

    #[test]
    fn hash_and_eq_track_content() {
        use core::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let hash = |b: BufSlice<'_>| {
            let mut h = DefaultHasher::new();
            b.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(BufSlice::new(b"x")), hash(BufSlice::new(b"x")));
        assert_ne!(hash(BufSlice::new(b"x")), hash(BufSlice::new(b"y")));
    }
}

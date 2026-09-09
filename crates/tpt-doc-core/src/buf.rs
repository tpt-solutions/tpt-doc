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
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    /// Return the underlying byte slice.
    #[inline]
    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }

    /// Length of the slice in bytes.
    #[inline]
    pub const fn len(self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the slice is empty.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    /// Split at `mid`, returning two `BufSlice`s.
    ///
    /// # Panics
    /// Panics if `mid > self.len()`.
    #[inline]
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
}

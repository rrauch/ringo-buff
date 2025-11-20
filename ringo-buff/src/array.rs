use crate::{CircularBuffer, Storage};

impl<const N: usize> crate::sealed::StorageBase for [u8; N] {}

impl<const N: usize> Storage for [u8; N] {
    fn len(&self) -> usize {
        N
    }

    fn as_slice(&self) -> &[u8] {
        self.as_slice()
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }

    fn split_at(&self, offset: usize) -> (&[u8], &[u8]) {
        self.as_slice().split_at(offset)
    }

    fn split_at_mut(&mut self, offset: usize) -> (&mut [u8], &mut [u8]) {
        self.as_mut_slice().split_at_mut(offset)
    }
}

/// A circular buffer backed by a stack-allocated array `[u8; N]`.
///
/// This implementation is used when the `hybrid-array` feature is **disabled**.
pub type StackBuffer<const N: usize> = CircularBuffer<[u8; N]>;

impl<const N: usize> StackBuffer<N> {
    /// Creates a new buffer with capacity `N`.
    ///
    /// # Panics
    ///
    /// Panics if `N` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringo_buff::StackBuffer;
    ///
    /// let buf: StackBuffer<128> = StackBuffer::new();
    /// assert_eq!(buf.capacity(), 128);
    /// ```
    pub fn new() -> Self {
        assert!(N > 0);
        Self::_new_with_storage([0u8; N])
    }
}

use crate::{CircularBuffer, Storage};
use ::hybrid_array::Array;

pub use ::hybrid_array::ArraySize;

impl<N: ArraySize> crate::sealed::StorageBase for Array<u8, N> {}

impl<N: ArraySize> Storage for Array<u8, N> {
    fn len(&self) -> usize {
        self.as_slice().len()
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

/// A circular buffer backed by `hybrid_array::Array`.
///
/// This implementation is used when the `hybrid-array` feature is **enabled**.
/// It allows for type-level sizes (e.g., `U10`) compatible with `hybrid-array` & `typenum`.
pub type StackBuffer<N> = CircularBuffer<Array<u8, N>>;

impl<N: ArraySize> StackBuffer<N> {
    /// Creates a new buffer.
    ///
    /// # Panics
    ///
    /// Panics if array size is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringo_buff::{StackBuffer, ArraySize};
    /// use hybrid_array::sizes::U128;
    ///
    /// let buf: StackBuffer<U128> = StackBuffer::new();
    /// assert_eq!(buf.capacity(), 128);
    /// ```
    pub fn new() -> Self {
        assert!(N::to_usize() > 0);
        Self::_new_with_storage(Array::default())
    }
}

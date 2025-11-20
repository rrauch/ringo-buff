use crate::{CircularBuffer, Storage};

impl crate::sealed::StorageBase for Vec<u8> {}

impl Storage for Vec<u8> {
    fn len(&self) -> usize {
        self.len()
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

/// A circular buffer backed by a heap-allocated `Vec<u8>`.
///
/// Requires the `alloc` feature (enabled by default).
pub type HeapBuffer = CircularBuffer<Vec<u8>>;

impl HeapBuffer {
    /// Creates a new buffer with the specified capacity.
    ///
    /// This allocates a `Vec` of `capacity` bytes filled with zeros.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringo_buff::HeapBuffer;
    ///
    /// let buf = HeapBuffer::new(1024);
    /// assert_eq!(buf.capacity(), 1024);
    /// ```
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self::_new_with_storage(vec![0u8; capacity])
    }

    /// Attempts to resize the buffer to a `new_capacity`.
    ///
    /// # Growing
    /// If `new_capacity` is larger than the current capacity, the buffer is extended.
    ///
    /// # Shrinking
    /// If `new_capacity` is smaller, the buffer attempts to shrink.
    /// * This operation forces the buffer to become contiguous (see [`make_contiguous`](CircularBuffer::make_contiguous)).
    /// * The operation fails if the active data (`remaining()`) is larger than `new_capacity`.
    ///
    /// # Errors
    ///
    /// Returns `Err(usize)` if the buffer cannot be shrunk because it currently holds too much data.
    /// The error value is the number of bytes that must be consumed (read) before this resize can succeed.
    ///
    /// # Panics
    ///
    /// Panics if `new_capacity` is 0.
    pub fn try_resize(&mut self, new_capacity: usize) -> Result<(), usize> {
        assert!(new_capacity > 0);
        if new_capacity >= self.capacity() {
            // growing
            self.bytes.resize(new_capacity, 0u8);
        } else {
            // shrinking
            if self.remaining() > new_capacity {
                // not enough empty space to shrink
                let extra_bytes_needed = self.remaining() - new_capacity;
                return Err(extra_bytes_needed);
            }
            self.make_contiguous();
            self.bytes.truncate(new_capacity);
        }
        Ok(())
    }
}

impl From<Vec<u8>> for HeapBuffer {
    /// Converts an existing `Vec<u8>` into a `HeapBuffer`, reusing the `Vec` as the storage mechanism for the buffer.
    fn from(value: Vec<u8>) -> Self {
        assert!(value.capacity() > 0);
        Self::_new_with_storage(value)
    }
}

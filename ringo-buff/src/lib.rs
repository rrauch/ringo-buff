#![cfg_attr(docsrs, feature(doc_cfg))]
//! # ringo-buff
//!
//! Ring buffers for bytes, with heap and stack storage.
//!
//! A simple, predictable ring buffer (circular buffer) implementation supporting both
//! stack and heap storage strategies.
//!
//! This crate provides a simple API for managing bytes in a circular buffer, useful for
//! I/O buffering, streaming data, etc. It can be used without allocating
//! and should be usable in a `no_std` environment.
//!
//! ## Quick Start
//!
//! Choose your storage strategy:
//!
//! - **[`HeapBuffer`]**: Backed by a `Vec<u8>`. Best for dynamic sizes or large buffers. Supports resizing at runtime.
//! - **[`StackBuffer`]**: Backed by an array. Best for fixed, small sizes where allocation is undesirable.
//!
//! ```rust
//! use ringo_buff::HeapBuffer;
//!
//! // Create a buffer with 1024 bytes capacity
//! let mut buf = HeapBuffer::new(1024);
//!
//! // Write data (advances write cursor)
//! let (first, _) = buf.as_mut_slices();
//! first[..3].copy_from_slice(b"foo");
//! buf.commit(3);
//!
//! // Read data (advances read cursor)
//! let (first, _) = buf.as_slices();
//! assert_eq!(first, b"foo");
//! buf.consume(3);
//! ```
//!
//! ## Feature Flags
//!
//! - **`alloc`** *(default)*: Enables [`HeapBuffer`] and `Vec` support.
//! - **`buf-trait`**: Implements [`bytes::Buf`] and [`bytes::BufMut`] traits.
//! - **`zeroize`**: Clears memory on drop via the [`zeroize`] crate.
//! - **`hybrid-array`**: Enables `hybrid-array` support for [`StackBuffer`], allowing
//!   sizes to be defined via types (e.g., `U1024`).

#[cfg(not(feature = "hybrid-array"))]
mod array;
#[cfg(not(feature = "hybrid-array"))]
pub use array::StackBuffer;

#[cfg(feature = "hybrid-array")]
mod hybrid_array;
#[cfg(feature = "hybrid-array")]
pub use hybrid_array::ArraySize;
#[cfg(feature = "hybrid-array")]
pub use hybrid_array::StackBuffer;

#[cfg(feature = "alloc")]
mod vec;
#[cfg(feature = "alloc")]
pub use vec::HeapBuffer;

#[cfg(feature = "buf-trait")]
use bytes::buf::UninitSlice;
#[cfg(feature = "buf-trait")]
use bytes::{Buf, BufMut};

#[cfg(feature = "zeroize")]
use zeroize::{Zeroize, ZeroizeOnDrop};

#[doc(hidden)]
mod sealed {
    #[cfg(not(feature = "zeroize"))]
    pub trait StorageBase: Send {}
    #[cfg(feature = "zeroize")]
    pub trait StorageBase: Send + zeroize::Zeroize {}
}

#[doc(hidden)]
/// Abstract storage backend for the circular buffer.
///
/// This trait is sealed and cannot be implemented by downstream crates.
pub trait Storage: sealed::StorageBase {
    fn len(&self) -> usize;
    fn as_slice(&self) -> &[u8];
    fn as_mut_slice(&mut self) -> &mut [u8];
    fn split_at(&self, offset: usize) -> (&[u8], &[u8]);
    fn split_at_mut(&mut self, offset: usize) -> (&mut [u8], &mut [u8]);
}

/// The core ring buffer logic generic over storage `S`.
///
/// Users should instantiate this via the type aliases [`HeapBuffer`] or [`StackBuffer`].
pub struct CircularBuffer<S: Storage> {
    bytes: S,
    size: usize,
    start: usize,
}

impl<S: Storage> CircularBuffer<S> {
    fn _new_with_storage(bytes: S) -> Self {
        Self {
            bytes,
            size: 0,
            start: 0,
        }
    }

    /// Returns the total capacity of the buffer.
    ///
    /// This is the maximum number of bytes the buffer can hold at once.
    pub fn capacity(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `true` if the buffer contains no readable bytes.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Returns the number of bytes currently available to read.
    pub fn remaining(&self) -> usize {
        self.size
    }

    /// Advances the read position by consuming `cnt` bytes.
    ///
    /// # Panics
    ///
    /// Panics if `cnt` is greater than [`remaining()`](Self::remaining).
    ///
    /// # Example
    ///
    /// ```
    /// use ringo_buff::HeapBuffer;
    /// let mut buf = HeapBuffer::new(10);
    /// buf.commit(5); // Assume 5 bytes written
    ///
    /// buf.consume(2);
    /// assert_eq!(buf.remaining(), 3);
    /// ```
    pub fn consume(&mut self, cnt: usize) {
        assert!(cnt <= self.size, "attempt to consume beyond available data");
        if cnt == 0 {
            return;
        }
        let capacity = self.bytes.len();
        debug_assert!(self.start < capacity, "start out-of-bounds");
        self.start = add_mod(self.start, cnt, capacity);
        self.size -= cnt;
    }

    /// Resets the buffer to an empty state.
    ///
    /// This resets the read/write cursors. It does not zero out the underlying memory
    /// unless the `zeroize` feature is enabled and `zeroize()` is called explicitly.
    pub fn reset(&mut self) {
        self.size = 0;
        self.start = 0;
    }

    /// Returns `true` if the buffer has no space for writing.
    pub fn is_full(&self) -> bool {
        self.size == self.bytes.len()
    }

    /// Returns the number of bytes available for writing.
    pub fn remaining_mut(&self) -> usize {
        self.bytes.len() - self.size
    }

    /// Advances the write position by committing `cnt` bytes.
    ///
    /// This should be called after writing data into the slices returned by [`as_mut_slices`](Self::as_mut_slices).
    ///
    /// # Panics
    ///
    /// Panics if `cnt` is greater than [`remaining_mut()`](Self::remaining_mut).
    pub fn commit(&mut self, cnt: usize) {
        assert!(
            cnt <= self.remaining_mut(),
            "attempt to advance beyond available space"
        );
        if cnt == 0 {
            return;
        }
        self.size += cnt;
    }

    /// Returns pairs of slices representing the readable data.
    ///
    /// Because the data is in a ring buffer, it may wrap around the end of the underlying
    /// storage.
    ///
    /// - If the data is contiguous, the second slice will be empty.
    /// - If the data wraps, the first slice contains the data up to the end of the buffer,
    ///   and the second slice contains the data from the start of the buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use ringo_buff::HeapBuffer;
    /// let mut buf = HeapBuffer::new(5);
    /// // Simulate wrapping: write 3, consume 2, write 3
    /// buf.commit(3);
    /// buf.consume(2);
    /// buf.commit(3);
    ///
    /// let (a, b) = buf.as_slices();
    /// // 'a' is at the end of memory, 'b' is at the start
    /// assert!(!a.is_empty());
    /// assert!(!b.is_empty());
    /// ```
    pub fn as_slices(&self) -> (&[u8], &[u8]) {
        let capacity = self.bytes.len();

        if capacity == 0 || self.is_empty() {
            return (&[], &[]);
        }

        debug_assert!(self.start < capacity, "start out-of-bounds");
        debug_assert!(self.size <= capacity, "size out-of-bounds");

        let start = self.start;
        let end = add_mod(self.start, self.size, capacity);

        if start < end {
            (&self.bytes.as_slice()[start..end], &[][..])
        } else {
            let (back, front) = self.bytes.split_at(start);
            (front, &back[..end])
        }
    }

    /// Returns pairs of mutable slices representing the writable space.
    ///
    /// Similar to [`as_slices`](Self::as_slices), the available space might be split
    /// into two parts if the read cursor is in the middle of the buffer.
    ///
    /// Write data to these slices, then call [`commit`](Self::commit) to make the data available for reading.
    pub fn as_mut_slices(&mut self) -> (&mut [u8], &mut [u8]) {
        let capacity = self.bytes.len();

        if capacity == 0 || self.size == capacity {
            return (&mut [][..], &mut [][..]);
        }

        debug_assert!(self.start < capacity, "start out-of-bounds");
        debug_assert!(self.size <= capacity, "size out-of-bounds");

        let write_start = add_mod(self.start, self.size, capacity);
        let available = capacity - self.size;
        let write_end = add_mod(write_start, available, capacity);

        if write_start < write_end {
            (
                &mut self.bytes.as_mut_slice()[write_start..write_end],
                &mut [][..],
            )
        } else {
            let (back, front) = self.bytes.split_at_mut(write_start);
            (front, &mut back[..write_end])
        }
    }

    /// Rotates the buffer contents to make the readable data contiguous.
    ///
    /// Returns a single slice containing all readable data.
    ///
    /// # Performance
    ///
    /// If the data is already contiguous, this is a no-op.
    /// If the data wraps around, this performs a memory rotation (O(N)).
    pub fn make_contiguous(&mut self) -> &[u8] {
        let capacity = self.bytes.len();

        if capacity == 0 || self.size == 0 {
            return &[];
        }

        debug_assert!(self.start < capacity, "start out-of-bounds");
        debug_assert!(self.size <= capacity, "size out-of-bounds");

        let start = self.start;
        let end = add_mod(self.start, self.size, capacity);

        if start < end {
            // Already contiguous; nothing to do
            &self.bytes.as_slice()[start..end]
        } else {
            // Not contiguous; need to rotate
            self.start = 0;
            self.bytes.as_mut_slice().rotate_left(start);
            &self.bytes.as_slice()[..self.size]
        }
    }
}

/// Returns `(x + y) % m` without risk of overflows if `x + y` cannot fit in `usize`.
///
/// `x` and `y` are expected to be less than, or equal to `m`.
#[inline]
const fn add_mod(x: usize, y: usize, m: usize) -> usize {
    debug_assert!(m > 0);
    debug_assert!(x <= m);
    debug_assert!(y <= m);
    let (z, overflow) = x.overflowing_add(y);
    (z + (overflow as usize) * (usize::MAX % m + 1)) % m
}

#[cfg(feature = "zeroize")]
impl<S: Storage> ZeroizeOnDrop for CircularBuffer<S> {}

#[cfg(feature = "zeroize")]
impl<S: Storage> Drop for CircularBuffer<S> {
    fn drop(&mut self) {
        self.bytes.zeroize()
    }
}

#[cfg(feature = "zeroize")]
impl<S: Storage> Zeroize for CircularBuffer<S> {
    /// Zeroes out the underlying storage and resets cursors.
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.reset();
    }
}

#[cfg(feature = "buf-trait")]
impl<S: Storage> Buf for CircularBuffer<S> {
    fn remaining(&self) -> usize {
        self.remaining()
    }

    fn chunk(&self) -> &[u8] {
        let (first, second) = self.as_slices();
        if !first.is_empty() { first } else { second }
    }

    fn advance(&mut self, cnt: usize) {
        self.consume(cnt);
    }
}

#[cfg(feature = "buf-trait")]
unsafe impl<S: Storage> BufMut for CircularBuffer<S> {
    fn remaining_mut(&self) -> usize {
        self.remaining_mut()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.commit(cnt)
    }

    fn chunk_mut(&mut self) -> &mut UninitSlice {
        let (first, second) = self.as_mut_slices();
        let slice = if !first.is_empty() { first } else { second };

        UninitSlice::new(slice)
    }
}

#[cfg(test)]
mod tests {
    use crate::{CircularBuffer, HeapBuffer, StackBuffer};
    use bytes::{Buf, BufMut};
    use hybrid_array::sizes::{U2, U3, U4, U5, U8, U10, U64};
    use std::cmp::min;
    use std::ptr;

    static ONE_MB: &'static [u8] = include_bytes!("../testdata/1mb.bin");

    macro_rules! test_all_impls {
        ($test_name:ident, $test_body:expr) => {
            #[test]
            fn $test_name() {
                // Test heap implementation
                {
                    let buf: HeapBuffer = HeapBuffer::new(10);
                    $test_body(buf);
                }

                // Test stack implementation
                {
                    let buf: StackBuffer<U10> = StackBuffer::new();
                    $test_body(buf);
                }
            }
        };
    }

    macro_rules! test_all_impls_custom_size {
        ($test_name:ident, $size:expr, $stack_size:ty, $test_body:expr) => {
            #[test]
            fn $test_name() {
                // Test heap implementation
                {
                    let buf: HeapBuffer = HeapBuffer::new($size);
                    $test_body(buf);
                }

                // Test stack implementation
                {
                    let buf: StackBuffer<$stack_size> = StackBuffer::new();
                    $test_body(buf);
                }
            }
        };
    }

    test_all_impls!(test_empty_buffer, |buf: CircularBuffer<_>| {
        assert!(buf.is_empty());
        assert_eq!(buf.remaining(), 0);
        assert_eq!(buf.remaining_mut(), buf.capacity());
        assert!(!buf.is_full());
    });

    test_all_impls_custom_size!(test_full_buffer, 5, U5, |mut buf: CircularBuffer<_>| {
        buf.commit(5);
        assert!(buf.is_full());
        assert_eq!(buf.remaining(), 5);
        assert_eq!(buf.remaining_mut(), 0);
    });

    test_all_impls_custom_size!(test_consume_partial, 5, U5, |mut buf: CircularBuffer<_>| {
        buf.commit(3);
        buf.consume(2);
        assert_eq!(buf.remaining(), 1);
        assert_eq!(buf.remaining_mut(), 4);
    });

    #[test]
    #[cfg(not(debug_assertions))]
    #[should_panic]
    fn test_consume_too_much() {
        let mut buf = HeapBuffer::new(5);
        buf.commit(3);
        buf.consume(4);
    }

    #[test]
    #[cfg(not(debug_assertions))]
    #[should_panic]
    fn test_zero_capacity_stack() {
        let _buf: StackBuffer<hybrid_array::sizes::U0> = StackBuffer::new();
    }

    #[test]
    #[cfg(not(debug_assertions))]
    #[should_panic]
    fn test_zero_capacity_heap() {
        let _buf: HeapBuffer = HeapBuffer::new(0);
    }

    test_all_impls_custom_size!(test_wrap_around_read, 3, U3, |mut buf: CircularBuffer<
        _,
    >| {
        buf.commit(3);
        buf.consume(2);
        buf.commit(2);

        let (slice1, slice2) = buf.as_slices();
        assert_eq!(slice1.len() + slice2.len(), 3);
        assert!(!slice1.is_empty());
        assert!(!slice2.is_empty());
    });

    test_all_impls_custom_size!(test_wrap_around_write, 3, U3, |mut buf: CircularBuffer<
        _,
    >| {
        buf.commit(2);
        buf.consume(2);
        buf.commit(2);

        let (slice1, slice2) = buf.as_slices();
        assert_eq!(slice1.len() + slice2.len(), 2);
    });

    test_all_impls_custom_size!(test_reset, 5, U5, |mut buf: CircularBuffer<_>| {
        buf.commit(3);
        buf.consume(1);
        buf.reset();

        assert!(buf.is_empty());
        assert_eq!(buf.start, 0);
        assert_eq!(buf.remaining_mut(), buf.capacity());
    });

    test_all_impls_custom_size!(test_mut_slices_wrap, 4, U4, |mut buf: CircularBuffer<_>| {
        buf.commit(4);
        buf.consume(3);
        buf.commit(2);

        let (slice1, slice2) = buf.as_mut_slices();
        assert_eq!(slice1.len() + slice2.len(), 1);
    });

    test_all_impls_custom_size!(
        test_exact_capacity_usage,
        2,
        U2,
        |mut buf: CircularBuffer<_>| {
            buf.commit(2);
            buf.consume(2);
            buf.commit(2);

            assert!(buf.is_full());
            assert_eq!(buf.remaining(), 2);
        }
    );

    #[test]
    fn test_data_integrity_through_circular_buffer() {
        let input_data: &[u8] = ONE_MB;
        let mut circular_buffer = HeapBuffer::new(64 * 1024);
        let mut output = Vec::new();
        let mut input_pos = 0;

        while input_pos < input_data.len() {
            let (first_mut, second_mut) = circular_buffer.as_mut_slices();
            let mut written = 0;

            if !first_mut.is_empty() {
                let to_copy = std::cmp::min(first_mut.len(), input_data.len() - input_pos);
                first_mut[..to_copy].copy_from_slice(&input_data[input_pos..input_pos + to_copy]);
                written += to_copy;
                input_pos += to_copy;
            }

            if !second_mut.is_empty() && input_pos < input_data.len() {
                let to_copy = std::cmp::min(second_mut.len(), input_data.len() - input_pos);
                second_mut[..to_copy].copy_from_slice(&input_data[input_pos..input_pos + to_copy]);
                written += to_copy;
                input_pos += to_copy;
            }

            circular_buffer.commit(written);

            let (first, second) = circular_buffer.as_slices();
            if !first.is_empty() {
                output.extend_from_slice(first);
            }
            if !second.is_empty() {
                output.extend_from_slice(second);
            }

            circular_buffer.consume(circular_buffer.remaining());
        }

        assert_eq!(input_data, output.as_slice(), "Data corruption detected!");
    }

    test_all_impls_custom_size!(
        test_data_integrity_through_circular_buffer_buf_traits,
        64,
        U64,
        |mut buf: CircularBuffer<_>| {
            let input_data: &[u8] =
                b"your_test_data_here_with_some_longer_content_to_test_wrapping";
            let mut output = Vec::new();
            let mut input_pos = 0;

            while input_pos < input_data.len() {
                while buf.remaining_mut() > 0 && input_pos < input_data.len() {
                    let chunk = buf.chunk_mut();
                    let to_copy = min(chunk.len(), input_data.len() - input_pos);

                    unsafe {
                        ptr::copy_nonoverlapping(
                            input_data[input_pos..].as_ptr(),
                            chunk.as_mut_ptr(),
                            to_copy,
                        );
                        buf.advance_mut(to_copy);
                    }
                    input_pos += to_copy;
                }

                while buf.remaining() > 0 {
                    let chunk = buf.chunk();
                    output.extend_from_slice(chunk);
                    let chunk_len = chunk.len();
                    buf.advance(chunk_len);
                }
            }

            assert_eq!(input_data, output.as_slice(), "Data corruption detected!");
        }
    );

    test_all_impls_custom_size!(
        test_as_mut_slices_returns_writable_space,
        8,
        U8,
        |mut buf: CircularBuffer<_>| {
            let (first, second) = buf.as_mut_slices();
            assert_eq!(
                first.len() + second.len(),
                8,
                "Empty buffer should have full capacity writable"
            );

            first[0..3].copy_from_slice(b"abc");
            buf.commit(3);

            let (first, second) = buf.as_mut_slices();
            assert_eq!(
                first.len() + second.len(),
                5,
                "After writing 3 bytes, should have 5 writable"
            );

            let total_writable = first.len() + second.len();
            if !first.is_empty() {
                first.fill(b'x');
            }
            if !second.is_empty() {
                second.fill(b'y');
            }
            buf.commit(total_writable);

            let (first, second) = buf.as_mut_slices();
            assert_eq!(
                first.len() + second.len(),
                0,
                "Full buffer should have no writable space"
            );

            buf.consume(2);

            let (first, second) = buf.as_mut_slices();
            assert_eq!(
                first.len() + second.len(),
                2,
                "After consuming 2 bytes, should have 2 writable"
            );
        }
    );

    test_all_impls_custom_size!(test_make_contiguous, 5, U5, |mut buf: CircularBuffer<_>| {
        // Case 1: Empty buffer
        assert_eq!(buf.make_contiguous(), &[]);

        // Case 2: Contiguous data (no wrap)
        // Fill: [1, 2, 3]
        let (first, _) = buf.as_mut_slices();
        first[0..3].copy_from_slice(&[1, 2, 3]);
        buf.commit(3);

        assert_eq!(buf.make_contiguous(), &[1, 2, 3]);

        // Case 3: Wrapped data
        // Setup: Fill buffer completely [1, 2, 3, 4, 5]
        // Current state: [1, 2, 3, _, _], start=0, size=3
        let (first, _) = buf.as_mut_slices();
        // 'first' is the remaining writable space (len 2).
        // We copy directly into it, not at offset 2.
        first.copy_from_slice(&[4, 5]);
        buf.commit(2);

        // State: [1, 2, 3, 4, 5], start=0, size=5 (Full)

        // Consume first 2 bytes to advance 'start' index to 2
        buf.consume(2);
        // State: [_, _, 3, 4, 5], start=2, size=3

        // Write [6, 7]. This will wrap around to the beginning.
        // remaining_mut = 2.
        // Write head is at (2+3)%5 = 0.
        // as_mut_slices should return buf[0..2] as 'first'.
        let (first, second) = buf.as_mut_slices();
        assert_eq!(first.len(), 2);
        assert!(second.is_empty());

        first.copy_from_slice(&[6, 7]);
        buf.commit(2);

        // State: [6, 7, 3, 4, 5] (Physical)
        // Logical: [3, 4, 5, 6, 7]
        // start=2, size=5.

        // Verify pre-condition: buffer is physically split
        let (_, s2) = buf.as_slices();
        assert!(
            !s2.is_empty(),
            "Test setup failed: Buffer should be wrapped"
        );

        // Action: Make contiguous
        let contiguous_slice = buf.make_contiguous();

        // Assertions
        assert_eq!(contiguous_slice, &[3, 4, 5, 6, 7]);

        // Verify internal state is physically contiguous (second slice empty)
        let (s1, s2) = buf.as_slices();
        assert_eq!(s1, &[3, 4, 5, 6, 7]);
        assert!(s2.is_empty());
    });

    #[test]
    #[cfg(feature = "alloc")]
    fn test_heap_resize_grow() {
        let mut buf = HeapBuffer::new(5);
        buf.commit(3); // [x, x, x, ., .]

        // Grow to 10
        buf.try_resize(10).expect("Resize should succeed");

        assert_eq!(buf.capacity(), 10);
        assert_eq!(buf.remaining(), 3);
        assert_eq!(buf.remaining_mut(), 7);

        // Verify data integrity
        let (s1, _) = buf.as_slices();
        assert_eq!(s1.len(), 3);

        // Verify we can write into new space
        buf.commit(7);
        assert!(buf.is_full());
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_heap_resize_shrink_success() {
        let mut buf = HeapBuffer::new(10);
        buf.commit(3); // Size 3, remaining 7

        // Shrink to 5.
        // shrink_by (5) <= remaining_mut (7), so this fits.
        buf.try_resize(5).expect("Resize should succeed");

        assert_eq!(buf.capacity(), 5);
        assert_eq!(buf.remaining(), 3);
        assert_eq!(buf.remaining_mut(), 2);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_heap_resize_shrink_fail() {
        let mut buf = HeapBuffer::new(10);
        buf.commit(8); // Size 8, remaining_mut 2

        // Try to shrink to 5.
        // We have 8 bytes of data, target capacity is 5.
        // We need to dispose of 3 bytes to fit.
        let err = buf.try_resize(5).unwrap_err();

        assert_eq!(err, 3, "Should report 3 bytes need to be consumed");

        // Verify buffer state is unchanged
        assert_eq!(buf.capacity(), 10);
        assert_eq!(buf.remaining(), 8);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_heap_resize_shrink_wraparound() {
        let mut buf = HeapBuffer::new(5);

        // Create wrap condition:
        // 1. Fill 5
        buf.commit(5);
        // 2. Consume 2 (Start = 2, Size = 3) -> [., ., x, x, x]
        buf.consume(2);
        // 3. Write 1 (Wraps to index 0) -> [x, ., x, x, x] (Size = 4)
        buf.commit(1);

        // Current state: Capacity 5, Size 4, Wrapped.
        // Shrink to 4.
        // shrink_by (1) == remaining_mut (1). This fits exactly.
        buf.try_resize(4).expect("Resize should succeed");

        assert_eq!(buf.capacity(), 4);
        assert!(buf.is_full());

        // Verify make_contiguous happened implicitly
        let (s1, s2) = buf.as_slices();
        assert_eq!(s1.len(), 4);
        assert!(s2.is_empty());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    #[should_panic]
    #[cfg(feature = "alloc")]
    fn test_heap_resize_zero_panics() {
        let mut buf = HeapBuffer::new(10);
        let _ = buf.try_resize(0);
    }
}

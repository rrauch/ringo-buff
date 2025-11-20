# ringo-buff

[![crates.io](https://img.shields.io/crates/v/ringo-buff.svg)](https://crates.io/crates/ringo-buff)
[![docs.rs](https://docs.rs/ringo-buff/badge.svg)](https://docs.rs/ringo-buff)
![license](https://img.shields.io/crates/l/ringo-buff.svg)

Ring buffers for bytes, with heap and stack storage.

`ringo-buff` provides a simple API for managing bytes in a circular buffer, supporting both **heap** and **stack**
storage strategies.
It can be used without allocating and should be usable in a `no_std` environment.
Useful for I/O buffering, streaming data, etc.

## Features

* **Dual Storage Backends**: Choose between `HeapBuffer` (dynamic `Vec`) or `StackBuffer` (fixed array).
* **Predictable API**: Explicit control over `consume` (read) and `commit` (write) cursors.
* **Zero-Copy Access**: Access internal data directly via mutable/immutable slices.
* **Ecosystem Integration**: Optional support for `bytes` traits and `zeroize`.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
ringo-buff = "0.1.0"
```

### Heap Buffer (Allocated)

Best for dynamic sizes or when large buffers are required.

```rust
use ringo_buff::HeapBuffer;

fn main() {
    // Allocate a buffer with 1KB capacity
    let mut buf = HeapBuffer::new(1024);

    // WRITING: Get writable slices
    let (first, _) = buf.as_mut_slices();
    // ... write data into 'first' ...
    // Advance the write cursor
    buf.commit(50);

    // READING: Get readable slices
    let (head, tail) = buf.as_slices();
    // ... process data ...
    // Advance the read cursor
    buf.consume(50);
}
```

### Stack Buffer (Fixed Size)

Best for small, fixed buffers where heap allocation is undesirable.

```rust
use ringo_buff::StackBuffer;

fn main() {
    // Create a buffer on the stack with 128 bytes capacity
    let mut buf: StackBuffer<128> = StackBuffer::new();

    // API is identical to HeapBuffer
    assert_eq!(buf.capacity(), 128);
}
```

## Feature Flags

| Flag               | Description                                                                                                                                                       | Default |
|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------|
| **`alloc`**        | Enables `HeapBuffer` and `Vec` support.                                                                                                                           | **Yes** |
| **`buf-trait`**    | Implements [`bytes::Buf`](https://docs.rs/bytes/latest/bytes/buf/trait.Buf.html) and [`bytes::BufMut`](https://docs.rs/bytes/latest/bytes/buf/trait.BufMut.html). | No      |
| **`zeroize`**      | Securely wipes memory on drop via the [`zeroize`](https://crates.io/crates/zeroize) crate.                                                                        | No      |
| **`hybrid-array`** | Enables `hybrid-array` support for `StackBuffer` (e.g., `StackBuffer<U1024>`).                                                                                    | No      |

To use `ringo-buff` in an environment without an allocator, disable default features:

```toml
[dependencies]
ringo-buff = { version = "0.1.0", default-features = false }
```

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
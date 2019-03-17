buddy_system_allocator
=================================

[![Crates.io version][crate-img]][crate]
[![docs.rs][docs-img]][docs]


An (almost) drop-in replacement for [phil-opp/linked-list-allocator](https://github.com/phil-opp/linked-list-allocator). But it uses buddy system instead.


## Usage

To use buddy_system_allocator for global allocator:

```rust
use buddy_system_allocator::LockedHeap;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
```

To init the allocator:

```rust
unsafe {
    HEAP_ALLOCATOR.lock().init(heap_start, heap_end);
}
```

## License

Some code comes from phil-opp's linked-list-allocator.

Licensed under MIT License. Thanks phill-opp's linked-list-allocator for inspirations and interface.

[crate-img]:     https://img.shields.io/crates/v/buddy_system_allocator.svg
[crate]:         https://crates.io/crates/buddy_system_allocator
[docs-img]:      https://docs.rs/buddy_system_allocator/badge.svg
[docs]:          https://docs.rs/buddy_system_allocator


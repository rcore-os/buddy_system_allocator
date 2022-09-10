#[macro_use]
extern crate alloc;

use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use alloc::alloc::GlobalAlloc;
use alloc::alloc::Layout;
use buddy_system_allocator::LockedHeap;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[inline]
pub fn large_alloc<const ORDER: usize>(heap: &LockedHeap<ORDER>) {
    let layout = unsafe { Layout::from_size_align_unchecked(1024, 8) };
    unsafe {
        let addr = heap.alloc(layout);
        heap.dealloc(addr, layout);
    }
}

#[inline]
pub fn small_alloc<const ORDER: usize>(heap: &LockedHeap<ORDER>) {
    let layout = unsafe { Layout::from_size_align_unchecked(8, 8) };
    unsafe {
        let addr = heap.alloc(layout);
        heap.dealloc(addr, layout);
    }
}

#[inline]
pub fn mutil_thread_alloc<const ORDER: usize>(heap: &'static LockedHeap<ORDER>) {
    let mut threads = vec![];
    let alloc = Arc::new(heap);
    for i in 0..10 {
        let a = alloc.clone();
        let handle = thread::spawn(move || {
            let layout = unsafe { Layout::from_size_align_unchecked(i * 10, 8) };
            let addr;
            unsafe { addr = a.alloc(layout) }
            sleep(Duration::from_nanos(10 - i as u64));
            unsafe { a.dealloc(addr, layout) }
        });
        threads.push(handle);
    }
    drop(alloc);

    for t in threads {
        t.join().unwrap();
    }
}

const ORDER: usize = 32;
static HEAP_ALLOCATOR: LockedHeap<ORDER> = LockedHeap::<ORDER>::new();
const KERNEL_HEAP_SIZE: usize = 16 * 1024 * 1024;
const MACHINE_ALIGN: usize = core::mem::size_of::<usize>();
const HEAP_BLOCK: usize = KERNEL_HEAP_SIZE / MACHINE_ALIGN;
static mut HEAP: [usize; HEAP_BLOCK] = [0; HEAP_BLOCK];

pub fn criterion_benchmark(c: &mut Criterion) {
    // init heap
    let heap_start = unsafe { HEAP.as_ptr() as usize };
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(heap_start, HEAP_BLOCK * MACHINE_ALIGN);
    }

    // run benchmark
    c.bench_function("small alloc", |b| {
        b.iter(|| small_alloc(black_box(&HEAP_ALLOCATOR)))
    });
    c.bench_function("large alloc", |b| {
        b.iter(|| large_alloc(black_box(&HEAP_ALLOCATOR)))
    });
    c.bench_function("mutil thread alloc", |b| {
        b.iter(|| mutil_thread_alloc(black_box(&HEAP_ALLOCATOR)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

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

const LARGE_SIZE: usize = 1024;
const SMALL_SIZE: usize = 8;
const THREAD_SIZE: usize = 10;
const ALIGN: usize = 8;

#[inline]
pub fn large_alloc<const ORDER: usize>(heap: &LockedHeap<ORDER>) {
    let layout = unsafe { Layout::from_size_align_unchecked(LARGE_SIZE, ALIGN) };
    unsafe {
        let addr = heap.alloc(layout);
        heap.dealloc(addr, layout);
    }
}

#[inline]
pub fn small_alloc<const ORDER: usize>(heap: &LockedHeap<ORDER>) {
    let layout = unsafe { Layout::from_size_align_unchecked(SMALL_SIZE, ALIGN) };
    unsafe {
        let addr = heap.alloc(layout);
        heap.dealloc(addr, layout);
    }
}

#[inline]
pub fn mutil_thread_alloc<const ORDER: usize>(heap: &'static LockedHeap<ORDER>) {
    let mut threads = Vec::with_capacity(THREAD_SIZE);
    let alloc = Arc::new(heap);
    for i in 0..THREAD_SIZE {
        let prethread_alloc = alloc.clone();
        let handle = thread::spawn(move || {
            let layout = unsafe { Layout::from_size_align_unchecked(i * THREAD_SIZE, ALIGN) };
            let addr;
            unsafe { addr = prethread_alloc.alloc(layout) }
            sleep(Duration::from_nanos((THREAD_SIZE - i) as u64));
            unsafe { prethread_alloc.dealloc(addr, layout) }
        });
        threads.push(handle);
    }
    drop(alloc);

    for t in threads {
        t.join().unwrap();
    }
}

/// # From **Hoard** benchmark: threadtest.cpp, rewrite in Rust
///
/// # Warnning
///
/// This benchmark generally needs long time to finish
///
/// ----------------------------------------------------------------------
/// Hoard: A Fast, Scalable, and Memory-Efficient Allocator
///       for Shared-Memory Multiprocessors
/// Contact author: Emery Berger, http://www.cs.utexas.edu/users/emery
//
/// Copyright (c) 1998-2000, The University of Texas at Austin.
///
/// This library is free software; you can redistribute it and/or modify
/// it under the terms of the GNU Library General Public License as
/// published by the Free Software Foundation, http://www.fsf.org.
///
/// This library is distributed in the hope that it will be useful, but
/// WITHOUT ANY WARRANTY; without even the implied warranty of
/// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
/// Library General Public License for more details.
/// ----------------------------------------------------------------------
///
#[inline]
pub fn thread_test<const ORDER: usize>(heap: &'static LockedHeap<ORDER>) {
    const N_ITERATIONS: usize = 50;
    const N_OBJECTS: usize = 30000;
    const N_THREADS: usize = 10;
    const OBJECT_SIZE: usize = 1;

    let mut threads = Vec::with_capacity(THREAD_SIZE);
    let alloc = Arc::new(heap);

    for i in 0..THREAD_SIZE {
        let prethread_alloc = alloc.clone();
        let handle = thread::spawn(move || {
            // a = new Foo * [nobjects / nthreads];
            let layout = unsafe {
                Layout::from_size_align_unchecked(SMALL_SIZE * (N_OBJECTS / N_THREADS), ALIGN)
            };
            let addr = unsafe { prethread_alloc.alloc(layout) };
            for j in 0..N_ITERATIONS {
                // inner object:
                // a[i] = new Foo[objSize];
                let mut addrs = vec![];
                let layout =
                    unsafe { Layout::from_size_align_unchecked(SMALL_SIZE * OBJECT_SIZE, ALIGN) };
                for i in 0..(N_OBJECTS / N_THREADS) {
                    addrs.push(unsafe { prethread_alloc.alloc(layout) });
                }
                for addr in addrs {
                    unsafe { prethread_alloc.dealloc(addr, layout) }
                }
            }
            unsafe { prethread_alloc.dealloc(addr, layout) }
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
    c.bench_function("threadtest", |b| {
        b.iter(|| thread_test(black_box(&HEAP_ALLOCATOR)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

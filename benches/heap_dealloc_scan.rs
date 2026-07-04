//! Benchmark for Heap::dealloc free-list scan cost at different scales.
//!
//! Constructs a fragmented heap where `free_list[class_6]` accumulates a long
//! chain of freed higher-half blocks, then measures the cost of deallocating
//! their lower-half buddies.  Each dealloc must linearly scan the free list to
//! find its buddy — exposing the O(N) scan path in the intrusive linked list.
//!
//! Multiple sizes are benchmarked (100, 500, 1000, 2000, 5000, 10000 pairs)
//! so the quadratic degradation is visible in the Criterion report.
//!
//! # Strategy
//!
//! 1. Allocate all possible 64-byte blocks to drain class-6.
//! 2. Identify real buddy pairs via `addr ^ BLOCK_SIZE`.
//! 3. Filter every-other 128-byte parent so class-7 doesn't cascade-merge.
//! 4. Free all higher-half buddies (builds a long `free_list[6]`).
//! 5. Measure: free all lower-half buddies in address order.
//!    Each call pushes itself to the head, then scans toward the tail
//!    where its buddy sits — the scan depth shrinks as pairs are consumed,
//!    averaging ~N/2 per dealloc, giving total ≈ O(N²/2) pointer chases.

use core::alloc::Layout;
use core::ptr::NonNull;
use std::collections::BTreeSet;

use buddy_system_allocator::Heap;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Max order.  `free_list` has `ORDER` slots.  Class `ORDER-1` = 22 → 4 MiB.
const ORDER: usize = 23;

/// Total backing memory (4 MiB).
const HEAP_BYTES: usize = 4 * 1024 * 1024;

/// Number of `usize` words.
const HEAP_WORDS: usize = HEAP_BYTES / core::mem::size_of::<usize>();

/// Target size class: 64 bytes → class 6 (2^6).
const BLOCK_SIZE: usize = 64;

/// Degradation curve — number of buddy *pairs* to measure at each point.
const PAIR_SIZES: &[usize] = &[100, 500, 1000, 2000, 5000];

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

struct BenchState {
    heap: Heap<ORDER>,
    #[allow(dead_code)]
    backing: Box<[usize]>,          // must outlive `heap`
    layout: Layout,
    lower_ptrs: Vec<NonNull<u8>>,   // sorted (ascending address)
}

// ---------------------------------------------------------------------------
// Setup (parameterised by target pair count)
// ---------------------------------------------------------------------------

/// Build a heap where `free_list[6]` holds `target_pairs` higher-half blocks,
/// ready for their lower-half buddies to be deallocated.
fn setup_fragmented_heap(target_pairs: usize) -> BenchState {
    // 1. backing memory -------------------------------------------------------

    let mut backing = vec![0usize; HEAP_WORDS].into_boxed_slice();
    let start = backing.as_mut_ptr() as usize;
    let size = backing.len() * core::mem::size_of::<usize>();

    let mut heap = Heap::<ORDER>::empty();
    unsafe { heap.init(start, size); }

    let layout =
        Layout::from_size_align(BLOCK_SIZE, core::mem::size_of::<usize>()).unwrap();

    // 2. allocate all 64-byte blocks (drain class-6 and above) ----------------

    let mut ptrs: Vec<NonNull<u8>> = Vec::new();
    loop {
        match heap.alloc(layout) {
            Ok(ptr) => ptrs.push(ptr),
            Err(()) => break,
        }
    }

    // 3. identify real buddy pairs --------------------------------------------

    let mut addrs: Vec<usize> = ptrs.iter().map(|p| p.as_ptr() as usize).collect();
    addrs.sort_unstable();

    let addr_set: BTreeSet<usize> = addrs.iter().copied().collect();

    let mut pairs: Vec<(usize, usize)> = Vec::new(); // (lower, higher)
    for &addr in &addrs {
        if addr & BLOCK_SIZE != 0 {
            continue; // skip higher halves during discovery
        }
        let buddy = addr ^ BLOCK_SIZE;
        if addr_set.contains(&buddy) {
            pairs.push((addr, buddy));
        }
    }

    // 4. filter: skip adjacent 128-byte parents to isolate class-6 scan -------

    pairs.retain(|(lower, _)| lower & (2 * BLOCK_SIZE) == 0);

    assert!(
        pairs.len() >= target_pairs,
        "not enough buddy pairs: need {target_pairs}, have {} — increase HEAP_BYTES",
        pairs.len()
    );
    pairs.truncate(target_pairs);

    // 5. sort by lower address → buddy = oldest in list when freed in order ---

    pairs.sort_unstable_by_key(|(lower, _)| *lower);

    let higher_ptrs: Vec<NonNull<u8>> = pairs
        .iter()
        .map(|(_, higher)| unsafe { NonNull::new_unchecked(*higher as *mut u8) })
        .collect();

    let lower_ptrs: Vec<NonNull<u8>> = pairs
        .iter()
        .map(|(lower, _)| unsafe { NonNull::new_unchecked(*lower as *mut u8) })
        .collect();

    // 6. build long free list — free higher halves (no merge: buddies still allocated)

    for ptr in &higher_ptrs {
        unsafe { heap.dealloc(*ptr, layout); }
    }

    BenchState { heap, backing, layout, lower_ptrs }
}

// ---------------------------------------------------------------------------
// Benchmarks (one per scale)
// ---------------------------------------------------------------------------

pub fn bench_heap_dealloc_freelist_scan(c: &mut Criterion) {
    for &n in PAIR_SIZES {
        let name = format!("heap_dealloc_freelist_scan/{n}_pairs");
        c.bench_function(&name, |b| {
            b.iter_batched(
                || setup_fragmented_heap(n),
                |mut state: BenchState| {
                    for ptr in &state.lower_ptrs {
                        unsafe { state.heap.dealloc(*ptr, state.layout); }
                    }
                    std::hint::black_box(state.heap.stats_alloc_actual());
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group!(benches, bench_heap_dealloc_freelist_scan);
criterion_main!(benches);

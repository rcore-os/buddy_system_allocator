use crate::FrameAllocator;
use crate::Heap;
use crate::LockedHeapWithRescue;
use crate::linked_list;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::mem::size_of;

#[test]
fn test_linked_list() {
    let mut value1: usize = 0;
    let mut value2: usize = 0;
    let mut value3: usize = 0;
    let mut list = linked_list::LinkedList::new();
    unsafe {
        list.push(&mut value1 as *mut usize);
        list.push(&mut value2 as *mut usize);
        list.push(&mut value3 as *mut usize);
    }

    // Test links
    assert_eq!(value3, &value2 as *const usize as usize);
    assert_eq!(value2, &value1 as *const usize as usize);
    assert_eq!(value1, 0);

    // Test iter
    let mut iter = list.iter();
    assert_eq!(iter.next(), Some(&mut value3 as *mut usize));
    assert_eq!(iter.next(), Some(&mut value2 as *mut usize));
    assert_eq!(iter.next(), Some(&mut value1 as *mut usize));
    assert_eq!(iter.next(), None);

    // Test iter_mut

    let mut iter_mut = list.iter_mut();
    assert_eq!(iter_mut.next().unwrap().pop(), &mut value3 as *mut usize);

    // Test pop
    assert_eq!(list.pop(), Some(&mut value2 as *mut usize));
    assert_eq!(list.pop(), Some(&mut value1 as *mut usize));
    assert_eq!(list.pop(), None);
}

#[test]
fn test_empty_heap() {
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());
}

#[test]
fn test_heap_add() {
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

    let space: [usize; 100] = [0; 100];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }
    let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

#[test]
fn test_heap_add_large() {
    // Max size of block is 2^7 == 128 bytes
    let mut heap = Heap::<8>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

    // 512 bytes of space
    let space: [u8; 512] = [0; 512];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(512) as usize);
    }
    let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap());
    assert!(addr.is_ok());
}

#[test]
fn test_heap_oom() {
    let mut heap = Heap::<32>::new();
    let space: [usize; 100] = [0; 100];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }

    assert!(
        heap.alloc(Layout::from_size_align(100 * size_of::<usize>(), 1).unwrap())
            .is_err()
    );
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_ok());
}

#[test]
fn test_heap_oom_rescue() {
    const SPACE_SIZE: usize = 100;
    static mut SPACE: [usize; 100] = [0; SPACE_SIZE];
    let heap = LockedHeapWithRescue::new(|heap: &mut Heap<32>, _layout: &Layout| unsafe {
        heap.init(&raw mut SPACE as usize, SPACE_SIZE);
    });

    unsafe {
        assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()) as usize != 0);
    }
}

#[test]
fn test_heap_alloc_and_free() {
    let mut heap = Heap::<32>::new();
    assert!(heap.alloc(Layout::from_size_align(1, 1).unwrap()).is_err());

    let space: [usize; 100] = [0; 100];
    unsafe {
        heap.add_to_heap(space.as_ptr() as usize, space.as_ptr().add(100) as usize);
    }
    for _ in 0..100 {
        let addr = heap.alloc(Layout::from_size_align(1, 1).unwrap()).unwrap();
        unsafe {
            heap.dealloc(addr, Layout::from_size_align(1, 1).unwrap());
        }
    }
}

#[test]
fn test_empty_frame_allocator() {
    let mut frame = FrameAllocator::<32>::new();
    assert!(frame.alloc(1).is_none());
}

#[test]
fn test_frame_allocator_add() {
    let mut frame = FrameAllocator::<32>::new();
    assert!(frame.alloc(1).is_none());

    frame.insert(0..3);
    let num = frame.alloc(1);
    assert_eq!(num, Some(2));
    let num = frame.alloc(2);
    assert_eq!(num, Some(0));
    assert!(frame.alloc(1).is_none());
    assert!(frame.alloc(2).is_none());
}

#[test]
fn test_frame_allocator_add_from_zero_keeps_large_block() {
    let mut frame = FrameAllocator::<7>::new();

    frame.add_frame(0, 64);

    assert_eq!(frame.alloc(64), Some(0));
}

#[test]
fn test_frame_allocator_allocate_large() {
    let mut frame = FrameAllocator::<32>::new();
    assert_eq!(frame.alloc(10_000_000_000), None);
}

#[test]
fn test_frame_allocator_add_large_size_split() {
    let mut frame = FrameAllocator::<32>::new();

    frame.insert(0..10_000_000_000);

    assert_eq!(frame.alloc(0x8000_0001), None);
    assert_eq!(frame.alloc(0x8000_0000), Some(0));
    assert_eq!(frame.alloc(0x8000_0000), Some(0x8000_0000));
}

#[test]
fn test_frame_allocator_add_large_size() {
    let mut frame = FrameAllocator::<33>::new();

    frame.insert(0..10_000_000_000);
    assert_eq!(frame.alloc(0x8000_0001), Some(0));
}

#[test]
fn test_frame_allocator_alloc_and_free() {
    let mut frame = FrameAllocator::<32>::new();
    assert!(frame.alloc(1).is_none());

    frame.add_frame(0, 1024);
    for _ in 0..100 {
        let addr = frame.alloc(512).unwrap();
        frame.dealloc(addr, 512);
    }
}

#[test]
fn test_frame_allocator_alloc_and_free_complex() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(100, 1024);
    for _ in 0..10 {
        let addr = frame.alloc(1).unwrap();
        frame.dealloc(addr, 1);
    }
    let addr1 = frame.alloc(1).unwrap();
    let addr2 = frame.alloc(1).unwrap();
    assert_ne!(addr1, addr2);
}

#[test]
fn test_frame_allocator_aligned() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(1, 64);
    assert_eq!(
        frame.alloc_aligned(Layout::from_size_align(2, 4).unwrap()),
        Some(4)
    );
    assert_eq!(
        frame.alloc_aligned(Layout::from_size_align(2, 2).unwrap()),
        Some(2)
    );
    assert_eq!(
        frame.alloc_aligned(Layout::from_size_align(2, 1).unwrap()),
        Some(8)
    );
    assert_eq!(
        frame.alloc_aligned(Layout::from_size_align(1, 16).unwrap()),
        Some(16)
    );
}

#[test]
fn test_frame_allocator_merge_final_order() {
    let mut frame = FrameAllocator::<2>::new();
    frame.add_frame(0, 4);

    let first = frame.alloc(2).unwrap();
    let second = frame.alloc(2).unwrap();

    frame.dealloc(first, 2);
    frame.dealloc(second, 2);

    assert_eq!(frame.alloc(2), Some(0));
}

#[test]
fn test_heap_merge_final_order() {
    const NUM_ORDERS: usize = 5;

    let backing_size = 1 << NUM_ORDERS;
    let backing_layout = Layout::from_size_align(backing_size, backing_size).unwrap();

    // create a new heap with 5 orders
    let mut heap = Heap::<NUM_ORDERS>::new();

    // allocate host memory for use by heap
    let backing_allocation = unsafe { std::alloc::alloc(backing_layout) };

    let start = backing_allocation as usize;
    let middle = unsafe { backing_allocation.add(backing_size / 2) } as usize;
    let end = unsafe { backing_allocation.add(backing_size) } as usize;

    // add two contiguous ranges of memory
    unsafe { heap.add_to_heap(start, middle) };
    unsafe { heap.add_to_heap(middle, end) };

    // NUM_ORDERS - 1 is the maximum order of the heap
    let layout = Layout::from_size_align(1 << (NUM_ORDERS - 1), 1).unwrap();

    // allocation should succeed, using one of the added ranges
    let alloc = heap.alloc(layout).unwrap();

    // deallocation should not attempt to merge the two contiguous ranges as the next order does not exist
    unsafe {
        heap.dealloc(alloc, layout);
    }
}

#[test]
fn test_frame_allocator_alloc_at_basic() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(0, 4);
    assert_eq!(frame.alloc_at(0, 4), Some(0));
    assert!(frame.alloc(1).is_none());
}

#[test]
fn test_frame_allocator_alloc_at_split() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(0, 8);
    // Alloc 2 frames at address 2 (requires splitting the order-3 block)
    assert_eq!(frame.alloc_at(2, 2), Some(2));
    // Remaining: [0..2) at order 1, [4..8) at order 2
    assert_eq!(frame.alloc(2), Some(0));
    assert_eq!(frame.alloc(4), Some(4));
    assert!(frame.alloc(1).is_none());
}

#[test]
fn test_frame_allocator_alloc_at_unavailable() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(0, 8);
    assert_eq!(frame.alloc(4), Some(0));
    // [0..4) is allocated, try to alloc_at within it
    assert_eq!(frame.alloc_at(0, 2), None);
    assert_eq!(frame.alloc_at(2, 2), None);
}

#[test]
fn test_frame_allocator_alloc_at_misaligned() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(0, 16);
    // 4 frames at address 3: not aligned to 4
    assert_eq!(frame.alloc_at(3, 4), None);
    // 2 frames at address 1: not aligned to 2
    assert_eq!(frame.alloc_at(1, 2), None);
    // 1 frame at address 1: aligned to 1, should work
    assert_eq!(frame.alloc_at(1, 1), Some(1));
}

#[test]
fn test_frame_allocator_alloc_at_then_dealloc() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(0, 16);
    assert_eq!(frame.alloc_at(4, 4), Some(4));
    frame.dealloc(4, 4);
    // Buddies should merge back; full 16-frame alloc should succeed
    assert_eq!(frame.alloc(16), Some(0));
}

#[test]
fn test_frame_allocator_alloc_at_outside_range() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(0, 8);
    assert_eq!(frame.alloc_at(16, 2), None);
}

#[test]
fn test_frame_allocator_alloc_at_multiple() {
    let mut frame = FrameAllocator::<32>::new();
    frame.add_frame(0, 16);
    assert_eq!(frame.alloc_at(0, 4), Some(0));
    assert_eq!(frame.alloc_at(4, 4), Some(4));
    assert_eq!(frame.alloc_at(8, 4), Some(8));
    assert_eq!(frame.alloc_at(12, 4), Some(12));
    assert!(frame.alloc(1).is_none());
}

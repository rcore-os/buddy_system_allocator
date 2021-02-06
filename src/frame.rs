use super::prev_power_of_two;
use alloc::collections::BTreeSet;
use core::cmp::min;
use core::ops::Range;

#[cfg(feature = "use_spin")]
use core::ops::Deref;
#[cfg(feature = "use_spin")]
use spin::Mutex;

/// A frame allocator that uses buddy system,
/// requiring a global allocator
///
/// # Usage
///
/// Create a frame allocator and add some frames to it:
/// ```
/// use buddy_system_allocator::*;
/// let mut frame = FrameAllocator::new();
/// assert!(frame.alloc(1).is_none());
///
/// frame.add_frame(0, 3);
/// let num = frame.alloc(1);
/// assert_eq!(num, Some(2));
/// let num = frame.alloc(2);
/// assert_eq!(num, Some(0));
/// ```
pub struct FrameAllocator {
    // buddy system with max order of 32
    free_list: [BTreeSet<usize>; 32],

    // statistics
    allocated: usize,
    total: usize,
}

impl FrameAllocator {
    /// Create an empty frame allocator
    pub fn new() -> Self {
        FrameAllocator {
            free_list: Default::default(),
            allocated: 0,
            total: 0,
        }
    }

    /// Add a range of frame number [start, end) to the allocator
    pub fn add_frame(&mut self, start: usize, end: usize) {
        assert!(start <= end);

        let mut total = 0;
        let mut current_start = start;

        while current_start < end {
            let lowbit = if current_start > 0 {
                current_start & (!current_start + 1)
            } else {
                32
            };
            let size = min(lowbit, prev_power_of_two(end - current_start));
            total += size;

            self.free_list[size.trailing_zeros() as usize].insert(current_start);
            current_start += size;
        }

        self.total += total;
    }

    /// Add a range of frame to the allocator
    pub fn insert(&mut self, range: Range<usize>) {
        self.add_frame(range.start, range.end);
    }

    /// Alloc a range of frames from the allocator, return the first frame of the allocated range
    pub fn alloc(&mut self, count: usize) -> Option<usize> {
        let size = count.next_power_of_two();
        let class = size.trailing_zeros() as usize;
        for i in class..self.free_list.len() {
            // Find the first non-empty size class
            if !self.free_list[i].is_empty() {
                // Split buffers
                for j in (class + 1..i + 1).rev() {
                    if let Some(block_ref) = self.free_list[j].iter().next() {
                        let block = *block_ref;
                        self.free_list[j - 1].insert(block + (1 << (j - 1)));
                        self.free_list[j - 1].insert(block);
                        self.free_list[j].remove(&block);
                    } else {
                        return None;
                    }
                }

                let result = self.free_list[class].iter().next().clone();
                if let Some(result_ref) = result {
                    let result = *result_ref;
                    self.free_list[class].remove(&result);
                    self.allocated += size;
                    return Some(result);
                } else {
                    return None;
                }
            }
        }
        None
    }

    /// Dealloc a range of frames [frame, frame+count) from the frame allocator.
    /// The range should be exactly the same when it was allocated, as in heap allocator
    pub fn dealloc(&mut self, frame: usize, count: usize) {
        let size = count.next_power_of_two();
        let class = size.trailing_zeros() as usize;

        // Merge free buddy lists
        let mut current_ptr = frame;
        let mut current_class = class;
        while current_class < self.free_list.len() {
            let buddy = current_ptr ^ (1 << current_class);
            if self.free_list[current_class].remove(&buddy) == true {
                // Free buddy found
                current_ptr = min(current_ptr, buddy);
                current_class += 1;
            } else {
                self.free_list[current_class].insert(current_ptr);
                break;
            }
        }

        self.allocated -= size;
    }
}

/// A locked version of `FrameAllocator`
///
/// # Usage
///
/// Create a locked frame allocator and add frames to it:
/// ```
/// use buddy_system_allocator::*;
/// let mut frame = LockedFrameAllocator::new();
/// assert!(frame.lock().alloc(1).is_none());
///
/// frame.lock().add_frame(0, 3);
/// let num = frame.lock().alloc(1);
/// assert_eq!(num, Some(2));
/// let num = frame.lock().alloc(2);
/// assert_eq!(num, Some(0));
/// ```
#[cfg(feature = "use_spin")]
pub struct LockedFrameAllocator(Mutex<FrameAllocator>);

#[cfg(feature = "use_spin")]
impl LockedFrameAllocator {
    /// Creates an empty heap
    pub fn new() -> LockedFrameAllocator {
        LockedFrameAllocator(Mutex::new(FrameAllocator::new()))
    }
}

#[cfg(feature = "use_spin")]
impl Deref for LockedFrameAllocator {
    type Target = Mutex<FrameAllocator>;

    fn deref(&self) -> &Mutex<FrameAllocator> {
        &self.0
    }
}

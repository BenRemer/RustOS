use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::mem::size_of;
use core::cmp::max;
use core::cmp::min;

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;

/// A simple allocator that allocates based on size classes.
///   bin 0 (2^3 bytes)    : handles allocations in (0, 2^3]
///   bin 1 (2^4 bytes)    : handles allocations in (2^3, 2^4]
///   ...
///   bin 29 (2^22 bytes): handles allocations in (2^31, 2^32]
///   
///   map_to_bin(size) -> k
///   

pub struct Allocator {
    // Add the necessary fields.
    list: [LinkedList; 32],
    allocated: usize,
    total: usize,
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        let mut list = [LinkedList::new(); 32];
        let mut current = start;
        let mut total = 0;
        while current + size_of::<usize>() <= end {
            let small = current & (!current + 1);
            let size = min(small, 1 << (8*(size_of::<usize>())-(end-current).leading_zeros() as usize -1));
            total += size;
            unsafe {
                list[size.trailing_zeros() as usize].push(current as *mut usize);
            }
            current += size;
        }
        Allocator {list: list, allocated: 0, total}
    }
}

impl LocalAlloc for Allocator {
    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning null pointer (`core::ptr::null_mut`)
    /// indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's
    /// size or alignment constraints.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = max(layout.size().next_power_of_two(), max(layout.align(), size_of::<usize>()));
        let class = size.trailing_zeros() as usize;
        for i in class..self.list.len() {
            if !self.list[i].is_empty() {
                for j in (class + 1..i + 1).rev() {
                    let block = self.list[j].pop().expect("bigger block should have free space");
                    unsafe {
                        self.list[j-1].push((block as usize + (1 << (j - 1))) as *mut usize);
                        self.list[j-1].push(block);
                    }
                }
                let result = self.list[class].pop().expect("current block should have free space now") as *mut u8;
                self.allocated += size;
                return result;
            }
        }
        ptr::null_mut()
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = max(
            layout.size().next_power_of_two(),
            max(layout.align(), size_of::<usize>()),
        );
        let bin = size.trailing_zeros() as usize;
        unsafe {
            self.list[bin].push(ptr as *mut usize);
            let mut curr_ptr = ptr as usize;
            let mut curr_bin = bin;
            loop {
                let addr = curr_ptr ^ (1 << curr_bin);
                let mut exists = false;
                for node in self.list[curr_bin].iter_mut() {
                    if (node.value() as usize) == addr {
                        exists = true;
                        node.pop();
                        break;
                    }
                }
                if exists {
                    self.list[curr_bin].pop();
                    curr_ptr = min(curr_ptr, addr);
                    curr_bin += 1;
                    self.list[curr_bin].push(curr_ptr as *mut usize);
                } else {
                    break;
                }
            }
        }
        self.allocated -= size;
    }
}

// Implement `Debug` for `Allocator`.
impl fmt::Debug for Allocator {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("BinAllocator")
            .field("allocated", &self.allocated)
            .field("total", &self.total)
            .field("list", &self.list)
            .finish()
    }
}

use std::alloc::{GlobalAlloc, Layout};
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

// ARENA_SIZE corresponds to HEAP size. n * 1024 where n is the quantity of bytes
pub const ARENA_SIZE: usize = 128 * 1024;

// MAX_SUPPORTED_ALIGN is the largest alignment allocator guarantees to support.
// Any allocation requiring alignment up to this value will be placed at an address
// divisible by it. It does not define a max allocation size.
pub const MAX_SUPPORTED_ALIGN: usize = 4096;

#[repr(C, align(4096))] // MAX_SUPPORTED_ALIGN
pub struct ReallyCoolAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    remaining: AtomicUsize,
}

#[global_allocator] // when basic required methods will be implemented
pub static ALLOCATOR: ReallyCoolAllocator = ReallyCoolAllocator {
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    remaining: AtomicUsize::new(ARENA_SIZE),
};

unsafe impl Sync for ReallyCoolAllocator {}

unsafe impl GlobalAlloc for ReallyCoolAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        let align_mask_to_round_down = !(align - 1);

        if align > MAX_SUPPORTED_ALIGN {
            return null_mut();
        }

        let mut allocated = 0;
        if self
            .remaining
            .fetch_update(Relaxed, Relaxed, |mut remaining| {
                if size > remaining {
                    return None;
                }
                remaining -= size;
                remaining &= align_mask_to_round_down;
                allocated = remaining;
                Some(remaining)
            })
            .is_err()
        {
            return null_mut();
        };
        unsafe { self.arena.get().cast::<u8>().add(allocated) }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

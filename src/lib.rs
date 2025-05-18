use std::alloc::{GlobalAlloc, Layout};
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

// ARENA_SIZE corresponds to HEAP size. n * 1024 where n is the quantity of bytes
const ARENA_SIZE: usize = 128 * 1024;

// MAX_SUPPORTED_ALIGN is the largest alignment allocator guarantees to support.
// Any allocation requiring alignment up to this value will be placed at an address
// divisible by it. It does not define a max allocation size.
const MAX_SUPPORTED_ALIGN: usize = 4096;

#[repr(C, align(4096))] // MAX_SUPPORTED_ALIGN
struct ReallyCoolAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    remaining: AtomicUsize,
}

// #[global_allocator]     when basic required methods will be implemented
static ALLOCATOR: ReallyCoolAllocator = ReallyCoolAllocator {
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    remaining: AtomicUsize::new(ARENA_SIZE),
};

unsafe impl Sync for ReallyCoolAllocator {}

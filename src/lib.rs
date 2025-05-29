// Free list allocator logic:
// at the very start First and only FreeMemList is the size
// of whole ARENA, whenever allocation needed its decreasing itself
// to size of ARENA - size of new allocation. When there are some allocations,
// and some are getting dropped(deallocated) their pointer and size are becoming
// a new nodes of FreeMemList, and if two free nodes are contiguos in memory
// they are megre(coalesce) and become a one bigger FreeMemList node
use std::alloc::{GlobalAlloc, Layout};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

// ARENA_SIZE corresponds to HEAP size. n * 1024 where n is the quantity of bytes
pub const ARENA_SIZE: usize = 10240 * 1024;

// MAX_SUPPORTED_ALIGN is the largest alignment allocator guarantees to support.
// Any allocation requiring alignment up to this value will be placed at an address
// divisible by it. It does not define a max allocation size.
pub const MAX_SUPPORTED_ALIGN: usize = 4096;

// ptr - pointer to the start of free mem block
// size - size of free mem block
// next - next FreeMemList node
#[allow(dead_code)]
pub struct FreeMemList {
    ptr: *mut u8,
    size: usize,
    next: Option<NonNull<FreeMemList>>,
}
#[repr(C, align(4096))] // MAX_SUPPORTED_ALIGN 
pub struct ReallyCoolAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    mem_list: FreeMemList,
}
#[allow(dead_code)]
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
#[global_allocator]
pub static ALLOCATOR: ReallyCoolAllocator = ReallyCoolAllocator {
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    mem_list: FreeMemList {
        ptr: &ALLOCATOR.arena as *const UnsafeCell<[u8; ARENA_SIZE]> as *mut u8,
        size: ARENA_SIZE,
        next: None,
    },
};

unsafe impl Sync for ReallyCoolAllocator {}

unsafe impl GlobalAlloc for ReallyCoolAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let size = layout.size();
            let align = layout.align();

            let mut head_initialized = false;
            if (*self.meta_offset.get()).load(Relaxed) == 0 {
                match self.alloc_metadata_node() {
                    Some(head_node) => {
                        let ptr_to_head_uninit = self.head.get();
                        (*ptr_to_head_uninit).write(MemoryList {
                            ptr: NonNull::dangling(),
                            layout: Layout::from_size_align_unchecked(0, 1),
                            free: true,
                            next: None,
                        });
                        head_initialized = true;
                    }
                    None => {
                        if cfg!(feature = "debug_alloc") {
                            eprintln!("rcmalloc: Failed to allocate initial head metadata node.");
                        }
                        return null_mut();
                    }
                }
            }

            if align > MAX_SUPPORTED_ALIGN {
                if cfg!(feature = "debug_alloc") {
                    eprintln!(
                        "rcmalloc: Requested alignment {} exceeds MAX_SUPPORTED_ALIGN",
                        align
                    );
                }
                return null_mut();
            }

            let current_remaining = self.remaining.load(Relaxed);
            let current_offset = ARENA_SIZE - current_remaining;
            let aligned_offset = (current_offset + (align - 1)) & !(align - 1);
            let required_space = aligned_offset + size;

            if cfg!(feature = "debug_alloc") {
                eprintln!(
                    "rcmalloc: alloc(size={}, align={}) - current_remaining={}, current_offset={}, aligned_offset={}, required_space={}",
                    size, align, current_remaining, current_offset, aligned_offset, required_space
                );
            }

            if required_space > ARENA_SIZE {
                if cfg!(feature = "debug_alloc") {
                    eprintln!("rcmalloc: Out of memory - required_space > ARENA_SIZE");
                }
                return null_mut();
            }

            let allocation_size = size;
            let new_remaining = self
                .remaining
                .fetch_sub(allocation_size + (aligned_offset - current_offset), Relaxed);

            if cfg!(feature = "debug_alloc") {
                eprintln!(
                    "rcmalloc: new_remaining after fetch_sub = {}",
                    new_remaining
                );
            }

            if new_remaining < allocation_size + (aligned_offset - current_offset) {
                if cfg!(feature = "debug_alloc") {
                    eprintln!(
                        "rcmalloc: Allocation failed due to race or insufficient space after subtraction"
                    );
                }
                self.remaining
                    .fetch_add(allocation_size + (aligned_offset - current_offset), Relaxed);
                return null_mut();
            }

            let ptr = self.arena.get().cast::<u8>().add(aligned_offset);

            if cfg!(feature = "debug_alloc") {
                static mut COUNT: usize = 0;
                COUNT += 1;
                let current_count = COUNT;
                eprintln!(
                    "rcmalloc: Allocation successful at ptr={:?}, count={}",
                    ptr, current_count
                );
            }

            ptr // alloc must return **hopefully** valid pointer to where data block starts
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {}
}

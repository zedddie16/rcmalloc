// I am now starting implementing free list allocator instead of simple bump one
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

#[repr(C, align(4096))] // MAX_SUPPORTED_ALIGN
pub struct ReallyCoolAllocator<'a> {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    meta_offset: UnsafeCell<AtomicUsize>,
    head: UnsafeCell<MaybeUninit<MemoryList<'a>>>,
    remaining: AtomicUsize,
}
#[allow(dead_code)]
pub struct MemoryList<'b> {
    ptr: NonNull<u8>, // not nul as valid memory allocation obv ensure ptr to be not null
    layout: Layout,
    free: bool,
    next: Option<&'b mut MemoryList<'b>>,
}
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}
impl ReallyCoolAllocator<'static> {
    unsafe fn alloc_metadata_node<'a>(&'a self) -> Option<&'a mut MemoryList<'a>> {
        unsafe {
            let offset = self.meta_offset.get();
            let node_size = std::mem::size_of::<MemoryList>();
            let align = std::mem::align_of::<MemoryList>();

            let align_offset = ((*offset).load(Relaxed) + align - 1) & !(align - 1);
            if align_offset + node_size > ARENA_SIZE {
                return None;
            }

            let ptr = self.arena.get().cast::<u8>().add(align_offset) as *mut MemoryList<'a>;
            *self.meta_offset.get() = AtomicUsize::from(align_offset + node_size);
            Some(&mut *ptr)
        }
    }
}
#[global_allocator]
pub static ALLOCATOR: ReallyCoolAllocator = ReallyCoolAllocator {
    meta_offset: UnsafeCell::new(AtomicUsize::new(0)),
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    head: UnsafeCell::new(MaybeUninit::uninit()),
    // head: MemoryList {
    //     ptr: unsafe { NonNull::new_unchecked(ALLOCATOR.arena.get().cast::<u8>()) },
    //     layout: unsafe { Layout::from_size_align_unchecked(0, 1) },
    //     free: true,
    //     next: None,
    // },
    remaining: AtomicUsize::new(ARENA_SIZE),
};

unsafe impl Sync for ReallyCoolAllocator<'static> {}

unsafe impl GlobalAlloc for ReallyCoolAllocator<'static> {
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

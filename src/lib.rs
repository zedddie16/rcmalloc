use std::alloc::{GlobalAlloc, Layout};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

//
// Free list allocator logic:
// at the very start First and only FreeMemList is the size
// of whole ARENA, whenever allocation needed its decreasing itself
// +-------------------------+ +-------------------------+
// |                         | |        FreeMemList      |
// |        ARENA_SIZE       | |        first node       |
// |                         | |       size of arena     |
// +-------------------------+ +-------------------------+
//                     | FreeMemList |
// ╔═══════════╗ ╔═══════════╗ ╔═══════════╗ ╔═══════════╗
// ║free node 1║►║free node 2║►║free node 3║►║free node 4║
// ╚═══════════╝ ╚═══════════╝ ╚═══════════╝ ╚═══════════╝
// to size of ARENA - size of new allocation. When there are some allocations,
// and some are getting dropped(deallocated) their pointer and size are becoming
// a new nodes of FreeMemList, and if two free nodes are contiguos in memory
// they are megre(coalesce) and become a one bigger FreeMemList node
//
//
// Also, for a basic understanding of ptr system, imagine this as virtual memory:
//           .a b-a = memory offset  .b     end of ARENA .c
// +---------+-----------------------+-------------------+
// |         |~~~~~~~~~~~~~~~~~~~~~~~|```````````````````|
// | smthng  |~~~~~~~~~~~~~~~~~~~~~~~|```````````````````|
// |         |~~~~~~~~~~~~~~~~~~~~~~~|```````````````````|
// +---------+-----------------------+-------------------+
//           ^base pointer           ^base pointer + memory offset
//           to the start
//           of ARENA
// so with that said, ptrs are relative and counted by offseting from
// some starting point.
//
//                 | node management |
// ╔═══════════╗ ╔═══════════╗ ╔═══════════╗ ╔═══════════╗
// ║free node 1║►║free node 2║►║free node 3║►║free node 4║
// ╚═══════════╝ ╚═══════════╝ ╚═══════════╝ ╚═══════════╝
// if free node 2 of FreeMemList is getting filled, we'll need to
// remove it and connect first and third, or if it's bigger then requested
// size(but still match by algorithm) then it divides in two,
// and the free part connects with same nodes as its past self.

// ARENA_SIZE corresponds to HEAP size. n * 1024 where n is the quantity of bytes
pub const ARENA_SIZE: usize = 10240 * 1024;

// align_offset is the number of bytes you need to add to a pointer to make it aligned
// to a specific alignment (e.g align), which basically indicate how much padding needed
// to reach for the next valid pointer.

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
// Iter struct for FreeMemList, needed in order to
// implement iteration through the FreeMemList
#[allow(dead_code)]
struct FreeMemListIter<'a> {
    current: Option<&'a FreeMemList>,
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
            // TODO: loop which iterates through the FreeMemList Linked list
            // and finds the fit for requested allocation and splits the node
            // if its bigger then requested (with align)

            let size = layout.size();
            let align = layout.align();

            let iter = FreeMemListIter {
                current: Some(&self.mem_list),
            };

            if align > MAX_SUPPORTED_ALIGN {
                if cfg!(feature = "debug_alloc") {
                    eprintln!(
                        "rcmalloc: Requested alignment {} exceeds MAX_SUPPORTED_ALIGN",
                        align
                    );
                }
                return null_mut();
            }
            if self.mem_list.size > size {}

            let ptr = self
                .arena
                .get()
                .cast::<u8>()
                .add(self.mem_list.ptr as usize);

            // if cfg!(feature = "debug_alloc") {
            //     static mut COUNT: usize = 0;
            //     COUNT += 1;
            //     let current_count = COUNT;
            //     eprintln!(
            //         "rcmalloc: Allocation successful at ptr={:?}, count={}",
            //         ptr, current_count
            //     );
            // }

            ptr // alloc must return **hopefully** valid pointer to where data block starts
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {}
}

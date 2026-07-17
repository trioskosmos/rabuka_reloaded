use crate::sys::{self, SceSysMemBlockTypes, SceSysMemPartitionId, SceUid};
use alloc::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::{mem, ptr};

const BLOCK_SIZE: usize = 131072;
const MAX_BLOCKS: usize = 192;

struct Block {
    id: SceUid,
    start: usize,
    end: usize,
    free: UnsafeCell<usize>,
}

struct BumpAlloc {
    blocks: UnsafeCell<[Option<Block>; MAX_BLOCKS]>,
    count: AtomicUsize,
}

unsafe impl Send for BumpAlloc {}
unsafe impl Sync for BumpAlloc {}

impl BumpAlloc {
    const fn new() -> Self {
        const NONE: Option<Block> = None;
        BumpAlloc {
            blocks: UnsafeCell::new([NONE; MAX_BLOCKS]),
            count: AtomicUsize::new(0),
        }
    }

    unsafe fn add_block(&self) -> bool {
        let id = sys::sceKernelAllocPartitionMemory(
            SceSysMemPartitionId::SceKernelPrimaryUserPartition,
            &b"rblk\0"[0],
            SceSysMemBlockTypes::Low,
            BLOCK_SIZE as u32,
            ptr::null_mut(),
        );
        if id.0 < 0 {
            return false;
        }
        let start = sys::sceKernelGetBlockHeadAddr(id) as usize;
        let idx = self.count.load(Ordering::SeqCst);
        if idx >= MAX_BLOCKS {
            sys::sceKernelFreePartitionMemory(id);
            return false;
        }
        let blocks = &mut *self.blocks.get();
        blocks[idx] = Some(Block {
            id,
            start,
            end: start + BLOCK_SIZE,
            free: UnsafeCell::new(start),
        });
        self.count.fetch_add(1, Ordering::SeqCst);
        true
    }

    unsafe fn try_alloc(&self, size: usize, align: usize) -> *mut u8 {
        let count = self.count.load(Ordering::Relaxed);
        let blocks = &*self.blocks.get();
        for i in 0..count {
            if let Some(ref block) = blocks[i] {
                let free = *block.free.get();
                let aligned = free.next_multiple_of(align);
                let new_free = aligned + size;
                if new_free <= block.end {
                    *block.free.get() = new_free;
                    return aligned as *mut u8;
                }
            }
        }
        ptr::null_mut()
    }
}

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(1);
        let align = layout.align();

        loop {
            let p = self.try_alloc(size, align);
            if !p.is_null() {
                return p;
            }
            if !self.add_block() {
                return ptr::null_mut();
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOC: BumpAlloc = BumpAlloc::new();

#[cfg(not(feature = "std"))]
#[alloc_error_handler]
fn aeh(layout: Layout) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memset(ptr: *mut u8, value: u32, num: usize) -> *mut u8 {
    let mut i = 0;
    while i < num {
        *((ptr as usize + i) as *mut u8) = value as u8;
        i += 1;
    }
    ptr
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, num: isize) -> *mut u8 {
    let mut i = 0;
    while i < num {
        *((dst as isize + i) as *mut u8) = *((src as isize + i) as *mut u8);
        i += 1;
    }
    dst
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memcmp(ptr1: *mut u8, ptr2: *mut u8, num: usize) -> i32 {
    let mut i = 0;
    while i < num {
        let val1 = *((ptr1 as usize + i) as *mut u8);
        let val2 = *((ptr2 as usize + i) as *mut u8);
        let diff = val1 as i32 - val2 as i32;
        if diff != 0 {
            return diff;
        }
        i += 1;
    }
    0
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memmove(dst: *mut u8, src: *mut u8, num: isize) -> *mut u8 {
    if dst < src {
        let mut i = 0;
        while i < num {
            *((dst as isize + i) as *mut u8) = *((src as isize + i) as *mut u8);
            i += 1;
        }
    } else {
        let mut i = num - 1;
        while i >= 0 {
            *((dst as isize + i) as *mut u8) = *((src as isize + i) as *mut u8);
            i -= 1;
        }
    }
    dst
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn strlen(s: *mut u8) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

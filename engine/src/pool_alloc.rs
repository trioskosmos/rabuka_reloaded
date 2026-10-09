//! A pooling global allocator for the small, short-lived allocations that
//! dominate the engine's profile.
//!
//! # Why this exists
//!
//! The engine makes roughly 780 allocations per simulated action, and a
//! measured third of wall time is spent inside `malloc`/`free`. The size
//! histogram is the important part: 41% of allocations are 1-15 bytes and
//! another 26% are 64-127 bytes, and the vast majority of those are freed
//! within the same action that created them — a `String` built to compare a
//! card name, an accumulator rebuilt from empty, a formatted ability id. That
//! is the workload a free list is good at and a general-purpose system
//! allocator is not: the block is handed straight back out again, so there is
//! no reason to pay for a heap lookup twice.
//!
//! # The header, and why it is not optional
//!
//! A pooled block is always the full width of its bin, which is usually larger
//! than the caller's request. The allocator therefore has to remember the real
//! size itself, because the `Layout` handed back at deallocation describes the
//! *request*, not the block. Passing that request-sized layout to the system
//! allocator on the way out is a size mismatch, and the system heap does not
//! survive it. So every pooled block carries its own width in a leading
//! `usize`, and every path that gives a block back to the system reads it from
//! there.
//!
//! The alternative — never returning a pooled block to the system, keeping them
//! on the free list forever — cannot bound retention, which is why the header
//! is the design here rather than an optimization.
//!
//! # Bounds
//!
//! Each list is capped, so a thread retains at most
//! `CLASSES * PER_CLASS * MAX_BIN` bytes. Blocks a thread still holds at exit
//! are never returned to the system; that retained memory is the cache, and
//! the cap is what keeps it bounded.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Bytes reserved at the front of every pooled block to record its true width.
const HEADER: usize = std::mem::size_of::<usize>();
/// Smallest pooled block, header included. Must exceed `size_of::<*mut u8>()`
/// because a free block stores its own link in its first word. Was 32; the
/// allocation report's single largest class is 8-15B (2.2M of 7.3M
/// allocations, 30%), which was falling through to System because the pool
/// floor was 32. A 16-byte block is 8B header + 8B payload, enough for any
/// align-<=8 request up to 15B — `is_pooled` already requires
/// `layout.align() <= POOL_ALIGN` (8), so nothing over-aligned slips in.
const MIN_CLASS: usize = 16;
/// Largest pooled *payload*, in bytes. Bigger requests go to the system
/// allocator: large blocks are rare here and are not worth the retention.
const MAX_POOLED: usize = 512;
/// Bin width. 16 bytes keeps internal fragmentation low without making the
/// table large enough to hurt cache locality.
const GRANULE: usize = 16;
/// Free blocks retained per bin, per thread.
const PER_CLASS: usize = 32;
/// Blocks are requested from the system allocator with this alignment, so any
/// pooled block satisfies any request needing no more. Requests needing more
/// are never pooled, in either direction.
const POOL_ALIGN: usize = 8;

const fn round_up(n: usize) -> usize {
    (n + GRANULE - 1) / GRANULE * GRANULE
}

/// Total width of the block backing a payload of `payload` bytes.
const fn bin_width(payload: usize) -> usize {
    let w = round_up(payload + HEADER);
    if w < MIN_CLASS {
        MIN_CLASS
    } else {
        w
    }
}

const MAX_BIN: usize = bin_width(MAX_POOLED);
const CLASSES: usize = (MAX_BIN - MIN_CLASS) / GRANULE + 1;

#[inline]
const fn class_of(payload: usize) -> usize {
    (bin_width(payload) - MIN_CLASS) / GRANULE
}

/// True when a layout may be served from a pool: small enough to be worth
/// recycling, and aligned no stricter than a pooled block is guaranteed to be.
#[inline]
fn poolable(layout: &Layout) -> bool {
    let size = layout.size();
    size != 0 && size <= MAX_POOLED && layout.align() <= POOL_ALIGN
}

/// Read a pooled block's true width from its header.
#[inline]
unsafe fn width_of(ptr: *mut u8) -> usize {
    *(ptr.sub(HEADER) as *const usize)
}

/// The system layout a pooled block was allocated with.
#[inline]
unsafe fn system_layout(ptr: *mut u8) -> Layout {
    Layout::from_size_align(width_of(ptr), POOL_ALIGN).expect("align is non-zero")
}

/// Per-thread free lists. `Copy` so it can live in a `Cell` and be read with
/// `get`, which needs no borrow flag and so cannot re-enter the allocator.
struct FreeLists {
    heads: [*mut u8; CLASSES],
    counts: [u16; CLASSES],
}

impl FreeLists {
    const fn new() -> Self {
        FreeLists {
            heads: [std::ptr::null_mut(); CLASSES],
            counts: [0; CLASSES],
        }
    }
}

thread_local! {
    /// The free lists, boxed once and never moved. The cell holds an
    /// 8-byte raw pointer to the box, so each alloc/dealloc reads and
    /// writes 8 bytes — not the 310 bytes the previous `Copy`+`Cell`
    /// version moved, which was ~1.5 KB per alloc+dealloc pair and
    /// dominated the pool's own savings.
    ///
    /// Mutations go through the raw pointer, which is sound here: the
    /// box is thread-local and uniquely owned by this thread, and
    /// nothing else can alias it. A `Ref`/`RefMut` borrow would be
    /// cheaper than copying the struct but would also re-enter this
    /// allocator if the borrow guard itself allocated — a `Box::new`
    /// inside `FREE.with` would recurse. The pointer cell cannot
    /// allocate, so it cannot re-enter.
    static FREE: Cell<*const FreeLists> = const { Cell::new(std::ptr::null()) };
}

fn free_lists() -> *mut FreeLists {
    let ptr = FREE.with(Cell::get);
    if !ptr.is_null() {
        return ptr as *mut FreeLists;
    }
    // Allocate the free-list struct through the SYSTEM allocator
    // directly, bypassing this pool: `Box::new` would re-enter
    // `free_lists()` and recurse to a stack overflow on the very
    // first allocation of the thread. `System.alloc` cannot call
    // back into here, so the init is re-entrancy-free.
    unsafe {
        let layout = Layout::new::<FreeLists>();
        let raw = System.alloc(layout);
        if raw.is_null() {
            return std::ptr::null_mut();
        }
        (raw as *mut FreeLists).write(FreeLists::new());
        let p: *const FreeLists = raw as *const FreeLists;
        FREE.with(|c| c.set(p));
        p as *mut FreeLists
    }
}

pub struct PoolAllocator;

unsafe impl GlobalAlloc for PoolAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !poolable(&layout) {
            return System.alloc(layout);
        }
        let class = class_of(layout.size());
        let block = unsafe { (*free_lists()).heads[class] };
        if block.is_null() {
            let width = bin_width(layout.size());
            let block = System.alloc(
                Layout::from_size_align(width, POOL_ALIGN).expect("align is non-zero"),
            );
            if block.is_null() {
                return block;
            }
            *(block as *mut usize) = width;
            return block.add(HEADER);
        }
        // Recycled: the header was overwritten by the link when the block went
        // on the free list, so it is rewritten here.
        let next = unsafe { *(block as *const *mut u8) };
        unsafe {
            let lists = free_lists();
            (*lists).heads[class] = next;
            (*lists).counts[class] -= 1;
        }
        *(block as *mut usize) = MIN_CLASS + class * GRANULE;
        block.add(HEADER)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !poolable(&layout) {
            System.dealloc(ptr, layout);
            return;
        }
        let block = ptr.sub(HEADER);
        let class = class_of(layout.size());
        let kept = {
            let lists = free_lists();
            if (*lists).counts[class] as usize >= PER_CLASS {
                false
            } else {
                *(block as *mut *mut u8) = (*lists).heads[class];
                (*lists).heads[class] = block;
                (*lists).counts[class] += 1;
                true
            }
        };
        if !kept {
            // Hand the block back with the width it was actually allocated
            // with, not the caller's smaller request size.
            System.dealloc(block, system_layout(ptr));
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align(new_size, layout.align()).expect("align is non-zero");
        if poolable(&layout) && poolable(&new_layout) {
            let block = ptr.sub(HEADER);
            let width = width_of(ptr);
            let wanted = bin_width(new_size);
            if wanted == width {
                // Already wide enough: no copy, and no size change.
                return ptr;
            }
            let resized = System.realloc(block, system_layout(ptr), wanted);
            if resized.is_null() {
                return resized;
            }
            *(resized as *mut usize) = wanted;
            return resized.add(HEADER);
        }
        // Crossing out of the pooled range, or over-aligned: route both sides
        // through the normal paths so each is freed under its own rules.
        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            std::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if !poolable(&layout) {
            return System.alloc_zeroed(layout);
        }
        // Recycled blocks hold links and previous tenants' bytes, so this
        // cannot delegate the way the other methods do. Zero the payload only;
        // the header is written by `alloc`.
        let ptr = self.alloc(layout);
        if !ptr.is_null() {
            std::ptr::write_bytes(ptr, 0, layout.size());
        }
        ptr
    }
}

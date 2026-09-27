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
/// because a free block stores its own link in its first word.
const MIN_CLASS: usize = 32;
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
#[derive(Clone, Copy)]
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
    /// Const-initialized and `Copy`, so first access allocates nothing and
    /// registers no destructor. Both matter: a lazy initializer here would
    /// recurse into this very allocator.
    static FREE: Cell<FreeLists> = const { Cell::new(FreeLists::new()) };
}

pub struct PoolAllocator;

unsafe impl GlobalAlloc for PoolAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !poolable(&layout) {
            return System.alloc(layout);
        }
        let class = class_of(layout.size());
        let block = FREE.with(|lists| lists.get().heads[class]);
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
        let next = *(block as *const *mut u8);
        FREE.with(|lists| {
            let mut l = lists.get();
            l.heads[class] = next;
            l.counts[class] -= 1;
            lists.set(l);
        });
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
        let kept = FREE.with(|lists| {
            let mut l = lists.get();
            if l.counts[class] as usize >= PER_CLASS {
                false
            } else {
                *(block as *mut *mut u8) = l.heads[class];
                l.heads[class] = block;
                l.counts[class] += 1;
                lists.set(l);
                true
            }
        });
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

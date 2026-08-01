use std::alloc::Layout;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

const ARENA_SIZE: usize = 64 * 1024;

static mut ARENA_BUF: [u8; ARENA_SIZE] = [0u8; ARENA_SIZE];
static ARENA_POS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static DEPTH: Cell<u8> = const { Cell::new(0) };
    static BYPASS: Cell<u8> = const { Cell::new(0) };
}

#[inline]
pub fn is_arena_active() -> bool {
    DEPTH.with(|d| d.get() > 0)
}

/// Temporarily opt out of the arena so persistent (never-freed) allocations
/// go to the system allocator. Required for any arena reset: cached abilities,
/// leaked group-name lists, etc. must not live in the bump region.
pub fn arena_bypass_enter() {
    BYPASS.with(|b| {
        let v = b.get();
        b.set(v.saturating_add(1));
    });
}

pub fn arena_bypass_exit() {
    BYPASS.with(|b| {
        let v = b.get();
        debug_assert!(
            v > 0,
            "arena_bypass_exit without matching arena_bypass_enter"
        );
        b.set(v.saturating_sub(1));
    });
}

pub fn arena_enter() {
    DEPTH.with(|d| {
        let v = d.get();
        d.set(v + 1);
    });
}

pub fn arena_exit() {
    DEPTH.with(|d| {
        let v = d.get();
        debug_assert!(v > 0, "arena_exit without matching arena_enter");
        d.set(v.saturating_sub(1));
    });
}

#[inline]
pub fn arena_alloc(layout: Layout) -> Option<*mut u8> {
    if !is_arena_active() || BYPASS.with(|b| b.get() > 0) {
        return None;
    }
    let pos = ARENA_POS.load(Ordering::Relaxed);
    let aligned = (pos + layout.align() - 1) & !(layout.align() - 1);
    let end = aligned + layout.size();
    if end > ARENA_SIZE {
        return None;
    }
    ARENA_POS.store(end, Ordering::Relaxed);
    Some(unsafe { ARENA_BUF.as_mut_ptr().add(aligned) })
}

#[inline]
pub fn arena_contains_ptr(ptr: *mut u8) -> bool {
    let base = unsafe { ARENA_BUF.as_ptr() } as usize;
    let end = base + ARENA_SIZE;
    let p = ptr as usize;
    p >= base && p < end
}

pub fn arena_stats() -> (usize, usize) {
    (ARENA_POS.load(Ordering::Relaxed), ARENA_SIZE)
}

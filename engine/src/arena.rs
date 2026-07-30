use std::alloc::Layout;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

const ARENA_SIZE: usize = 64 * 1024;

static mut ARENA_BUF: [u8; ARENA_SIZE] = [0u8; ARENA_SIZE];
static ARENA_POS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static DEPTH: Cell<u8> = const { Cell::new(0) };
}

#[inline]
pub fn is_arena_active() -> bool {
    DEPTH.with(|d| d.get() > 0)
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
    if !is_arena_active() {
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

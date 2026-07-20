use std::alloc::Layout;
use std::sync::atomic::{AtomicUsize, Ordering};

const ARENA_SIZE: usize = 64 * 1024;

static mut ARENA_BUF: [u8; ARENA_SIZE] = [0u8; ARENA_SIZE];
static ARENA_POS: AtomicUsize = AtomicUsize::new(0);

#[inline]
pub fn is_arena_active() -> bool {
    ARENA_POS.load(Ordering::Relaxed) > 0
}

pub fn arena_enter() {}

pub fn arena_exit() {}

#[inline]
pub fn arena_alloc(layout: Layout) -> Option<*mut u8> {
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

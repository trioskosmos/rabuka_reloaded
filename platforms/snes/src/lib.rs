#![no_std]
#![feature(asm_experimental_arch)]
#![allow(unused_features)]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

pub mod display;
pub mod hardware;
pub mod input;
pub mod sneslib;

// Simple bump allocator over a static WRAM heap. A card game makes bounded,
// long-lived allocations; a bump allocator (never frees) is fine and avoids a
// per-allocation metadata cost on the 65816.
struct Bump;
static mut HEAP: [u8; 16384] = [0; 16384];
static mut OFFSET: usize = 0;

unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = HEAP.as_mut_ptr() as usize;
        let off = OFFSET;
        let aligned = (off + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > HEAP.len() {
            return core::ptr::null_mut();
        }
        OFFSET = end;
        (base + aligned) as *mut u8
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOC: Bump = Bump;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

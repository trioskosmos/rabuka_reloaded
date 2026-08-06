#![no_std]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};

// ---- Global allocator: bump allocator over Jaguar DRAM (below the framebuffer) ----
const HEAP_START: usize = 0x0006_0000;
const HEAP_END: usize = 0x000F_FFFF; // ~640KB

struct BumpAlloc;
static ALLOC: BumpAlloc = BumpAlloc;

#[global_allocator]
static GLOBAL_ALLOC: BumpAlloc = BumpAlloc;

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        static mut CURSOR: usize = HEAP_START;
        let start = CURSOR;
        let aligned = (start + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        if end > HEAP_END {
            core::ptr::null_mut()
        } else {
            CURSOR = end;
            aligned as *mut u8
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // bump allocator: no free
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn rabuka_jaguar_main() -> ! {
    loop {}
}

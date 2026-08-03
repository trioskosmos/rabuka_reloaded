#![no_std]
#![no_main]

extern crate alloc;

use alloc::alloc::handle_alloc_error;
use core::alloc::{GlobalAlloc, Layout};

// ── libnds FFI (BlocksDS via a tiny C shim). ─────────────────────
mod nds {
    extern "C" {
        pub fn nds_init();
        pub fn nds_printf(fmt: *const u8, ...);
        pub fn nds_clear();
        pub fn nds_wait_vblank();
        pub fn nds_scan_keys();
        pub fn nds_keys_held() -> i32;
        pub fn malloc(size: usize) -> *mut u8;
        pub fn free(ptr: *mut u8);
        pub fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    }
}

// ── Allocator: BlocksDS picolibc malloc (stable, proper heap) ───
struct PicolibcAlloc;
unsafe impl GlobalAlloc for PicolibcAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = nds::malloc(layout.size());
        if p.is_null() {
            handle_alloc_error(layout);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if !ptr.is_null() {
            nds::free(ptr);
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = nds::realloc(ptr, new_size);
        if p.is_null() {
            handle_alloc_error(layout);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = self.alloc(layout);
        if !p.is_null() {
            core::ptr::write_bytes(p, 0, layout.size());
        }
        p
    }
}
#[global_allocator]
static ALLOC: PicolibcAlloc = PicolibcAlloc;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("PANIC: {}", info);
    unsafe {
        nds::nds_clear();
        nds::nds_printf(b"PANIC\0".as_ptr());
        nds::nds_printf(msg.as_bytes().as_ptr());
    }
    loop {
        unsafe { nds::nds_wait_vblank() }
    }
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    unsafe {
        nds::nds_init();
        nds::nds_printf(b"\x1b[2JHello Rabuka DS!\x1b[1;0H\0".as_ptr());
    }

    let mut count = 0u32;
    loop {
        unsafe {
            nds::nds_scan_keys();
            let held = nds::nds_keys_held() as u32;
            nds::nds_printf(b"\x1b[1;0Hframe=%u keys=0x%x \0".as_ptr(), count, held);
            nds::nds_wait_vblank();
        }
        count += 1;
    }
}

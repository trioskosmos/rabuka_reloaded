#![no_std]

extern crate alloc;

use alloc::alloc::handle_alloc_error;
use core::alloc::{GlobalAlloc, Layout};

// ── libnds FFI (BlocksDS via a tiny C shim). ─────────────────────
pub mod ffi {
    extern "C" {
        pub fn nds_init();
        pub fn nds_printf(fmt: *const u8, ...);
        pub fn nds_print(text: *const u8);
        pub fn nds_print_len(text: *const u8, len: i32);
        pub fn nds_set_cursor(x: i32, y: i32);
        pub fn nds_clear_line(row: i32);
        pub fn nds_clear();
        pub fn nds_wait_vblank();
        pub fn nds_scan_keys();
        pub fn nds_keys_held() -> i32;
        pub fn nds_get_tick() -> u64;
        pub fn nds_nocash_log(text: *const u8);
        pub fn nds_set_backdrop_color(color: u16);
        pub fn malloc(size: usize) -> *mut u8;
        pub fn free(ptr: *mut u8);
        pub fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    }
}

pub mod decks_baked;
pub mod display;
pub mod input;

// ── Allocator: BlocksDS picolibc malloc (stable, proper heap) ───
struct PicolibcAlloc;
unsafe impl GlobalAlloc for PicolibcAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = ffi::malloc(layout.size());
        if p.is_null() {
            handle_alloc_error(layout);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if !ptr.is_null() {
            ffi::free(ptr);
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = ffi::realloc(ptr, new_size);
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

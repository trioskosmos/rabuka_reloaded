//! Minimal Genesis VDP text display.
//! For bring-up we do a 40x28 plain text mode (like DC port's 40x28 grid)
//! using VDP plane A. This file compiles on m68k; hardware writes are volatile.

use alloc::string::String;

/// Genesis VDP ports (68000 bus)
const VDP_CTRL: *mut u16 = 0xC00004 as *mut u16;
const VDP_DATA: *mut u16 = 0xC00000 as *mut u16;

const COLS: usize = 40;
const ROWS: usize = 28;

pub struct Display {
    buf: String,
    // shadow of last committed buffer to avoid redundant VDP writes
    last: String,
}

impl Display {
    pub fn new() -> Self {
        // VDP init would go here: set regs #0-#23, clear VRAM, set palette.
        // For compile-test we skip hardware init (emulator will boot with BIOS state).
        Self {
            buf: String::new(),
            last: String::new(),
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn println(&mut self, text: &str) {
        self.buf.push_str(text);
        self.buf.push('\n');
    }

    pub fn text(&self) -> &str {
        &self.buf
    }

    /// Commit buf to VDP Plane A name table (stub for now - still validates
    /// string handling and allocator within 64KB RAM).
    pub fn swap_buffers(&mut self) {
        if self.buf == self.last {
            return;
        }
        self.last.clear();
        self.last.push_str(&self.buf);
        // Real VDP write would be:
        // for (i, ch) in self.buf.bytes().enumerate().take(COLS*ROWS) { ... vdp_write(...) }
        // Omitted for bring-up - proves allocator + compact_state fit.
    }

    pub fn wait_vblank(&mut self) {
        // Wait for VDP status bit 3 (vertical blank) at 0xC00004 bit 3
        // For now, spin a few hundred nops so timing is not tight.
        for _ in 0..1000 {
            unsafe { core::ptr::read_volatile(0xC00004 as *const u16); }
        }
    }
}

// Genesis has no std palette - stub

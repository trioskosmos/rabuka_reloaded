use crate::ffi;

pub const COLS: usize = 32;
pub const ROWS: usize = 24;

/// Text display backed by BlocksDS's console (`consoleDemoInit` + printf path),
/// which handles the 4bpp font and ANSI escapes correctly. This is the same
/// rendering path as the BlocksDS `ansi_console` reference example.
pub struct Display {
    cursor_x: usize,
    cursor_y: usize,
}

impl Display {
    pub fn new() -> Self {
        Display {
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            ffi::nds_printf(b"\x1b[2J\0".as_ptr());
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn print(&mut self, text: &str) {
        unsafe {
            ffi::nds_print_len(text.as_ptr() as *const u8, text.len() as i32);
        }
        for ch in text.chars() {
            if ch == '\n' {
                self.cursor_y += 1;
                self.cursor_x = 0;
            } else if self.cursor_x < COLS {
                self.cursor_x += 1;
            }
        }
        if self.cursor_y >= ROWS {
            self.cursor_y = ROWS - 1;
        }
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        self.print("\n");
    }

    pub fn swap_buffers(&mut self) {
        unsafe { ffi::nds_wait_vblank() }
    }

    pub fn clear_line(&mut self, row: usize) {
        if row < ROWS {
            unsafe { ffi::nds_clear_line(row as i32) }
        }
    }
}

pub fn wait_vblank() {
    unsafe {
        ffi::nds_wait_vblank();
    }
}

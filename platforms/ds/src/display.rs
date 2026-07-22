pub const COLS: u32 = 32;
pub const ROWS: u32 = 24;

extern "C" {
    fn nds_console_clear();
    fn nds_print(text: *const u8);
    fn nds_println(text: *const u8);
    fn nds_clear_line(row: i32);
    fn nds_set_cursor(row: i32, col: i32);
    fn nds_wait_vblank();
    fn nds_write_tile_row(row: i32, tiles: *const u16);
}

pub struct Display {
    pub cursor_x: i32,
    pub cursor_y: i32,
}

impl Display {
    pub fn new() -> Self {
        Display {
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn clear(&mut self) {
        unsafe { nds_console_clear() }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn print(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                self.cursor_y += 1;
                self.cursor_x = 0;
                if self.cursor_y >= ROWS as i32 {
                    self.cursor_y = ROWS as i32 - 1;
                    unsafe { nds_set_cursor(self.cursor_y, 0) }
                }
                continue;
            }
            if self.cursor_x >= COLS as i32 {
                self.cursor_y += 1;
                self.cursor_x = 0;
                if self.cursor_y >= ROWS as i32 {
                    self.cursor_y = ROWS as i32 - 1;
                    unsafe { nds_set_cursor(self.cursor_y, 0) }
                }
            }
            if ch.is_ascii() {
                let c_str = [ch as u8, 0];
                unsafe { nds_print(c_str.as_ptr()) }
                self.cursor_x += 1;
            }
        }
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        let newline = [b'\n', 0];
        unsafe { nds_println(newline.as_ptr()) }
        self.cursor_x = 0;
        self.cursor_y += 1;
        if self.cursor_y >= ROWS as i32 {
            self.cursor_y = ROWS as i32 - 1;
            unsafe { nds_set_cursor(self.cursor_y, 0) }
        }
    }

    pub fn swap_buffers(&mut self) {
        unsafe { nds_wait_vblank() }
    }

    pub fn clear_line(&mut self, row: u32) {
        if row < ROWS {
            unsafe { nds_clear_line(row as i32) }
        }
    }

    pub fn write_screen(&mut self, rows: &[alloc::string::String]) {
        let mut tiles: [u16; 32] = [0; 32];
        for row_idx in 0..(rows.len().min(ROWS as usize)) {
            let bytes = rows[row_idx].as_bytes();
            let mut ti = 0usize;
            let mut si = 0usize;
            while si < bytes.len() && ti < 32 {
                let b = bytes[si];
                tiles[ti] = b as u16;
                ti += 1;
                si += 1;
            }
            while ti < 32 {
                tiles[ti] = 0x20; // space character
                ti += 1;
            }
            unsafe { nds_write_tile_row(row_idx as i32, tiles.as_ptr()) }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn draw_menu(&mut self, items: &[&str], selected: usize, title: &str) {
        self.clear();
        self.println(title);
        for (i, item) in items.iter().enumerate() {
            let prefix = if i == selected { ">" } else { " " };
            self.println(&alloc::format!("{prefix} {item}"));
        }
    }
}

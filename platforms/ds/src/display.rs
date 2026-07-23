pub const COLS: u32 = 32;
pub const ROWS: u32 = 24;

// After consoleLoadFont, fontCurPal = 15 << 12 = 0xF000 (palette 15 = white text).
// consoleGetDefault()->fontCurPal is NOT updated — it remains 0 (the static default).
// Hardcode the correct palette bits.
const FONT_PALETTE: u16 = 0xF000;

extern "C" {
    fn nds_wait_vblank();
    fn nds_write_tile_row(row: i32, tiles: *const u16);
}

pub struct Display {
    pub cursor_x: i32,
    pub cursor_y: i32,
    palette: u16,
    space_entry: u16,
    shadow: [u16; (COLS * ROWS) as usize],
}

impl Display {
    pub fn new() -> Self {
        let space = FONT_PALETTE | 0x20;
        Display {
            cursor_x: 0,
            cursor_y: 0,
            palette: FONT_PALETTE,
            space_entry: space,
            shadow: [space; (COLS * ROWS) as usize],
        }
    }

    pub fn clear(&mut self) {
        self.shadow.fill(self.space_entry);
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
                }
                continue;
            }
            if self.cursor_x >= COLS as i32 {
                self.cursor_y += 1;
                self.cursor_x = 0;
                if self.cursor_y >= ROWS as i32 {
                    self.cursor_y = ROWS as i32 - 1;
                }
            }
            if self.cursor_x < COLS as i32 && self.cursor_y < ROWS as i32 && ch.is_ascii() {
                let idx = (self.cursor_y * COLS as i32 + self.cursor_x) as usize;
                self.shadow[idx] = self.palette | (ch as u16);
                self.cursor_x += 1;
            }
        }
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        self.cursor_y += 1;
        self.cursor_x = 0;
    }

    pub fn swap_buffers(&mut self) {
        unsafe {
            nds_wait_vblank();
            for row in 0..ROWS as i32 {
                let offset = (row * COLS as i32) as usize;
                nds_write_tile_row(row, self.shadow[offset..].as_ptr());
            }
        }
    }

    pub fn clear_line(&mut self, row: u32) {
        if row < ROWS {
            let start = (row * COLS) as usize;
            let end = start + COLS as usize;
            for cell in self.shadow[start..end].iter_mut() {
                *cell = self.space_entry;
            }
        }
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

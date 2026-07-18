use core::ffi::{c_char, c_int, CStr};

extern "C" {
    static vid_mode: c_int;
    fn vid_set_mode(mode: c_int);
    fn vid_get_fb() -> *mut u16;
    fn bfont_draw(vram: *mut u16, x: c_int, y: c_int, str: *const c_char);
    fn bfont_draw_memset(vram: *mut u16, x: c_int, y: c_int, str: *const c_char);
    fn bfont_set_encoding(encoding: c_int) -> c_int;
}

const DM_320x240: c_int = 0;
const DM_640x480: c_int = 1;
const DM_256: c_int = 2;
const BFONT_ENC_ASCII: c_int = 0;
const BFONT_ENC_SJIS: c_int = 1;

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;
const FONT_W: i32 = 8;
const FONT_H: i32 = 16;
const COLUMNS: i32 = WIDTH as i32 / FONT_W;
const ROWS: i32 = HEIGHT as i32 / FONT_H;

pub struct Display {
    vram: *mut u16,
    cursor_x: i32,
    cursor_y: i32,
}

impl Display {
    pub fn new() -> Self {
        unsafe {
            vid_set_mode(DM_640x480);
            bfont_set_encoding(BFONT_ENC_ASCII);
        }
        let vram = unsafe { vid_get_fb() };
        let mut d = Display {
            vram,
            cursor_x: 0,
            cursor_y: 0,
        };
        d.clear();
        d
    }

    pub fn clear(&mut self) {
        let pixels = (WIDTH * HEIGHT) as usize;
        unsafe {
            core::ptr::write_bytes(self.vram, 0, pixels);
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn print(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                self.cursor_y += FONT_H;
                self.cursor_x = 0;
                if self.cursor_y >= HEIGHT as i32 {
                    self.scroll();
                }
                continue;
            }
            if ch == '\t' {
                let tab = 4;
                self.cursor_x = ((self.cursor_x / (FONT_W * tab)) + 1) * FONT_W * tab;
                continue;
            }
            self.draw_char(ch);
        }
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        self.cursor_y += FONT_H;
        self.cursor_x = 0;
        if self.cursor_y >= HEIGHT as i32 {
            self.scroll();
        }
    }

    pub fn write_at(&mut self, row: u32, col: u32, text: &str) {
        let start_x = (col as i32) * FONT_W;
        let start_y = (row as i32) * FONT_H;
        let old_x = self.cursor_x;
        let old_y = self.cursor_y;
        self.cursor_x = start_x;
        self.cursor_y = start_y;
        self.print(text);
        self.cursor_x = old_x;
        self.cursor_y = old_y;
    }

    pub fn clear_line(&mut self, row: u32) {
        let y = (row as usize) * FONT_H as usize;
        let line_words = WIDTH as usize * FONT_H as usize;
        unsafe {
            core::ptr::write_bytes(self.vram.add(y * WIDTH as usize), 0, line_words);
        }
    }

    pub fn draw_menu(&mut self, items: &[&str], selected: usize, title: &str) {
        self.clear();
        self.println(title);
        self.println(&"-".repeat(COLUMNS as usize));
        for (i, item) in items.iter().enumerate() {
            let prefix = if i == selected { " >" } else { "  " };
            let line = alloc::format!("{prefix} {item}");
            self.println(&line);
        }
    }

    pub fn swap_buffers(&mut self) {
        // Dreamcast vid_set_mode uses single-buffer; no swap needed.
    }

    fn draw_char(&mut self, ch: char) {
        if self.cursor_x + FONT_W > WIDTH as i32 {
            self.cursor_x = 0;
            self.cursor_y += FONT_H;
            if self.cursor_y >= HEIGHT as i32 {
                self.scroll();
            }
        }
        let mut buf = [0u8; 8];
        let s = ch.encode_utf8(&mut buf);
        let len = s.len();
        buf[len] = 0;
        let c_str = unsafe { CStr::from_bytes_with_nul_unchecked(&buf[..=len]) };
        unsafe {
            bfont_draw(self.vram, self.cursor_x, self.cursor_y, c_str.as_ptr());
        }
        self.cursor_x += FONT_W;
    }

    fn scroll(&mut self) {
        let row_words = WIDTH as usize;
        let total_rows = ROWS as usize;
        let scroll_rows = total_rows - 1;
        unsafe {
            core::ptr::copy(
                self.vram.add(row_words * FONT_H as usize),
                self.vram,
                row_words * scroll_rows * FONT_H as usize,
            );
            core::ptr::write_bytes(
                self.vram.add(row_words * scroll_rows * FONT_H as usize),
                0,
                row_words * FONT_H as usize,
            );
        }
        self.cursor_y = (scroll_rows as i32) * FONT_H;
    }
}

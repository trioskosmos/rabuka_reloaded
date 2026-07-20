use std::ffi::{c_char, c_int, CStr};
use std::format;
use std::string::ToString;

extern "C" {
    fn vid_set_mode(dm: c_int, pm: c_int);
    fn bfont_draw(buffer: *mut core::ffi::c_void, bufwidth: u32, opaque: bool, c: u32) -> usize;
    fn bfont_set_encoding(encoding: c_int) -> c_int;
    static vram_s: *mut u16;
}

const DM_640X480: c_int = 0;
const PM_RGB565: c_int = 3;
const BFONT_ENC_ASCII: c_int = 0;

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
            vid_set_mode(DM_640X480, PM_RGB565);
            bfont_set_encoding(BFONT_ENC_ASCII);
        }
        let vram = unsafe { vram_s };
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
            let line = std::format!("{prefix} {item}");
            self.println(&line);
        }
    }

    pub fn swap_buffers(&mut self) {}

    fn draw_char(&mut self, ch: char) {
        if self.cursor_x + FONT_W > WIDTH as i32 {
            self.cursor_x = 0;
            self.cursor_y += FONT_H;
            if self.cursor_y >= HEIGHT as i32 {
                self.scroll();
            }
        }
        for c in ch.to_string().bytes() {
            unsafe {
                bfont_draw(
                    self.vram
                        .add(self.cursor_y as usize * WIDTH as usize + self.cursor_x as usize)
                        as *mut core::ffi::c_void,
                    WIDTH,
                    true,
                    c as u32,
                );
            }
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

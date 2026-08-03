use psx::gpu::colors::BLACK;
use psx::gpu::VideoMode;
use psx::{Framebuffer, TextBox};

/// Text display on the PS1: double-buffered 320x240 framebuffer + the default
/// font text box (the same rendering path as the psx-sdk-rs examples).
pub struct Display {
    fb: Framebuffer,
    txt: TextBox,
}

impl Display {
    pub fn new() -> Display {
        let buf0 = (0, 0);
        let buf1 = (0, 240);
        let res = (320, 240);
        let mut fb =
            Framebuffer::new(buf0, buf1, res, VideoMode::NTSC, None).expect("framebuffer init");
        let font = fb.load_default_font();
        let txt = font.new_text_box((0, 8), res);
        Display { fb, txt }
    }

    pub fn clear(&mut self) {
        self.fb.set_bg_color(BLACK);
        self.txt.reset();
    }

    pub fn print(&mut self, text: &str) {
        for &b in text.as_bytes() {
            match b {
                b'\n' => self.txt.newline(),
                0x20..=0x7E => self.txt.print_char(b),
                _ => {}
            }
        }
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        self.txt.newline();
    }

    pub fn swap_buffers(&mut self) {
        self.fb.draw_sync();
        self.fb.wait_vblank();
        self.fb.swap();
    }

    pub fn clear_line(&mut self, _row: usize) {
        // No per-line erase; redraw the whole frame (text games redraw each frame).
    }
}

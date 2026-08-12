use core::ptr;

// Text screen buffer: 1-bpp bitmap rendered by the object processor. Must be
// in DRAM that doesn't overlap the Rust program's .data/.bss (0x4000..0x30C00)
// or heap (0x60000+). 0x42000 matches the OP bitmap object in rabuka_boot/boot.c.
pub const TEXT_SCREEN: usize = 0x0004_2000;
pub const MAX_X: usize = 320;
pub const MAX_Y: usize = 240;
const BYTES_PER_ROW: usize = MAX_X / 8; // 40
const CHARS_PER_ROW: usize = MAX_X / 8; // 40
const CHAR_ROWS: usize = MAX_Y / 8; // 30

// CLUT at $F00400; entry n is a 16-bit colour at offset 2n.
const CLUT: *mut u16 = 0x00F0_0400 as *mut u16;

static FONT: &[u8] = include_bytes!("font/light8x8.fnt");

pub struct Display {
    col: usize,
    row: usize,
}

impl Display {
    pub fn new() -> Display {
        let d = Display { col: 0, row: 0 };
        d.init_clut();
        d
    }

    // Set up the colour lookup table: black background, white text.
    // 1-bpp OP uses index 0: bit0 -> CLUT[0]=black (bg), bit1 -> CLUT[1]=white (text).
    fn init_clut(&self) {
        for i in 0..256 {
            unsafe { ptr::write_volatile(CLUT.add(i), 0x0000); }
        }
        // index 0 = background (black), index 1 = text (white)
        unsafe {
            ptr::write_volatile(CLUT.add(0), 0x0000);
            ptr::write_volatile(CLUT.add(1), 0xFFFF);
        }
    }

    pub fn clear(&mut self) {
        let len = BYTES_PER_ROW * MAX_Y;
        for i in 0..len {
            unsafe { ptr::write_volatile((TEXT_SCREEN + i) as *mut u8, 0); }
        }
        self.col = 0;
        self.row = 0;
    }

    pub fn print(&mut self, text: &str) {
        for &b in text.as_bytes() {
            match b {
                b'\n' => self.newline(),
                0x20..=0x7E => self.put_char(b),
                _ => {}
            }
        }
    }

    pub fn println(&mut self, text: &str) {
        self.print(text);
        self.newline();
    }

    fn newline(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row >= CHAR_ROWS {
            self.row = 0;
        }
    }

    fn put_char(&mut self, c: u8) {
        if self.col >= CHARS_PER_ROW {
            self.newline();
        }
        let glyph_off = 8 + (c as usize) * 8;
        for r in 0..8 {
            let addr = (self.row * 8 + r) * BYTES_PER_ROW + self.col;
            let byte = FONT[glyph_off + r];
            unsafe { ptr::write_volatile((TEXT_SCREEN + addr) as *mut u8, byte); }
        }
        self.col += 1;
    }

    pub fn swap_buffers(&mut self) {
        self.wait();
    }

    /// Pace to roughly a frame; single-buffered so no actual swap.
    pub fn wait(&mut self) {
        let mut n = 0;
        while n < 200_000 {
            n += 1;
            core::hint::spin_loop();
        }
    }
}

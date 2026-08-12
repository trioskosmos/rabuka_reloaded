use alloc::string::String;

use crate::hardware;
use crate::sneslib;

// PVSnesLib 8x16 font, uploaded as 2bpp CHR tiles. Each glyph is 32 bytes.
const FONT_BYTES: &[u8] = include_bytes!("../font.pic");
const GLYPH_BYTES: usize = 32; // 8x16, 2bpp
const TILE_BASE: u16 = 0x1000; // CHR VRAM word address
const TILEMAP_ADDR: u16 = 0x0000; // BG1 tilemap

/// Minimal SNES text display: accumulates lines and renders them as 8x16 font
/// tiles to BG1 in mode 0 (4-colour).
pub struct Display {
    buf: String,
    last: String,
    initialized: bool,
}

impl Display {
    pub fn new() -> Self {
        Display {
            buf: String::new(),
            last: String::new(),
            initialized: false,
        }
    }

    pub fn init(&mut self) {
        sneslib::ppu_off();
        hardware::write_vmain(0x80);
        sneslib::bg_scroll_zero();

        // Upload font tiles to VRAM CHR at TILE_BASE.
        sneslib::vram_set_addr(TILE_BASE);
        for chunk in FONT_BYTES.chunks(2) {
            if let [lo, hi] = *chunk {
                sneslib::vram_write(lo, hi);
            }
        }

        // Palette: index 0 = transparent/backdrop, 1 = white.
        sneslib::cgram_set(0, hardware::color(0, 0, 0));
        sneslib::cgram_set(1, hardware::color(31, 31, 31));

        // BG1 in mode 0, 4-colour (tilemap at $0000, CHR at $1000 via bg12nba=0x01).
        hardware::write_bgmode(0x00);
        hardware::write_bg1sc(0x00);
        hardware::write_bg12nba(0x01);
        hardware::write_tm(0x01);

        sneslib::ppu_on();
        self.initialized = true;
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn println(&mut self, text: &str) {
        self.buf.push_str(text);
        self.buf.push('\n');
    }

    pub fn swap_buffers(&mut self) {
        if !self.initialized {
            self.init();
        }
        if self.buf == self.last {
            return;
        }
        self.last = self.buf.clone();

        // Render each line of the buffer as 8x16 font tiles into the BG1 tilemap.
        sneslib::vram_set_addr(TILEMAP_ADDR);
        let mut map_index: u16 = 0;
        for line in self.buf.lines() {
            for &b in line.as_bytes() {
                let tile = glyph_tile(b);
                sneslib::vram_write(tile as u8, (tile >> 8) as u8);
                map_index += 1;
                if map_index >= 32 * 28 {
                    return;
                }
            }
            // advance to next tilemap row (32 tiles wide)
            let row = (map_index / 32 + 1) * 32;
            map_index = row;
            while map_index > 0 && (map_index as usize) % 32 != 0 {
                map_index = map_index.wrapping_add(1);
            }
        }
    }

    pub fn wait(&mut self) {
        sneslib::wait_vblank();
    }
}

/// Map an ASCII byte to a font tile index. PVSnesLib's font starts ' ' at tile 0
/// within the CHR range; printable ASCII maps linearly from index 0.
fn glyph_tile(b: u8) -> u16 {
    if b == b'\n' {
        return 0;
    }
    if b == b' ' {
        return 0;
    }
    if (0x21..=0x7e).contains(&b) {
        (b - 0x20) as u16 // ' ' = 0, '!' = 1, ...
    } else {
        0
    }
}

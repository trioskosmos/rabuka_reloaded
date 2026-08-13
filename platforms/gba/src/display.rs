use alloc::string::String;

use agb::display::tiled::{
    RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Priority, Rgb15};

use crate::font_tiles_gen::{FONT_GLYPHS, FONT_TILES};

/// Full-screen text via a pre-baked, per-screen-shared glyph tile set.
///
/// All glyphs are baked into `FONT_TILES` in ROM. Each screen references only the
/// glyphs it shows, so just those tiles are copied into VRAM (well under the
/// 1024-tile budget) and the background is recreated each screen so old tiles
/// free. No per-glyph-position tiles -> no VRAM exhaustion; no object cap -> full
/// screens; pre-baked bytes -> no runtime-rasterization garble.
pub struct Display<'a> {
    gfx: Graphics<'a>,
    buf: String,
    last: String,
}

impl<'a> Display<'a> {
    pub fn new(mut gfx: Graphics<'a>) -> Self {
        static PALETTE: Palette16 = const {
            let mut palette = [Rgb15::BLACK; 16];
            palette[1] = Rgb15::WHITE;
            Palette16::new(palette)
        };
        gfx.set_background_palette(0, &PALETTE);
        Display {
            gfx,
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

    fn glyph(ch: char) -> (u32, u32) {
        match FONT_GLYPHS.binary_search_by(|(c, _, _)| c.cmp(&ch)) {
            Ok(i) => (FONT_GLYPHS[i].1, FONT_GLYPHS[i].2),
            Err(_) => (0, 1),
        }
    }

    pub fn swap_buffers(&mut self) {
        if self.buf == self.last {
            return;
        }
        self.last = self.buf.clone();

        let tileset = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let mut bg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );

        let e = TileEffect::new(false, false, 0);
        let mut tx = 0usize;
        let mut ty = 0usize; // each line is 2 tile-rows tall (16px)

        for ch in self.buf.chars() {
            if ty >= 30 {
                break;
            }
            if ch == '\n' {
                tx = 0;
                ty += 2;
                continue;
            }
            let (idx, cols) = Self::glyph(ch);
            if tx + 1 >= 32 {
                tx = 0;
                ty += 2;
                if ty >= 30 {
                    break;
                }
            }
            // Every glyph is a 16x16 (2x2 tile) block at the current position.
            bg.set_tile((tx as i32, ty as i32), &tileset, TileSetting::new(idx as u16, e));
            bg.set_tile(
                (tx as i32 + 1, ty as i32),
                &tileset,
                TileSetting::new((idx + 1) as u16, e),
            );
            bg.set_tile(
                (tx as i32, ty as i32 + 1),
                &tileset,
                TileSetting::new((idx + 2) as u16, e),
            );
            bg.set_tile(
                (tx as i32 + 1, ty as i32 + 1),
                &tileset,
                TileSetting::new((idx + 3) as u16, e),
            );
            tx += cols as usize;
        }

        let mut frame = self.gfx.frame();
        bg.show(&mut frame);
        frame.commit();
    }

    pub fn wait(&mut self) {
        busy_wait_for_vblank();
    }
}

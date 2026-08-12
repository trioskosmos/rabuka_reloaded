use alloc::{string::String, vec::Vec};

use agb::display::font::{Font, Layout, LayoutSettings};
use agb::display::tiled::{
    RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Priority, Rgb15};

static FONT: Font = agb::include_font!("assets/NotoSubset.otf", 12);

/// Glyph cell size in pixels (each glyph occupies a CELLxCELL block of 8x8
/// tiles). 16px holds a readable 12px font and fullwidth Japanese.
const CELL: usize = 16;
const TILE: usize = 8;
const TILES_PER_GLYPH: usize = 4; // 2x2 tiles per glyph
/// Max unique glyphs per screen. Each uses 4 tiles; 120*4 = 480 tiles fits the
/// ~512-tile background VRAM budget (and is freed when the bg is recreated).
const MAX_GLYPHS: usize = 120;
/// Cells that fit on the 240x160 screen at 16px.
const COLS: usize = 15;
const ROWS: usize = 10;

#[repr(align(4))]
struct TileBuf([u8; MAX_GLYPHS * TILES_PER_GLYPH * 32]);
static mut TILE_DATA: TileBuf = TileBuf([0; MAX_GLYPHS * TILES_PER_GLYPH * 32]);

/// Tile-sharing text renderer on a tiled background.
///
/// Real (including Japanese) GBA games render text this way: rasterize each
/// unique glyph once into shared 8x8 tiles, then point the tile map at them via
/// `set_tile` (which dedups identical tiles in VRAM). This avoids the
/// sprite/object renderer's 128-object cap (whole screens fit) and the
/// per-glyph DynamicTile16 renderer's VRAM exhaustion (glyphs are shared, and
/// the background is recreated each screen so its tiles free on drop). Cursor
/// moves only rewrite tile-map entries, so navigation is fast.
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

    pub fn swap_buffers(&mut self) {
        if self.buf == self.last {
            return;
        }
        self.last = self.buf.clone();

        // 1. Collect the unique characters on this screen.
        let mut glyphs: Vec<(char, usize)> = Vec::new();
        for ch in self.buf.chars() {
            if ch == '\n' {
                continue;
            }
            if !glyphs.iter().any(|(c, _)| *c == ch) {
                glyphs.push((ch, glyphs.len()));
                if glyphs.len() >= MAX_GLYPHS {
                    break;
                }
            }
        }

        // 2. Rasterize each glyph into the shared static tile buffer.
        unsafe {
            for (ch, gidx) in glyphs.iter() {
                let bmp = rasterize_glyph(*ch);
                let base = gidx * TILES_PER_GLYPH * 32;
                for ty in 0..2usize {
                    for tx in 0..2usize {
                        let tile = pack_tile(&bmp, tx, ty);
                        let dst = base + (ty * 2 + tx) * 32;
                        TILE_DATA.0[dst..dst + 32].copy_from_slice(&tile);
                    }
                }
            }
        }
        let tile_bytes = glyphs.len() * TILES_PER_GLYPH * 32;
        let tileset = unsafe { TileSet::new(&TILE_DATA.0[..tile_bytes], TileFormat::FourBpp) };

        // 3. Build a fresh background and fill the tile map.
        let mut bg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );
        let mut cx = 0usize;
        let mut cy = 0usize;
        for ch in self.buf.chars() {
            if cy >= ROWS {
                break;
            }
            if ch == '\n' {
                cx = 0;
                cy += 1;
                continue;
            }
            if cx >= COLS {
                cx = 0;
                cy += 1;
                continue;
            }
            let gidx = glyphs
                .iter()
                .find(|(c, _)| *c == ch)
                .map(|(_, i)| *i)
                .unwrap_or(0);
            set_cell(&mut bg, &tileset, cx, cy, gidx);
            cx += 1;
        }

        let mut frame = self.gfx.frame();
        bg.show(&mut frame);
        frame.commit();
    }

    pub fn wait(&mut self) {
        busy_wait_for_vblank();
    }
}

fn set_cell(bg: &mut RegularBackground, ts: &TileSet, cx: usize, cy: usize, gidx: usize) {
    let b = gidx * TILES_PER_GLYPH;
    let px = cx * 2;
    let py = cy * 2;
    let e = TileEffect::new(false, false, 0);
    bg.set_tile((px, py), ts, TileSetting::new(b as u16, e));
    bg.set_tile((px + 1, py), ts, TileSetting::new((b + 1) as u16, e));
    bg.set_tile((px, py + 1), ts, TileSetting::new((b + 2) as u16, e));
    bg.set_tile((px + 1, py + 1), ts, TileSetting::new((b + 3) as u16, e));
}

/// Rasterize a single character into a CELLxCELL palette-index bitmap
/// (0 = transparent, 1 = foreground/white).
fn rasterize_glyph(ch: char) -> [u8; CELL * CELL] {
    let mut bmp = [0u8; CELL * CELL];
    let mut sbuf = [0u8; 4];
    let txt = ch.encode_utf8(&mut sbuf);
    let layout = Layout::new(txt, &FONT, &LayoutSettings::new());
    for group in layout {
        for (pos, pal) in group.pixels() {
            let x = pos.x as i32;
            let y = pos.y as i32;
            // Small vertical offset so the glyph sits inside the 16px cell.
            let y = y + 2;
            if x >= 0 && y >= 0 && (x as usize) < CELL && (y as usize) < CELL && pal != 0 {
                bmp[(y as usize) * CELL + (x as usize)] = 1;
            }
        }
    }
    bmp
}

/// Pack an 8x8 sub-tile of the glyph bitmap into GBA 4bpp tile bytes (32 bytes).
fn pack_tile(bmp: &[u8; CELL * CELL], tx: usize, ty: usize) -> [u8; 32] {
    let mut tile = [0u8; 32];
    for r in 0..TILE {
        for c in 0..TILE {
            let src = (ty * TILE + r) * CELL + (tx * TILE + c);
            let val = bmp[src] & 0xF;
            let byte = r * 4 + c / 2;
            let shift = if c % 2 == 0 { 4 } else { 0 };
            tile[byte] |= val << shift;
        }
    }
    tile
}

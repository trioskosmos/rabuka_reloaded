use alloc::string::String;

use agb::display::tiled::{
    RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Priority, Rgb15, Rgb};

use crate::board::BoardFrame;
use crate::card_art_gen::CardArt;
use crate::font_tiles_gen::{FONT_GLYPHS, FONT_TILES};

/// Screen is 240x160 = 30 cols x 20 rows of 8px tiles. Each text line is 2
/// tile-rows (16px).
const COLS: i32 = 30;
const ROWS: i32 = 20;

/// Full-screen text via a pre-baked, per-screen-shared glyph tile set, plus a
/// tiled board (card backs, zones, cursor) rendered from a [`BoardFrame`].
pub struct Display<'a> {
    gfx: Graphics<'a>,
    buf: String,
    last: String,
}

/// 4bpp text/board palette (bank 0).
static TEXT_PALETTE: Palette16 = const {
    let mut palette = [Rgb15::BLACK; 16];
    palette[0] = Rgb15::BLACK;
    palette[1] = Rgb15::WHITE;
    palette[2] = Rgb::new(26, 35, 50).to_rgb15(); // zone fill
    palette[3] = Rgb::new(42, 58, 90).to_rgb15(); // card back
    palette[4] = Rgb::new(245, 158, 11).to_rgb15(); // gold
    palette[5] = Rgb::new(46, 204, 113).to_rgb15(); // green
    palette[6] = Rgb::new(160, 174, 192).to_rgb15(); // dim
    Palette16::new(palette)
};

/// Text palette for the card-detail view, on bank 15 (reserved from the art's
/// 240-colour palette so the two never collide).
static DETAIL_TEXT_PALETTE: Palette16 = const {
    let mut palette = [Rgb15::BLACK; 16];
    palette[0] = Rgb15::BLACK;
    palette[1] = Rgb15::WHITE;
    palette[6] = Rgb::new(160, 174, 192).to_rgb15(); // dim
    Palette16::new(palette)
};

impl<'a> Display<'a> {
    pub fn new(gfx: Graphics<'a>) -> Self {
        gfx.set_background_palette(0, &TEXT_PALETTE);
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

    /// The buffered text (action-bar lines) for the board renderer.
    pub fn text(&self) -> &str {
        &self.buf
    }

    fn glyph(ch: char) -> (u32, u32) {
        match FONT_GLYPHS.binary_search_by(|(c, _, _)| c.cmp(&ch)) {
            Ok(i) => (FONT_GLYPHS[i].1, FONT_GLYPHS[i].2),
            Err(_) => (0, 1),
        }
    }

    /// Place a 16px (2x2 tile) glyph at (tx, ty); returns its width in tiles.
    fn place_glyph(bg: &mut RegularBackground, ts: &TileSet, e: TileEffect, ch: char, tx: i32, ty: i32) -> i32 {
        let (idx, cols) = Self::glyph(ch);
        bg.set_tile((tx, ty), ts, TileSetting::new(idx as u16, e));
        bg.set_tile((tx + 1, ty), ts, TileSetting::new((idx + 1) as u16, e));
        bg.set_tile((tx, ty + 1), ts, TileSetting::new((idx + 2) as u16, e));
        bg.set_tile((tx + 1, ty + 1), ts, TileSetting::new((idx + 3) as u16, e));
        cols as i32
    }

    /// Render one pre-wrapped line of 16px text at (tx0, ty), clipped to COLS.
    fn blit_line(bg: &mut RegularBackground, ts: &TileSet, e: TileEffect, text: &str, tx0: i32, ty: i32) {
        let mut tx = tx0;
        for ch in text.chars() {
            let (_, cols) = Self::glyph(ch);
            if tx + cols as i32 > COLS {
                break;
            }
            tx += Self::place_glyph(bg, ts, e, ch, tx, ty);
        }
    }

    /// Render a board frame as clear text lines (board from the top, action
    /// bar at the bottom).
    pub fn render_board_frame(&mut self, frame: &BoardFrame) {
        self.last = self.buf.clone();
        self.gfx.set_background_palette(0, &TEXT_PALETTE);
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let e = TileEffect::new(false, false, 0);

        let mut tbg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );

        // Board text from the top.
        let mut row = 0i32;
        for line in &frame.lines {
            if row + 2 > ROWS {
                break;
            }
            Self::blit_line(&mut tbg, &font_ts, e, line, 0, row);
            row += 2;
        }

        // Action bar pinned to the bottom (last two lines).
        let n = frame.action_lines.len();
        for k in 0..2 {
            if k >= n {
                break;
            }
            let r = ROWS - 2 - 2 * k as i32;
            Self::blit_line(&mut tbg, &font_ts, e, &frame.action_lines[n - 1 - k], 0, r);
        }

        let mut f = self.gfx.frame();
        tbg.show(&mut f);
        f.commit();
    }

    /// Render the full-screen Action view: the buffered action text with a
    /// small hint line at the top.
    pub fn render_action_text(&mut self) {
        self.last = self.buf.clone();
        self.gfx.set_background_palette(0, &TEXT_PALETTE);
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let e = TileEffect::new(false, false, 0);

        let mut tbg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );

        Self::blit_line(&mut tbg, &font_ts, e, "ACTIONS  [Select: Board]", 0, 0);
        let mut row = 2i32;
        for line in self.buf.split('\n') {
            if row + 2 > ROWS {
                break;
            }
            Self::blit_line(&mut tbg, &font_ts, e, line, 0, row);
            row += 2;
        }

        let mut f = self.gfx.frame();
        tbg.show(&mut f);
        f.commit();
    }

    /// Render a card-detail view: 8bpp art on a P0 background (indices 0-239)
    /// with 4bpp text on P1 using palette bank 15 (indices 240-255).
    pub fn render_card_detail(&mut self, art: Option<&CardArt>, lines: &[String]) {
        self.last = self.buf.clone();
        self.gfx.set_background_palette(15, &DETAIL_TEXT_PALETTE);
        if let Some(art) = art {
            for i in 0..240 {
                let v = art.palette[i * 2] as u16 | ((art.palette[i * 2 + 1] as u16) << 8);
                self.gfx.set_background_palette_colour_256(i, Rgb15::new(v));
            }
        }
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let e_text = TileEffect::new(false, false, 15);

        let mut f = self.gfx.frame();

        if let Some(art) = art {
            let art_ts = unsafe { TileSet::new(art.tiles, TileFormat::EightBpp) };
            let mut abg = RegularBackground::new(
                Priority::P0,
                RegularBackgroundSize::Background32x32,
                TileFormat::EightBpp,
            );
            for i in 0..192 {
                let tx = (i % 12) as i32;
                let ty = (i / 12) as i32;
                abg.set_tile((tx, ty), &art_ts, TileSetting::new(i as u16, TileEffect::new(false, false, 0)));
            }
            abg.show(&mut f);
        }

        let mut tbg = RegularBackground::new(
            Priority::P1,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );
        for (idx, line) in lines.iter().enumerate() {
            Self::blit_line(&mut tbg, &font_ts, e_text, line, 13, idx as i32 * 2);
        }
        tbg.show(&mut f);
        f.commit();
    }

    pub fn swap_buffers(&mut self) {
        if self.buf == self.last {
            return;
        }
        self.last = self.buf.clone();
        self.gfx.set_background_palette(0, &TEXT_PALETTE);

        let tileset = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let mut bg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );

        let e = TileEffect::new(false, false, 0);
        let mut tx = 0i32;
        let mut ty = 1i32; // 8px top margin

        for ch in self.buf.chars() {
            if ch == '\n' {
                tx = 0;
                ty += 2;
                continue;
            }
            let (_, cols) = Self::glyph(ch);
            if tx + cols as i32 > COLS {
                tx = 0;
                ty += 2;
            }
            if ty + 2 > ROWS {
                break;
            }
            tx += Self::place_glyph(&mut bg, &tileset, e, ch, tx, ty);
        }

        let mut frame = self.gfx.frame();
        bg.show(&mut frame);
        frame.commit();
    }

    pub fn wait(&mut self) {
        busy_wait_for_vblank();
    }
}

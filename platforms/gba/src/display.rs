use alloc::string::String;

use agb::display::tiled::{
    RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Priority, Rgb15, Rgb};

use crate::board::BoardFrame;
use crate::card_art_gen::{CardArt, BOARD_UI, CARD_FRONTS, LIVE_FRONTS, MASTER_PAL, STAGE_FRONTS};
use crate::font_tiles_gen::{FONT_GLYPHS, FONT_TILES};

/// Screen is 240x160 = 30 cols x 20 rows of 8px tiles. Text glyphs are
/// 16x16 (2x2 tiles, 12px font) — each text line is 2 tile-rows.
pub const COLS: i32 = 30;
pub const ROWS: i32 = 20;
const FONT_ROWS: i32 = 2;

/// Card tile sizes baked in `tools/bake_card_art.py` (multiples of 8px).
const STAGE_CARD: (i32, i32) = (5, 6); // 40x48
const HAND_CARD: (i32, i32) = (3, 4); // 24x32

/// Layout derived from card sizes + font metrics (no magic absolute Y).
const HEADER_H: i32 = FONT_ROWS; // 2
const STAGE_H: i32 = STAGE_CARD.1; // 6
const HAND_H: i32 = HAND_CARD.1; // 4
const BAR_H: i32 = FONT_ROWS; // single-line action bar

const STAGE_PITCH: i32 = STAGE_CARD.0 + 1; // 1 col gap for badge
const HAND_PITCH: i32 = HAND_CARD.0 + 1; // 1 col gap
const STAGE_START_X: i32 = 1;
const LIVE_CARD: (i32, i32) = (2, 3); // 16x24 mini live
const LIVE_PITCH: i32 = LIVE_CARD.0 + 1; // 3
const HAND_START_X: i32 = 0;

const STAGE_YS: [i32; 2] = [HEADER_H, HEADER_H + STAGE_H]; // [2, 8]
const HAND_Y: i32 = STAGE_YS[1] + STAGE_H; // 14
const BAR_Y: i32 = ROWS - BAR_H; // 18
const INFO_X: i32 = STAGE_START_X + STAGE_PITCH * 2 + STAGE_CARD.0 + 1; // 19

/// Hand capacity derived from screen width, not hardcoded (30 cols / 4 pitch = 7).
pub const HAND_FITS: usize = (COLS / HAND_PITCH) as usize; // 7

/// Board UI tile indices inside [`BOARD_UI`] (4bpp text BG, bank 15).
/// Single solid gray tile repeated for all empty zones (VRAM-cheap).
const UI_EMPTY: u16 = 0; // 1 tile, solid zone fill
const UI_BADGE: u16 = 1; // gold actionable diamond
const UI_MARKER: u16 = 2; // white focus triangle
const UI_GOLD: u16 = 3; // solid gold for cursor border

/// Full-screen text via a pre-baked, per-screen-shared glyph tile set, plus a
/// tiled board (card fronts, zones, cursor) rendered from a [`BoardFrame`].
pub struct Display<'a> {
    gfx: Graphics<'a>,
    buf: String,
    last: String,
    detail_active: bool,
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
/// 240-colour palette so the two never collide). Matches 3DS COL_CARD_OPAQUE
/// dark panel + white text.
static DETAIL_TEXT_PALETTE: Palette16 = const {
    let mut palette = [Rgb15::BLACK; 16];
    palette[0] = Rgb15::BLACK;
    palette[1] = Rgb15::WHITE;
    palette[2] = Rgb::new(26, 35, 50).to_rgb15(); // zone fill behind text
    palette[6] = Rgb::new(160, 174, 192).to_rgb15(); // dim
    Palette16::new(palette)
};

impl<'a> Display<'a> {
    pub fn new(mut gfx: Graphics<'a>) -> Self {
        // Master 240-colour palette for all card fronts (8bpp BG, indices 0-239)
        // Loaded once here, not per-frame, to avoid mid-frame palette writes
        // flashing the backdrop (black rectangle top left).
        for i in 0..240 {
            let v = MASTER_PAL[i * 2] as u16 | ((MASTER_PAL[i * 2 + 1] as u16) << 8);
            gfx.set_background_palette_colour_256(i, Rgb15::new(v));
        }
        gfx.set_background_palette(15, &TEXT_PALETTE);
        Display {
            gfx,
            buf: String::new(),
            last: String::new(),
            detail_active: false,
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

    /// Render one line of 16px text at (tx0, ty), clipped to COLS.
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

    /// Render the integrated board: 8bpp shared-palette card art on an
    /// 8bpp BG underneath a 4bpp text/UI BG, as Tonc/butano recommend for
    /// many colourful tiles (one 240-colour master palette + bank 15 for text).
    pub fn render_board_frame(&mut self, frame: &BoardFrame) {
        self.last = self.buf.clone();
        if self.detail_active {
            // Detail view overwrote 0-239 with per-card palette — restore master
            for i in 0..240 {
                let v = MASTER_PAL[i * 2] as u16 | ((MASTER_PAL[i * 2 + 1] as u16) << 8);
                self.gfx.set_background_palette_colour_256(i, Rgb15::new(v));
            }
            self.detail_active = false;
        }
        // TEXT_PALETTE may have been overwritten by detail view's
        // DETAIL_TEXT_PALETTE — restore it
        self.gfx.set_background_palette(15, &TEXT_PALETTE);
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
        let e0 = TileEffect::new(false, false, 15);

        let mut art_bg = RegularBackground::new(
            Priority::P1,
            RegularBackgroundSize::Background32x32,
            TileFormat::EightBpp,
        );
        let mut ui_bg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );

        // Solid board background on ui BG so gaps/margins are dark blue, not
        // backdrop black. Card interiors will be cleared to transparent per-slot
        // so the 8bpp art underneath shows through.
        for ty in 0..ROWS {
            for tx in 0..COLS {
                ui_bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e0));
            }
        }
        Self::blit_line(&mut ui_bg, &font_ts, e0, &frame.header, 0, 0);
        Self::blit_line(&mut ui_bg, &font_ts, e0, &frame.action_count, COLS - 6, 0);

        // Stage rows: opponent then player, live zone + info in the right column.
        for (row, y) in STAGE_YS.iter().enumerate() {
            let y = *y;
            let stage = if row == 0 { &frame.p2_stage } else { &frame.p1_stage };
            let live = if row == 0 { &frame.p2_live } else { &frame.p1_live };
            for (i, slot) in stage.iter().enumerate() {
                let x = 1 + STAGE_PITCH * i as i32;
                draw_slot(
                    &mut art_bg,
                    &mut ui_bg,
                    &ui_ts,
                    &font_ts,
                    e0,
                    slot,
                    x,
                    y,
                    STAGE_CARD,
                    STAGE_FRONTS,
                );
            }
            // Live/success zone: 3 mini cards (16x16) at the stage's right
            for (i, slot) in live.iter().enumerate() {
                let x = INFO_X + LIVE_PITCH * i as i32;
                draw_slot(
                    &mut art_bg,
                    &mut ui_bg,
                    &ui_ts,
                    &font_ts,
                    e0,
                    slot,
                    x,
                    y,
                    LIVE_CARD,
                    LIVE_FRONTS,
                );
            }
            let info = if row == 0 { &frame.p2_info } else { &frame.p1_info };
            Self::blit_line(&mut ui_bg, &font_ts, e0, &info[0], INFO_X, y + 2);
            Self::blit_line(&mut ui_bg, &font_ts, e0, &info[1], INFO_X, y + 4);
        }

        // Hand window; a gold badge marks more cards off-screen right.
        for (i, slot) in frame.hand.iter().enumerate() {
            let x = HAND_PITCH * i as i32;
            draw_slot(
                &mut art_bg,
                &mut ui_bg,
                &ui_ts,
                &font_ts,
                e0,
                slot,
                x,
                HAND_Y,
                HAND_CARD,
                CARD_FRONTS,
            );
        }
        if frame.hand_more {
            ui_bg.set_tile(
                (COLS - 1, HAND_Y),
                &ui_ts,
                TileSetting::new(UI_BADGE, e0),
            );
        }

        // Hand cursor: small white marker on the top-left corner of the
        // cursored card (overlay on the card itself, not in the gap — so the
        // first card doesn't get covered by a gap tile and more cards fit).
        if let Some(w) = frame.hand_cursor {
            let x = HAND_PITCH * w as i32;
            ui_bg.set_tile((x, HAND_Y), &ui_ts, TileSetting::new(UI_MARKER, e0));
        }

        // Action bar pinned to the bottom (single 16px line; hint lives in header).
        let bar = alloc::format!("> {}", frame.action_line);
        Self::blit_line(&mut ui_bg, &font_ts, e0, &bar, 0, BAR_Y);

        let mut f = self.gfx.frame();
        art_bg.show(&mut f);
        ui_bg.show(&mut f);
        f.commit();
    }

    /// Render the full-screen Actions view: the buffered action list with a
    /// small hint line at the top. Input stays with the engine so Up/Down/A
    /// drive the list live.
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
    /// `scroll` is the first visible line index — paginated like 3DS detail
    /// (render.rs:437 lpp 10) to avoid VRAM blow-up with long ability text.
    pub fn render_card_detail(&mut self, art: Option<&CardArt>, lines: &[String], scroll: usize) {
        self.last = self.buf.clone();
        self.detail_active = true;
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
        // Dark panel behind ability text like 3DS COL_CARD_OPAQUE (render.rs:498)
        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
        for ty in 0..ROWS {
            for tx in 13..COLS {
                tbg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e_text));
            }
        }
        const VISIBLE: usize = 8;
        let end = (scroll + VISIBLE).min(lines.len());
        for (i, line) in lines[scroll..end].iter().enumerate() {
            Self::blit_line(&mut tbg, &font_ts, e_text, line, 13, i as i32 * 2);
        }
        // Scroll indicators like 3DS detail (render.rs:568)
        if scroll > 0 {
            Self::blit_line(&mut tbg, &font_ts, e_text, "^", 29, 0);
        }
        if end < lines.len() {
            Self::blit_line(&mut tbg, &font_ts, e_text, "v", 29, 18);
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
        let mut ty = 0i32;

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

/// Draw one card slot: 8bpp shared-palette front art on the art BG (or a
/// solid gray slot on the text BG), plus the gold badge in the right gap
/// column when the card has valid actions.
fn draw_slot(
    art_bg: &mut RegularBackground,
    ui_bg: &mut RegularBackground,
    ui_ts: &TileSet,
    font_ts: &TileSet,
    e0: TileEffect,
    slot: &crate::board::Slot,
    x: i32,
    y: i32,
    card: (i32, i32),
    fronts: &[crate::card_art_gen::CardFront],
    ) {
    let (cols, rows) = card;
    let mut empty = |bg: &mut RegularBackground| {
        for ty in 0..rows {
            for tx in 0..cols {
                bg.set_tile((x + tx, y + ty), ui_ts, TileSetting::new(UI_EMPTY, e0));
            }
        }
    };
    match &slot.card_no {
        Some(card_no) => match fronts.iter().find(|f| f.card_no == card_no.as_str()) {
            Some(front) => {
                let ts = unsafe { TileSet::new(front.tiles, TileFormat::EightBpp) };
                for ty in 0..rows {
                    for tx in 0..cols {
                        art_bg.set_tile(
                            (x + tx, y + ty),
                            &ts,
                            TileSetting::new((ty * cols + tx) as u16, TileEffect::new(false, false, 0)),
                        );
                        // Clear ui_bg so card art shows through (ui is in front)
                        ui_bg.set_tile((x + tx, y + ty), &ui_ts, TileSetting::BLANK);
                    }
                }
            }
            None => {
                empty(ui_bg);
                Display::blit_line(ui_bg, font_ts, e0, card_no, x, y + rows / 2);
            }
        },
        None => empty(ui_bg),
    }
    if slot.actionable {
        // Badge in the right gap column; if there is none (last hand slot),
        // overlay the card's top-right corner tile.
        let bx = if x + cols < COLS {
            x + cols
        } else {
            x + cols - 1
        };
        ui_bg.set_tile((bx, y), ui_ts, TileSetting::new(UI_BADGE, e0));
    }
}

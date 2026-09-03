use alloc::string::String;
use alloc::vec::Vec;

use agb::display::tiled::{
    RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Priority, Rgb15, Rgb};

use crate::board::BoardFrame;
use crate::card_art_gen::{CardArt, BOARD_UI, CARD_FRONTS, LIVE_FRONTS, MASTER_PAL, STAGE_FRONTS, WAITED_FRONTS};
use crate::font_tiles_gen::{FONT_GLYPHS, FONT_TILES};
use crate::texticons_gen::{TEXTICON_GLYPHS, TEXTICON_TILES};

/// Screen is 240x160 = 30 cols x 20 rows of 8px tiles. Text glyphs are
/// 16x16 (2x2 tiles, 12px font) — each text line is 2 tile-rows.
pub const COLS: i32 = 30;
pub const ROWS: i32 = 20;
const FONT_ROWS: i32 = 2;

/// Card tile grids baked in `tools/bake_card_art.py` (multiples of 8px).
/// The card pixels inside each grid are tuned to the source's 0.716 aspect;
/// the grid itself is what the layout uses, so cards sit at their real shape
/// with padding instead of being forced to a grid aspect.
const STAGE_CARD: (i32, i32) = (5, 6); // 40x48 grid, 34x48 card
const HAND_CARD: (i32, i32) = (3, 4); // 24x32 grid, 24x32 card

/// Layout derived from card sizes + font metrics (no magic absolute Y).
const HEADER_H: i32 = FONT_ROWS; // 2
const STAGE_H: i32 = STAGE_CARD.1; // 6
#[allow(dead_code)]
const HAND_H: i32 = HAND_CARD.1; // 4
const BAR_H: i32 = FONT_ROWS; // single-line action bar

const STAGE_PITCH: i32 = STAGE_CARD.0; // badge on card saves 1 col
const HAND_PITCH: i32 = HAND_CARD.0;
const STAGE_START_X: i32 = 1;
const LIVE_CARD: (i32, i32) = (3, 2); // 24x16 landscape (live cards are wide)
const LIVE_PITCH: i32 = LIVE_CARD.0; // badge on card
#[allow(dead_code)]
const HAND_START_X: i32 = 0;
const STAGE_YS: [i32; 2] = [HEADER_H, HEADER_H + STAGE_H]; // [2, 8]
const HAND_Y: i32 = STAGE_YS[1] + STAGE_H; // 14
const BAR_Y: i32 = ROWS - BAR_H; // 18
const INFO_X: i32 = STAGE_START_X + STAGE_PITCH * 3 + 1; // 14

/// Hand capacity derived from screen width, not hardcoded (30 cols / 3 pitch = 10).
#[allow(dead_code)]
pub const HAND_FITS: usize = (COLS / HAND_PITCH) as usize; // 10

/// Board UI tile indices inside [`BOARD_UI`] (4bpp text BG, bank 15).
/// Single solid gray tile repeated for all empty zones (VRAM-cheap).
const UI_EMPTY: u16 = 0; // 1 tile, solid zone fill
const UI_BADGE: u16 = 1; // gold actionable diamond
const UI_MARKER: u16 = 2; // white focus triangle
#[allow(dead_code)]
const UI_GOLD: u16 = 3; // solid gold for cursor border

/// Full-screen text via a pre-baked, per-screen-shared glyph tile set, plus a
/// tiled board (card fronts, zones, cursor) rendered from a [`BoardFrame`].
pub struct Display<'a> {
    gfx: Graphics<'a>,
    buf: String,
    last: String,
    detail_active: bool,
    /// Card art queued via [`Display::queue_card_image`] since the last
    /// [`Display::clear`]. `swap_buffers` composites these on an 8bpp BG
    /// underneath the text BG (punching transparent holes in the text fill),
    /// so generic choice menus can show card images like the 3DS grid.
    /// Coordinates are in tiles; `selected` draws a gold border.
    pending_art: Vec<PendingArt>,
}

/// One queued card image for the next [`Display::swap_buffers`].
struct PendingArt {
    card_no: String,
    x: i32,
    y: i32,
    cols: i32,
    rows: i32,
    selected: bool,
}

/// 4bpp text/board palette (bank 0). Entries 7..=15 are the texticon colours,
/// mirrored by `tools/bake_texticon_tiles.py` PALETTE_TARGETS — keep in sync.
static TEXT_PALETTE: Palette16 = const {
    let mut palette = [Rgb15::BLACK; 16];
    palette[0] = Rgb15::BLACK;
    palette[1] = Rgb15::WHITE;
    palette[2] = Rgb::new(26, 35, 50).to_rgb15(); // zone fill
    palette[3] = Rgb::new(42, 58, 90).to_rgb15(); // card back
    palette[4] = Rgb::new(245, 158, 11).to_rgb15(); // gold
    palette[5] = Rgb::new(46, 204, 113).to_rgb15(); // green
    palette[6] = Rgb::new(160, 174, 192).to_rgb15(); // dim
    palette[7] = Rgb::new(224, 32, 96).to_rgb15(); // icon red
    palette[8] = Rgb::new(240, 128, 176).to_rgb15(); // icon pink
    palette[9] = Rgb::new(112, 64, 144).to_rgb15(); // icon purple
    palette[10] = Rgb::new(128, 192, 0).to_rgb15(); // icon lime
    palette[11] = Rgb::new(232, 224, 0).to_rgb15(); // icon yellow
    palette[12] = Rgb::new(0, 168, 168).to_rgb15(); // icon teal
    palette[13] = Rgb::new(48, 160, 224).to_rgb15(); // icon sky
    palette[14] = Rgb::new(184, 120, 48).to_rgb15(); // icon brown
    palette[15] = Rgb::new(128, 128, 128).to_rgb15(); // icon gray
    Palette16::new(palette)
};

/// Text palette for the card-detail view, on bank 15 (reserved from the art's
/// 240-colour palette so the two never collide). Matches 3DS COL_CARD_OPAQUE
/// dark panel + white text. Entries 7..=15 mirror TEXT_PALETTE so baked
/// texticon tiles render with the same colours in both banks.
static DETAIL_TEXT_PALETTE: Palette16 = const {
    let mut palette = [Rgb15::BLACK; 16];
    palette[0] = Rgb15::BLACK;
    palette[1] = Rgb15::WHITE;
    palette[2] = Rgb::new(26, 35, 50).to_rgb15(); // zone fill behind text
    palette[6] = Rgb::new(160, 174, 192).to_rgb15(); // dim
    palette[7] = Rgb::new(224, 32, 96).to_rgb15(); // icon red
    palette[8] = Rgb::new(240, 128, 176).to_rgb15(); // icon pink
    palette[9] = Rgb::new(112, 64, 144).to_rgb15(); // icon purple
    palette[10] = Rgb::new(128, 192, 0).to_rgb15(); // icon lime
    palette[11] = Rgb::new(232, 224, 0).to_rgb15(); // icon yellow
    palette[12] = Rgb::new(0, 168, 168).to_rgb15(); // icon teal
    palette[13] = Rgb::new(48, 160, 224).to_rgb15(); // icon sky
    palette[14] = Rgb::new(184, 120, 48).to_rgb15(); // icon brown
    palette[15] = Rgb::new(128, 128, 128).to_rgb15(); // icon gray
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
            pending_art: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.pending_art.clear();
    }

    pub fn println(&mut self, text: &str) {
        self.buf.push_str(text);
        self.buf.push('\n');
    }

    /// Set a background palette bank (0-14 for 4bpp, 15 for text).
    pub fn set_background_palette(&mut self, bank: usize, palette: &Palette16) {
        self.gfx.set_background_palette(bank as u8, palette);
    }

    /// Get the graphics frame for custom rendering.
    pub fn graphics(&mut self) -> &mut Graphics<'a> {
        &mut self.gfx
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

    /// Look up a `{{name|...}}` token's icon: (start tile index, width in
    /// tiles). Names are stored without the `.png` suffix. Each baked cell is
    /// 16px = 2 tiles wide, so tile width = cells * 2.
    fn icon(name: &str) -> Option<(u16, i32)> {
        TEXTICON_GLYPHS
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|&(_, t, cells)| (t, cells as i32 * 2))
    }

    /// Find the next `{{name.png|label}}` token at or after byte `from`.
    /// Returns (token_start, token_end, name-without-.png).
    fn find_token<'t>(text: &'t str, from: usize) -> Option<(usize, usize, &'t str)> {
        let open = text[from..].find("{{")? + from;
        let close = text[open + 2..].find("}}")? + open + 2;
        let inner = &text[open + 2..close];
        let name = inner.split('|').next().unwrap_or(inner);
        let name = name.strip_suffix(".png").unwrap_or(name);
        Some((open, close + 2, name))
    }

    /// Place a baked texticon (cells of 2x2 tiles) at (tx, ty).
    fn place_icon(
        bg: &mut RegularBackground,
        ts: &TileSet,
        e: TileEffect,
        tile: u16,
        cells: i32,
        tx: i32,
        ty: i32,
    ) {
        for c in 0..cells {
            let idx = tile + (c * 4) as u16;
            bg.set_tile((tx + c * 2, ty), ts, TileSetting::new(idx, e));
            bg.set_tile((tx + c * 2 + 1, ty), ts, TileSetting::new(idx + 1, e));
            bg.set_tile((tx + c * 2, ty + 1), ts, TileSetting::new(idx + 2, e));
            bg.set_tile((tx + c * 2 + 1, ty + 1), ts, TileSetting::new(idx + 3, e));
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

    /// Render one line of 16px text at (tx0, ty), clipped to COLS. When
    /// `wrap` is set, an overlong line continues on the next text row (used
    /// by the free-form `swap_buffers` screen); otherwise trailing glyphs
    /// past COLS are clipped (board bars / fixed panes).
    ///
    /// `{{name.png|label}}` tokens render as their baked texticon tiles
    /// inline (mirroring the 3DS `render_text_with_icons`); unknown names
    /// fall back to drawing the raw token text. Icons come from a separate
    /// 4bpp tileset (`TEXTICON_TILES`) sharing the same palette bank.
    fn blit_text(
        bg: &mut RegularBackground,
        font_ts: &TileSet,
        icon_ts: &TileSet,
        e: TileEffect,
        text: &str,
        mut tx: i32,
        mut ty: i32,
        wrap: bool,
    ) {
        let draw_ch = |bg: &mut RegularBackground, ch: char, tx: i32, ty: i32| -> i32 {
            let (_, cols) = Self::glyph(ch);
            if tx + cols as i32 > COLS {
                if !wrap {
                    return cols as i32; // clipped: consume without drawing
                }
                return -(cols as i32); // signal wrap needed
            }
            Self::place_glyph(bg, font_ts, e, ch, tx, ty)
        };
        let mut pos = 0usize;
        while pos < text.len() {
            match Self::find_token(text, pos) {
                None => {
                    for ch in text[pos..].chars() {
                        let w = draw_ch(bg, ch, tx, ty);
                        if w < 0 {
                            tx = 0;
                            ty += 2;
                            tx += draw_ch(bg, ch, tx, ty);
                        } else {
                            tx += w;
                        }
                    }
                    break;
                }
                Some((s, e_, name)) => {
                    for ch in text[pos..s].chars() {
                        let w = draw_ch(bg, ch, tx, ty);
                        if w < 0 {
                            tx = 0;
                            ty += 2;
                            tx += draw_ch(bg, ch, tx, ty);
                        } else {
                            tx += w;
                        }
                    }
                    match Self::icon(name) {
                        Some((tile, w)) if tx + w <= COLS => {
                            Self::place_icon(bg, icon_ts, e, tile, w / 2, tx, ty);
                            tx += w;
                        }
                        _ => {
                            // Unknown icon: draw the raw token text.
                            for ch in text[s..e_].chars() {
                                let w = draw_ch(bg, ch, tx, ty);
                                if w < 0 {
                                    tx = 0;
                                    ty += 2;
                                    tx += draw_ch(bg, ch, tx, ty);
                                } else {
                                    tx += w;
                                }
                            }
                        }
                    }
                    pos = e_;
                }
            }
        }
    }

    /// Render one line of 16px text at (tx0, ty), clipped to COLS.
    fn blit_line(
        bg: &mut RegularBackground,
        font_ts: &TileSet,
        icon_ts: &TileSet,
        e: TileEffect,
        text: &str,
        tx0: i32,
        ty: i32,
    ) {
        Self::blit_text(bg, font_ts, icon_ts, e, text, tx0, ty, false);
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
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
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
        Self::blit_line(&mut ui_bg, &font_ts, &icon_ts, e0, &frame.header, 0, 0);
        Self::blit_line(&mut ui_bg, &font_ts, &icon_ts, e0, &frame.action_count, COLS - 6, 0);

        // Stage rows: opponent (flipped 180°) then player, with live success + live set stacked on right
        for (row, y) in STAGE_YS.iter().enumerate() {
            let y = *y;
            let is_opp = row == 0;
            let stage = if is_opp { &frame.p2_stage } else { &frame.p1_stage };
            let live = if is_opp { &frame.p2_live } else { &frame.p1_live };
            let live_set = if is_opp { &frame.p2_live_set } else { &frame.p1_live_set };
            let flipped = is_opp;
            let e_stage = if flipped { TileEffect::new(true, true, 15) } else { e0 };
            for (i, slot) in stage.iter().enumerate() {
                let xi = if is_opp { 2 - i } else { i };
                let x = 1 + STAGE_PITCH * xi as i32;
                draw_slot(
                        &mut art_bg,
                        &mut ui_bg,
                        &ui_ts,
                        &font_ts,
                        &icon_ts,
                        e_stage,
                        slot,
                        x,
                        y,
                        STAGE_CARD,
                        STAGE_FRONTS,
                        WAITED_FRONTS,
                        flipped,
                    );
            }
            // Live/success zone (top 3 rows of stage row)
            for (i, slot) in live.iter().enumerate() {
                let xi = if is_opp { 2 - i } else { i };
                let x = INFO_X + LIVE_PITCH * xi as i32;
                draw_slot(
                    &mut art_bg,
                    &mut ui_bg,
                    &ui_ts,
                    &font_ts,
                    &icon_ts,
                    e_stage,
                    slot,
                    x,
                    y,
                    LIVE_CARD,
                    LIVE_FRONTS,
                    WAITED_FRONTS,
                    flipped,
                );
            }
            // Live card set zone (bottom 3 rows of stage row) — flipped like 3DS landscape handling
            for (i, slot) in live_set.iter().enumerate() {
                let xi = if is_opp { 2 - i } else { i };
                let x = INFO_X + LIVE_PITCH * xi as i32;
                draw_slot(
                    &mut art_bg,
                    &mut ui_bg,
                    &ui_ts,
                    &font_ts,
                    &icon_ts,
                    e_stage,
                    slot,
                    x,
                    y + 3,
                    LIVE_CARD,
                    LIVE_FRONTS,
                    WAITED_FRONTS,
                    flipped,
                );
            }
        }

        // Hand window; a gold badge marks more cards off-screen right.
        for (i, slot) in frame.hand.iter().enumerate() {
            let x = HAND_PITCH * i as i32;
            draw_slot(
                &mut art_bg,
                &mut ui_bg,
                &ui_ts,
                &font_ts,
                &icon_ts,
                e0,
                slot,
                x,
                HAND_Y,
                HAND_CARD,
                CARD_FRONTS,
                WAITED_FRONTS,
                false,
            );
        }
        if frame.hand_more {
            ui_bg.set_tile(
                (COLS - 1, HAND_Y),
                &ui_ts,
                TileSetting::new(UI_BADGE, e0),
            );
        }

        // Cursor: hand or stage (depending on L-cycled focus). Marker is white triangle.
        if let Some(w) = frame.hand_cursor {
            let x = HAND_PITCH * w as i32;
            ui_bg.set_tile((x, HAND_Y), &ui_ts, TileSetting::new(UI_MARKER, e0));
        }
        if let Some(idx) = frame.own_stage_cursor {
            let x = STAGE_START_X + STAGE_PITCH * idx as i32;
            ui_bg.set_tile((x, STAGE_YS[1]), &ui_ts, TileSetting::new(UI_MARKER, e0));
        }
        if let Some(idx) = frame.opp_stage_cursor {
            // Opponent cards are drawn mirrored (2 - i, like the 3DS far
            // side), so the cursor must mirror too or it points at the
            // wrong card.
            let x = STAGE_START_X + STAGE_PITCH * (2 - idx) as i32;
            ui_bg.set_tile((x, STAGE_YS[0]), &ui_ts, TileSetting::new(UI_MARKER, e0));
        }

        // Action bar pinned to the bottom (single 16px line; hint lives in header).
        let bar = alloc::format!("> {}", frame.action_line);
        Self::blit_line(&mut ui_bg, &font_ts, &icon_ts, e0, &bar, 0, BAR_Y);

        let mut f = self.gfx.frame();
        art_bg.show(&mut f);
        ui_bg.show(&mut f);
        f.commit();
    }

    /// Render the full-screen Actions view: the buffered action list with a
    /// small hint line at the top. Input stays with the engine so Up/Down/A
    /// drive the list live. Mirrors 3DS bottom screen (VISUAL_DESIGN.md:88-127):
    /// color-coded action types, scroll indicators with exact counts.
    pub fn render_action_text(&mut self) {
        self.last = self.buf.clone();
        self.gfx.set_background_palette(15, &TEXT_PALETTE);
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let e = TileEffect::new(false, false, 15);

        let mut tbg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );

        Self::blit_line(&mut tbg, &font_ts, &icon_ts, e, "ACTIONS [Sel:Board] [Sta:Menu]", 0, 0);
        let mut row = 2i32;
        for line in self.buf.split('\n') {
            if row + 2 > ROWS {
                break;
            }
            // Color-code action lines like 3DS: gold for play actions,
            // green for confirm/pass, dim for scroll indicators
            let color = if line.contains("Pass") || line.contains("Confirm") {
                TileEffect::new(false, false, 15) // white via bank 15
            } else if line.starts_with("  ..") {
                e
            } else {
                e
            };
            Self::blit_line(&mut tbg, &font_ts, &icon_ts, color, line, 0, row);
            row += 2;
        }

        let mut f = self.gfx.frame();
        tbg.show(&mut f);
        f.commit();
    }

    /// Render a card-detail view: 8bpp art on a P0 background (indices 0-239)
    /// with 4bpp text on P1 using palette bank 15 (indices 240-255).
    /// The 12x18 portrait fills the height left of the text pane (centered
    /// with one row top and bottom); text keeps its size in the remaining
    /// 17 columns. `scroll` is the first visible line index — paginated like
    /// 3DS detail (render.rs:437 lpp 10) to avoid VRAM blow-up with long
    /// ability text.
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
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let e_text = TileEffect::new(false, false, 15);

        let mut f = self.gfx.frame();

        if let Some(art) = art {
            // Baked 12x18 tiles address the portrait's own static bytes, so
            // every card uploads its own (pointer, tile) pairs — always the
            // right pixels, never stale, and freed on close.
            let art_ts = unsafe { TileSet::new(art.tiles, TileFormat::EightBpp) };
            let mut abg = RegularBackground::new(
                Priority::P0,
                RegularBackgroundSize::Background32x32,
                TileFormat::EightBpp,
            );
            for i in 0..(DETAIL_DW * DETAIL_DH) {
                let tx = (i % DETAIL_DW) as i32;
                let ty = (i / DETAIL_DW) as i32 + DETAIL_Y0;
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
            for tx in DETAIL_DW as i32..COLS {
                tbg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e_text));
            }
        }
        const VISIBLE: usize = 8;
        let end = (scroll + VISIBLE).min(lines.len());
        for (i, line) in lines[scroll..end].iter().enumerate() {
            Self::blit_line(&mut tbg, &font_ts, &icon_ts, e_text, line, DETAIL_DW as i32 + 1, i as i32 * 2);
        }
        // Scroll indicators like 3DS detail (render.rs:568)
        if scroll > 0 {
            Self::blit_line(&mut tbg, &font_ts, &icon_ts, e_text, "^", 29, 0);
        }
        if end < lines.len() {
            Self::blit_line(&mut tbg, &font_ts, &icon_ts, e_text, "v", 29, 18);
        }
        tbg.show(&mut f);
        f.commit();
    }

    /// Reset VRAM tile pressure: commit an empty frame so the previous
    /// screen's tiles (already unreferenced) are garbage-collected before
    /// the next screen allocates.
    ///
    /// agb frees dead tiles only at commit, so without this the dead
    /// previous screen and the new screen briefly coexist — a 280-tile
    /// portrait plus a 150-tile choice grid plus CJK text blows the ~500
    /// 8bpp-tile budget ("Ran out of video RAM for tiles"). Call on heavy
    /// screen transitions (detail open/close, choice open/close); the one
    /// blank frame reads as a transition flicker.
    pub fn reset_vram(&mut self) {
        let f = self.gfx.frame();
        f.commit();
    }
    /// Queue a card image for the next [`Display::swap_buffers`]. Called by
    /// the `PlatformUi::draw_card_image` impl so generic choice menus show
    /// real card fronts (3DS-style) instead of a text-only list. Coordinates
    /// are in tiles; `selected` draws a gold badge over the top-right corner.
    pub fn queue_card_image(
        &mut self,
        card_no: &str,
        x: i32,
        y: i32,
        cols: i32,
        rows: i32,
        selected: bool,
    ) {
        use alloc::string::ToString;
        self.pending_art.push(PendingArt {
            card_no: card_no.to_string(),
            x,
            y,
            cols,
            rows,
            selected,
        });
        // The early-out in swap_buffers compares text only; queued art must
        // force a redraw even when the text is unchanged (cursor moves swap
        // the highlight while the buffered lines stay identical).
        self.last.clear();
    }

    pub fn swap_buffers(&mut self) {
        if self.buf == self.last && self.pending_art.is_empty() {
            return;
        }
        self.last = self.buf.clone();

        // Bank 15 holds TEXT_PALETTE (white text on dark blue). Bank 0
        // overlaps the 256-colour card-art master palette, so text drawn
        // with bank 0 picks up art colours (the old orange-menu bug).
        // The detail view also overwrites bank 15, so restore it here.
        self.gfx.set_background_palette(15, &TEXT_PALETTE);
        if self.detail_active {
            // Detail view overwrote 0-239 with per-card palette — restore master
            for i in 0..240 {
                let v = MASTER_PAL[i * 2] as u16 | ((MASTER_PAL[i * 2 + 1] as u16) << 8);
                self.gfx.set_background_palette_colour_256(i, Rgb15::new(v));
            }
            self.detail_active = false;
        }
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let mut bg = RegularBackground::new(
            Priority::P0,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );

        // Fill entire screen with zone fill color (UI_EMPTY, palette index 2 = dark blue)
        // so menus have the same dark blue background as the board.
        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
        let e_ui = TileEffect::new(false, false, 15);
        for ty in 0..ROWS {
            for tx in 0..COLS {
                bg.set_tile((tx, ty), &ui_ts, TileSetting::new(UI_EMPTY, e_ui));
            }
        }

        let e = TileEffect::new(false, false, 15);
        let mut ty = 0i32;

        for line in self.buf.split('\n') {
            if ty + 2 > ROWS {
                break;
            }
            Self::blit_text(&mut bg, &font_ts, &icon_ts, e, line, 0, ty, true);
            ty += 2;
        }

        if self.pending_art.is_empty() {
            let mut frame = self.gfx.frame();
            bg.show(&mut frame);
            frame.commit();
            return;
        }

        // Composite queued card art underneath the text BG: blit each front
        // onto an 8bpp BG and punch transparent holes in the text BG so the
        // art shows through (same layering as the board renderer). Corner
        // ticks mark the selected card.
        let mut art_bg = RegularBackground::new(
            Priority::P1,
            RegularBackgroundSize::Background32x32,
            TileFormat::EightBpp,
        );
        let art_eff = TileEffect::new(false, false, 0);
        for q in self.pending_art.drain(..) {
            // Stage-size requests (5x6, the choice grid) use the stage
            // fronts; anything else uses the hand-size fronts.
            let fronts = if q.cols == 5 && q.rows == 6 {
                STAGE_FRONTS
            } else {
                CARD_FRONTS
            };
            if q.card_no.is_empty() {
                // Marker slot with no art: badge so the cursor stays visible.
                if q.selected {
                    bg.set_tile((q.x, q.y), &ui_ts, TileSetting::new(UI_BADGE, e_ui));
                }
            } else if let Some(front) = fronts
                .iter()
                .find(|f| f.card_no == q.card_no.as_str())
            {
                let ts = unsafe { TileSet::new(front.tiles, TileFormat::EightBpp) };
                for ay in 0..q.rows {
                    for ax in 0..q.cols {
                        let sidx = (ay * q.cols + ax) as u16;
                        art_bg.set_tile(
                            (q.x + ax, q.y + ay),
                            &ts,
                            TileSetting::new(sidx, art_eff),
                        );
                        bg.set_tile((q.x + ax, q.y + ay), &ui_ts, TileSetting::BLANK);
                    }
                }
                if q.selected {
                    // Gold badge overlapping the card's top-right corner.
                    bg.set_tile(
                        (q.x + q.cols - 1, q.y),
                        &ui_ts,
                        TileSetting::new(UI_BADGE, e_ui),
                    );
                }
            } else {
                // Unknown card number: leave the zone fill and print the id.
                Self::blit_line(&mut bg, &font_ts, &icon_ts, e, &q.card_no, q.x, q.y);
            }
        }

        let mut frame = self.gfx.frame();
        art_bg.show(&mut frame);
        bg.show(&mut frame);
        frame.commit();
    }

    pub fn wait(&mut self) {
        busy_wait_for_vblank();
    }
}

/// Detail portrait grid: 12x18 tiles (96x144px) at rows 1-18, centered
/// vertically next to the text pane.
const DETAIL_DW: usize = 12;
const DETAIL_DH: usize = 18;
/// First tile row of the portrait (18 tall on a 20-row screen).
const DETAIL_Y0: i32 = 1;

/// Draw one card slot: 8bpp shared-palette front art on the art BG (or a
/// solid gray slot on the text BG), plus the gold badge in the right gap
/// column when the card has valid actions.
fn draw_slot(
    art_bg: &mut RegularBackground,
    ui_bg: &mut RegularBackground,
    ui_ts: &TileSet,
    font_ts: &TileSet,
    icon_ts: &TileSet,
    e0: TileEffect,
    slot: &crate::board::Slot,
    x: i32,
    y: i32,
    card: (i32, i32),
    fronts: &[crate::card_art_gen::CardFront],
    waited_fronts: &[crate::card_art_gen::CardFront],
    flipped: bool,
    ) {
    let (cols, rows) = if slot.waited {
        (4, 3) // wait grid: 4x3 tiles = 32x24
    } else {
        card
    };
    let fronts = if slot.waited { waited_fronts } else { fronts };
    let empty = |bg: &mut RegularBackground| {
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
                let art_eff = |fl: bool| if fl { TileEffect::new(true, true, 0) } else { TileEffect::new(false, false, 0) };
                for ty in 0..rows {
                    for tx in 0..cols {
                        let sidx = if flipped { (rows - 1 - ty) * cols + (cols - 1 - tx) } else { ty * cols + tx } as u16;
                        art_bg.set_tile(
                            (x + tx, y + ty),
                            &ts,
                            TileSetting::new(sidx, art_eff(flipped)),
                        );
                        // Clear ui_bg so card art shows through (ui is in front)
                        ui_bg.set_tile((x + tx, y + ty), &ui_ts, TileSetting::BLANK);
                    }
                }
            }
            None => {
                empty(ui_bg);
                Display::blit_line(ui_bg, font_ts, icon_ts, e0, card_no, x, y + rows / 2);
            }
        },
        None => empty(ui_bg),
    }
    if slot.actionable {
        // Gold badge on the card itself (top-right corner, or bottom-left when flipped 180°)
        let (bx, by) = if flipped { (x, y + rows - 1) } else { (x + cols - 1, y) };
        ui_bg.set_tile((bx, by), ui_ts, TileSetting::new(UI_BADGE, e0));
    }
}

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;

use agb::display::tiled::{
    RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
};
use agb::display::object::{
    DynamicSprite256, Object, PaletteMulti, PaletteVramMulti, Size, SpriteVram,
};
use agb::display::{busy_wait_for_vblank, Graphics, Palette16, Priority, Rgb15, Rgb};

use crate::board::{BoardFrame, Slot};
use crate::card_art_gen::{CardArt, CardFront, BACK_FRONT, BOARD_UI, CARD_FRONTS, LIVE_FRONTS, MASTER_PAL, STAGE_FRONTS, WAITED_FRONTS};
use crate::font_tiles_gen::{FONT_GLYPHS, FONT_TILES};
use crate::texticons_gen::{TEXTICON_GLYPHS, TEXTICON_TILES};

/// Screen is 240x160 = 30 cols x 20 rows of 8px tiles. Text glyphs are
/// 16x16 (2x2 tiles, 12px font) — each text line is 2 tile-rows.
pub const COLS: i32 = 30;
pub const ROWS: i32 = 20;
const FONT_ROWS: i32 = 2;

/// Card pixel grids baked in `tools/bake_card_art.py` (multiples of 8px).
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

/// Stage pitch is one tile wider than the 40px art: the 48px box leaves a
/// 4px gap each side, so tapped (wait-state) cards rotated 90° to 48px wide
/// exactly fill their box without touching neighbours — like the 3DS square
/// stage slots. Even all-waited rows never overlap.
const STAGE_PITCH: i32 = STAGE_CARD.0 + 1; // 6 tiles = 48px box
const HAND_PITCH: i32 = HAND_CARD.0;
const STAGE_START_X: i32 = 1;
const LIVE_CARD: (i32, i32) = (3, 2); // 24x16 landscape (live cards are wide)
const LIVE_PITCH: i32 = LIVE_CARD.0; // badge on card
#[allow(dead_code)]
const HAND_START_X: i32 = 0;
const STAGE_YS: [i32; 2] = [HEADER_H, HEADER_H + STAGE_H]; // [2, 8]
const HAND_Y: i32 = STAGE_YS[1] + STAGE_H; // 14
const BAR_Y: i32 = ROWS - BAR_H; // 18
const INFO_X: i32 = STAGE_START_X + STAGE_PITCH * 3 + 1; // 20 with pitch 6

/// Hand capacity derived from screen width, not hardcoded (30 cols / 3 pitch = 10).
#[allow(dead_code)]
pub const HAND_FITS: usize = (COLS / HAND_PITCH) as usize; // 10

/// Board UI tile indices inside [`BOARD_UI`] (4bpp text BG, bank 15).
const UI_EMPTY: u16 = 0; // solid zone fill (opaque)
const UI_BADGE: u16 = 1; // gold actionable diamond
const UI_MARKER: u16 = 2; // white focus triangle
#[allow(dead_code)]
const UI_GOLD: u16 = 3; // solid gold for cursor border
const UI_TRANS: u16 = 4; // fully transparent (front-BG clear)
const UI_BADGE_EDGE: u16 = 5; // gold diamond nudged right, for badges placed
                              // one tile left of the grid edge (on the card's
                              // visible border instead of the padding)

/// Layering (GBA: lower number wins; OBJ beats a BG of the SAME number, so
/// every layer gets its own number):
/// front text/badges P0 > card sprites P1 > back zone fill P2.
const SPRITE_PRIO: Priority = Priority::P1;

/// Card pixel boxes (on-screen footprint; art is centred inside).
/// Stage is 48px wide (not 40): the gap fits tapped cards rotated 90°.
const HAND_PX: (usize, usize) = (24, 32);
const STAGE_PX: (usize, usize) = (48, 48);
const LIVE_PX: (usize, usize) = (24, 16);
/// Waited (tapped) art grid: 4x3 tiles = 32x24px, centred in the slot box.
const WAIT_GRID: (usize, usize) = (4, 3);
const DETAIL_PX: (usize, usize) = (96, 144);

/// Sprite tilings per card box: (hardware size, x-offset, y-offset) in px.
/// Every entry is a legal GBA sprite size; gutters (sprite area outside the
/// card box) are filled with palette index 0 = transparent.
const HAND_PARTS: &[(Size, usize, usize)] =
    &[(Size::S16x32, 0, 0), (Size::S16x32, 16, 0)];
const STAGE_PARTS: &[(Size, usize, usize)] = &[
    (Size::S32x32, 0, 0),
    (Size::S16x32, 32, 0),
    (Size::S32x16, 0, 32),
    (Size::S16x16, 32, 32),
];
const LIVE_PARTS: &[(Size, usize, usize)] = &[(Size::S32x16, 0, 0)];
// NOTE: waited art (32x24) is centred inside the slot's own box tiling via
// ox/oy, so it needs no dedicated parts table.
const DETAIL_PARTS: &[(Size, usize, usize)] = &[
    (Size::S32x64, 0, 0),
    (Size::S32x64, 32, 0),
    (Size::S32x64, 64, 0),
    (Size::S32x64, 0, 64),
    (Size::S32x64, 32, 64),
    (Size::S32x64, 64, 64),
    (Size::S32x16, 0, 128),
    (Size::S32x16, 32, 128),
    (Size::S32x16, 64, 128),
];

/// Upper bound on cached card placements. Board steady state is ~28 keys;
/// anything far above that is a screen-transition leak, so drop everything
/// and re-upload (one hitch beats an OBJ-VRAM OOM panic).
const SPRITE_CACHE_CAP: usize = 96;

/// Full-screen text + sprite-composited cards.
///
/// * `ui_back` (P2): opaque zone fill, always behind cards.
/// * card sprites (P1): all card fronts/backs, multi-Object per card.
/// * `ui_front` (P0): text, badges, cursors — transparent elsewhere so
///   cards show through. No punching needed: it is cleared with UI_TRANS.
pub struct Display<'a> {
    gfx: Graphics<'a>,
    buf: String,
    last: String,
    /// Card art queued via [`Display::queue_card_image`] since the last
    /// [`Display::clear`]. `swap_buffers` composites these as sprites
    /// between the two text BGs, so generic choice menus show card images.
    pending_art: Vec<PendingArt>,
    /// Opaque zone-fill background (behind sprites).
    ui_back: RegularBackground,
    /// Transparent text/badge/cursor background (in front of sprites).
    ui_front: RegularBackground,
    /// Sprite cache for all card art (shared master palette).
    sprite_cache: CardSpriteCache,
    /// Active card sprites this frame (shown back-to-front deterministically).
    active_sprites: Vec<Object>,
    /// Scroll offset of the Actions view, following the `>` cursor so long
    /// action lists scroll fully instead of clipping past 9 rows.
    action_offset: usize,
}

/// One queued card image for the next [`Display::swap_buffers`].
struct PendingArt {
    card_no: String,
    x: i32,
    y: i32,
    cols: i32,
    rows: i32,
    selected: bool,
    dimmed: bool,
}

/// Cached SpriteVram for card fronts, keyed by
/// `{tag}:{card_no}:{box}x{box}:{flip}:{dim}`. Each entry holds all parts of
/// one card placement, aligned with the corresponding `*_PARTS` table.
/// Reuses VRAM allocations across frames.
///
/// OBJ VRAM is only 32KB (agb `SPRITE_ALLOCATOR`) and a full board already
/// needs ~30KB, so a rotated/duplicate entry (e.g. a waited card alongside
/// its unrotated self) does NOT fit — its upload fails and the card would
/// vanish instead of rotating. Two rules keep the budget exact:
/// - `begin_frame`/`end_frame` retain only placements shown this frame;
///   stale variants die at the frame edge, never accumulate.
/// - callers pass `evict` keys replaced by the new upload (waited stage art
///   evicts its unrotated twin first), so transitions never exceed steady
///   state even mid-frame.
/// One cached card placement: all parts plus the frame stamp for the
/// end-of-frame retention sweep.
struct CachedPlacement {
    vrams: Vec<SpriteVram>,
    last_used: u32,
}

struct CardSpriteCache {
    sprites: BTreeMap<String, CachedPlacement>,
    palette: PaletteVramMulti,
    /// Current frame id. Entries stamp it on every touch (hit or upload);
    /// `end_frame` drops everything older.
    frame: u32,
    /// Whether anything was pushed this frame (gates `end_frame`).
    touched: bool,
    /// Reused key buffers: lookups format into these instead of allocating
    /// a fresh String per card per frame (steady 60fps churn that fragments
    /// the 256KB heap over a long match). Inserts clone out of the buffer,
    /// but those happen only on cache misses (screen transitions).
    key_buf: String,
    twin_buf: String,
}

impl CardSpriteCache {
    fn new(_gfx: &mut Graphics) -> Self {
        // Build 256-colour master palette from baked MASTER_PAL.
        // Index 0 is forced magenta at bake time and is the OBJ
        // transparent index, so no art pixel may use it (verified: the bake
        // emits zero index-0 pixels; gutters are filled with 0 here).
        // PaletteMulti is 16 Palette16 entries (16*16=256 colours).
        // Use Box::leak to get 'static lifetime for PaletteMulti::new.
        log::debug!(
            "sprite palette master[0]={:02x}{:02x} (expect magenta 1f7c)",
            MASTER_PAL[0],
            MASTER_PAL[1]
        );
        let palettes: Box<[Palette16; 16]> = Box::new(core::array::from_fn(|_| Palette16::new([Rgb15::BLACK; 16])));
        let mut palettes = palettes;
        for i in 0..240 {
            let v = MASTER_PAL[i * 2] as u16 | ((MASTER_PAL[i * 2 + 1] as u16) << 8);
            let palette_idx = i / 16;
            let colour_idx = i % 16;
            palettes[palette_idx].update_colour(colour_idx, Rgb15::new(v));
        }
        let palette_multi = Box::leak(Box::new(PaletteMulti::new(Box::leak(palettes))));
        let palette = PaletteVramMulti::new(palette_multi);
        let mut key_buf = String::new();
        key_buf.reserve(64);
        let mut twin_buf = String::new();
        twin_buf.reserve(64);
        Self {
            sprites: BTreeMap::new(),
            palette,
            frame: 1,
            touched: false,
            key_buf,
            twin_buf,
        }
    }

    /// Start a frame. The id wraps after ~2 years at 60fps; on wrap the
    /// whole cache is dropped once so no stale stamp can survive.
    fn begin_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.frame == 0 {
            self.sprites.clear();
            self.frame = 1;
        }
        self.touched = false;
    }

    /// End a frame: drop placements nothing showed (stale flips, dims,
    /// rotations, last screen's cards). Their VRAM frees with the last ref.
    /// Skipped when nothing was pushed (text-only menu over a live board
    /// overlay): retaining empty would wipe a cache the next frame still
    /// needs, buying a full re-upload hitch for nothing.
    fn end_frame(&mut self) {
        if !self.touched {
            return;
        }
        let frame = self.frame;
        let before = self.sprites.len();
        self.sprites.retain(|_, p| p.last_used == frame);
        if self.sprites.len() != before {
            log::debug!(
                "sprite retain: {} -> {} keys",
                before,
                self.sprites.len()
            );
        }
    }

    /// Upload one sprite part: sample the baked tile grid (centred in the
    /// card box, optionally rotated 180°, optionally 90° CW for the
    /// wait-state, and dither-dimmed) via `set_pixel` so rectangular sizes
    /// use the correct OBJ 1D-mapping layout.
    /// Gutter pixels (sprite area outside the card box) stay index 0.
    #[allow(clippy::too_many_arguments)]
    fn upload_part(
        tiles: &[u8],
        grid_w: usize,
        grid_h: usize,
        box_w: usize,
        box_h: usize,
        flipped: bool,
        dimmed: bool,
        rot_cw: bool,
        size: Size,
        dx: usize,
        dy: usize,
        palette: &PaletteVramMulti,
    ) -> Option<SpriteVram> {
        let art_w = grid_w * 8;
        let art_h = grid_h * 8;
        // Rotated (wait-state) footprint: dimensions swap, like the 3DS
        // tapped path which rotates to fill the slot.
        let (rw, rh) = if rot_cw { (art_h, art_w) } else { (art_w, art_h) };
        // Centre the art inside the card box. Negative only if art overflows
        // the box, in which case the outer pixels clip to transparent.
        let ox = (box_w as i32 - rw as i32) / 2;
        let oy = (box_h as i32 - rh as i32) / 2;
        let (sw, sh) = size.to_width_height();
        let mut sprite = DynamicSprite256::new(size);
        // Gutters (sprite area outside the card box) must be index 0 =
        // transparent. clear() it explicitly: `new` does not guarantee
        // zero-fill, and stale IWRAM here reads as confetti.
        sprite.clear(0);
        for sy in 0..sh {
            for sx in 0..sw {
                let bx = dx + sx;
                let by = dy + sy;
                // Outside the card box: transparent gutter.
                if bx >= box_w || by >= box_h {
                    continue;
                }
                // Box pixel -> art pixel (art is centred in the box via ox/oy).
                // 180° rotation mirrors in BOX space first, then offsets
                // into art space, so centred art stays centred.
                let (fx, fy) = if flipped {
                    (box_w as i32 - 1 - bx as i32, box_h as i32 - 1 - by as i32)
                } else {
                    (bx as i32, by as i32)
                };
                // 90° CW (wait-state, like 3DS tapped): art top-left lands
                // at box top-right, i.e. (ax,ay) -> (H-1-ay, ax).
                let (ax, ay) = if rot_cw {
                    (fy - oy, art_h as i32 - 1 - (fx - ox))
                } else {
                    (fx - ox, fy - oy)
                };
                if ax < 0 || ay < 0 || ax >= art_w as i32 || ay >= art_h as i32 {
                    continue;
                }
                let tx = ax as usize / 8;
                let ty = ay as usize / 8;
                let px = ax as usize % 8;
                let py = ay as usize % 8;
                let idx = (ty * grid_w + tx) * 64 + py * 8 + px;
                let v = tiles.get(idx).copied().unwrap_or(0);
                if v == 0 {
                    // Index 0 is transparent on OBJ; baked art never uses it
                    // (bake forces magenta-0), so treat as gutter.
                    continue;
                }
                if dimmed && (bx + by) % 2 == 0 {
                    // Unpickable overlay: checkerboard dither to transparent
                    // so the dark zone fill shows through (mirrors the 3DS
                    // `disabled` overlay at zero ROM cost).
                    continue;
                }
                sprite.set_pixel(sx, sy, v);
            }
        }
        // Only non-zero art pixels were written over the cleared buffer,
        // so gutters stay transparent.
        match sprite.try_to_vram(palette.clone()) {
            Ok(vram) => Some(vram),
            Err(_) => {
                log::debug!("sprite VRAM full, will evict and retry");
                None
            }
        }
    }

    /// Get (or upload) all parts of one card placement. `twin_tag` names the
    /// mutually-exclusive transform of the same art (board `stage` <->
    /// `stagew`): on a miss the twin is dropped FIRST, since there is no
    /// room to hold both twins.
    ///
    /// Allocation discipline: cache HITS allocate nothing — the key formats
    /// into a reused buffer and looks up as `&str`. Only misses allocate
    /// (owned key insert + VRAM upload), and those happen on screen
    /// transitions, never in steady frames.
    #[allow(clippy::too_many_arguments)]
    fn get_or_upload(
        &mut self,
        tag: &str,
        card_no: &str,
        tiles: &[u8],
        grid_w: usize,
        grid_h: usize,
        box_w: usize,
        box_h: usize,
        parts: &[(Size, usize, usize)],
        flipped: bool,
        dimmed: bool,
        rot_cw: bool,
        twin_tag: Option<&str>,
    ) -> Option<&[SpriteVram]> {
        use core::fmt::Write as _;
        // Box dims are part of the key: the board stage box (48px, with the
        // wait gap) and the menu stage box (40px) share tag/art but centre
        // differently — aliasing them shifts art by the gap.
        let flip_c = if flipped { 'f' } else { 'n' };
        let dim_c = if dimmed { 'd' } else { 'n' };
        self.touched = true;
        self.key_buf.clear();
        let _ = write!(
            self.key_buf,
            "{}:{}:{}x{}:{}:{}",
            tag, card_no, box_w, box_h, flip_c, dim_c
        );
        if let Some(p) = self.sprites.get_mut(self.key_buf.as_str()) {
            p.last_used = self.frame;
        } else {
            if let Some(tt) = twin_tag {
                self.twin_buf.clear();
                let _ = write!(
                    self.twin_buf,
                    "{}:{}:{}x{}:{}:{}",
                    tt, card_no, box_w, box_h, flip_c, dim_c
                );
                // Miss path only: clone the twin key out to end the buffer
                // borrow before mutating the map.
                let twin: String = self.twin_buf.as_str().into();
                self.sprites.remove(&twin);
            }
            if self.sprites.len() >= SPRITE_CACHE_CAP {
                log::debug!(
                    "sprite cache over cap ({} keys), evicting all",
                    self.sprites.len()
                );
                self.sprites.clear();
            }
            let mut vrams = Vec::with_capacity(parts.len());
            let mut oom = false;
            for &(size, dx, dy) in parts {
                match Self::upload_part(
                    tiles,
                    grid_w,
                    grid_h,
                    box_w,
                    box_h,
                    flipped,
                    dimmed,
                    rot_cw,
                    size,
                    dx,
                    dy,
                    &self.palette,
                ) {
                    Some(v) => vrams.push(v),
                    None => {
                        oom = true;
                        break;
                    }
                }
            }
            // Miss path only: clone the key out to end the buffer borrow.
            let key: String = self.key_buf.as_str().into();
            if oom {
                // OBJ VRAM exhausted (e.g. stale screen still cached):
                // drop everything and retry once in the freed space.
                log::debug!("sprite upload OOM for {}, evicting and retrying", key);
                self.sprites.clear();
                let mut vrams = Vec::with_capacity(parts.len());
                for &(size, dx, dy) in parts {
                    match Self::upload_part(
                        tiles,
                        grid_w,
                        grid_h,
                        box_w,
                        box_h,
                        flipped,
                        dimmed,
                        rot_cw,
                        size,
                        dx,
                        dy,
                        &self.palette,
                    ) {
                        Some(v) => vrams.push(v),
                        None => return None,
                    }
                }
                self.sprites.insert(
                    key,
                    CachedPlacement {
                        vrams,
                        last_used: self.frame,
                    },
                );
            } else {
                self.sprites.insert(
                    key,
                    CachedPlacement {
                        vrams,
                        last_used: self.frame,
                    },
                );
            }
        }
        // Both shared borrows of disjoint fields — no clone needed.
        self.sprites
            .get(self.key_buf.as_str())
            .map(|p| p.vrams.as_slice())
    }

    /// Keep exactly one cached placement, drop everything else. Used by the
    /// detail view: scrolling re-renders every step, and re-uploading the
    /// 9-part portrait each time churns OBJ VRAM (occasional oom -> the
    /// portrait vanishes for that frame). Retaining the current card's key
    /// turns scrolls into pure cache hits with zero VRAM traffic.
    fn retain_key(&mut self, keep: &str) {
        if self.sprites.len() == 1 && self.sprites.contains_key(keep) {
            return;
        }
        log::debug!("retaining sprite key {}", keep);
        self.sprites.retain(|k, _| k == keep);
    }

    /// Drop cached placements whose key starts with `prefix` (e.g. detail
    /// portraits before the board re-uploads into the freed OBJ VRAM).
    fn evict_prefix(&mut self, prefix: &str) {
        let dead: Vec<String> = self
            .sprites
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        if !dead.is_empty() {
            log::debug!("evicting {} sprite keys ({})", dead.len(), prefix);
            for k in dead {
                self.sprites.remove(&k);
            }
        }
    }

    fn clear(&mut self) {
        if !self.sprites.is_empty() {
            log::debug!("clearing {} sprite keys", self.sprites.len());
            self.sprites.clear();
        }
    }
}

/// 4bpp text/board palette (bank 15). Entries 7..=15 are the texticon colours,
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

/// Text palette for the card-detail view, on bank 15 (the OBJ sprite palette
/// lives in a separate hardware block, so the two never collide). Matches
/// 3DS COL_CARD_OPAQUE dark panel + white text. Entries 7..=15 mirror
/// TEXT_PALETTE so baked texticon tiles render the same in both views.
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
        // Bank 15 carries the 4bpp text/UI palette for both BGs. The 256-col
        // card palette lives in OBJ palette RAM via PaletteVramMulti (a
        // separate hardware block), so no per-frame BG palette writes are
        // needed and the backdrop never flashes.
        gfx.set_background_palette(15, &TEXT_PALETTE);
        // Wait one VBlank to ensure palette is active before first frame.
        busy_wait_for_vblank();
        let sprite_cache = CardSpriteCache::new(&mut gfx);
        Display {
            gfx,
            buf: String::new(),
            last: String::new(),
            pending_art: Vec::new(),
            ui_back: RegularBackground::new(
                Priority::P2,
                RegularBackgroundSize::Background32x32,
                TileFormat::FourBpp,
            ),
            ui_front: RegularBackground::new(
                Priority::P0,
                RegularBackgroundSize::Background32x32,
                TileFormat::FourBpp,
            ),
            sprite_cache,
            active_sprites: Vec::new(),
            action_offset: 0,
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

    /// Wrap `text` into lines of at most `cols` tiles, honouring existing
    /// newlines as hard breaks and keeping `{{name.png|label}}` texticon
    /// tokens atomic.
    ///
    /// This MUST measure exactly what [`Display::blit_text`] draws: the
    /// engine's `wrap_text` estimates a token by its bracket-label width
    /// (e.g. `heart_00` = 8 cols) while the GBA draws the baked icon
    /// (1 cell = 2 tiles), so engine-wrapped stat lines shatter onto a
    /// newline per token in the 17-tile detail pane. Chars are measured by
    /// their real glyph advance (`FONT_GLYPHS` cols); unknown icons fall
    /// back to the raw token text width, which is what the blitter draws.
    /// Tile width of one `{{name.png|label}}` token as drawn: the baked
    /// icon width, or the raw token text width when the icon is unknown
    /// (which is what the blitter falls back to drawing).
    fn token_tiles(token: &str) -> usize {
        let inner = token
            .strip_prefix("{{")
            .and_then(|s| s.strip_suffix("}}"))
            .unwrap_or(token);
        let name = inner.split('|').next().unwrap_or(inner);
        let name = name.strip_suffix(".png").unwrap_or(name);
        match Self::icon(name) {
            Some((_, w)) => w as usize,
            None => token.chars().map(|ch| Self::glyph(ch).1 as usize).sum(),
        }
    }

    pub fn wrap_pane(text: &str, cols: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut w = 0usize;
        for piece in text.split('\n') {
            // Split into atomic units: whole tokens or single chars.
            let mut i = 0usize;
            // After a soft wrap, separator spaces belong to the previous
            // line's end — emitting them starts the new line indented (the
            // old blade-count "tab"). Skip them; hard-newline indentation
            // (leading spaces right after '\n') is preserved.
            let mut skip_ws = false;
            while i < piece.len() {
                let (uw, unit) = match Self::find_token(piece, i) {
                    Some((s, e, _)) if s == i => (Self::token_tiles(&piece[s..e]), &piece[s..e]),
                    _ => {
                        let ch = piece[i..].chars().next().unwrap();
                        (Self::glyph(ch).1 as usize, &piece[i..i + ch.len_utf8()])
                    }
                };
                if w + uw > cols && !cur.is_empty() {
                    out.push(core::mem::take(&mut cur));
                    w = 0;
                    skip_ws = true;
                }
                if skip_ws {
                    if unit.trim().is_empty() {
                        i += unit.len();
                        continue;
                    }
                    skip_ws = false;
                }
                cur.push_str(unit);
                w += uw;
                i += unit.len();
            }
            if !cur.is_empty() {
                out.push(core::mem::take(&mut cur));
            }
            w = 0;
        }
        if out.is_empty() {
            out.push(String::new());
        }
        out
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

    /// Clear both BGs: opaque zone fill behind, transparent in front.
    /// Also starts sprite-frame tracking (see `end_frame`).
    fn clear_layers(&mut self, ui_ts: &TileSet, e: TileEffect) {
        for ty in 0..ROWS {
            for tx in 0..COLS {
                self.ui_back
                    .set_tile((tx, ty), ui_ts, TileSetting::new(UI_EMPTY, e));
                self.ui_front
                    .set_tile((tx, ty), ui_ts, TileSetting::new(UI_TRANS, e));
            }
        }
        self.active_sprites.clear();
        self.sprite_cache.begin_frame();
    }

    /// Commit the current layers: back fill, then card sprites (P1) in
    /// deterministic order, then front text (P0). Sprites not shown this
    /// frame are simply absent from OAM — no manual clearing, no ghosts.
    /// Placements nothing showed are evicted from the cache here, so VRAM
    /// holds exactly the visible set.
    fn present(&mut self) {
        self.sprite_cache.end_frame();
        let mut f = self.gfx.frame();
        self.ui_back.show(&mut f);
        for obj in &self.active_sprites {
            obj.show(&mut f);
        }
        self.ui_front.show(&mut f);
        f.commit();
    }

    /// Push all sprite parts of one card placement into `active_sprites`.
    /// `tiles` is the baked 8bpp grid (`grid_w` x `grid_h` tiles); the art is
    /// centred in the `box_w` x `box_h` px box at (`px`, `py`).
    #[allow(clippy::too_many_arguments)]
    fn push_card(
        &mut self,
        tag: &str,
        card_no: &str,
        tiles: &[u8],
        grid_w: usize,
        grid_h: usize,
        box_w: usize,
        box_h: usize,
        parts: &[(Size, usize, usize)],
        px: i32,
        py: i32,
        flipped: bool,
        dimmed: bool,
        rot_cw: bool,
    ) {
        // Board stage twins share art but never show together; name the
        // other so a miss evicts it first (no room for both).
        let twin_tag: Option<&str> = match tag {
            "stage" => Some("stagew"),
            "stagew" => Some("stage"),
            _ => None,
        };
        // Disjoint fields: the cache borrow and the sprite-list push coexist
        // with no intermediate Vec clone.
        match self.sprite_cache.get_or_upload(
            tag, card_no, tiles, grid_w, grid_h, box_w, box_h, parts, flipped,
            dimmed, rot_cw, twin_tag,
        ) {
            Some(vrams) => {
                for (vram, &(_, dx, dy)) in vrams.iter().zip(parts.iter()) {
                    let mut obj = Object::new(vram.clone());
                    obj.set_pos((px + dx as i32, py + dy as i32));
                    obj.set_priority(SPRITE_PRIO);
                    self.active_sprites.push(obj);
                }
            }
            None => {
                log::debug!("dropping card {} (no sprite VRAM)", card_no);
            }
        }
    }

    /// Render the integrated board: sprite card art between the back fill
    /// and the front text/UI layer.
    pub fn render_board_frame(&mut self, frame: &BoardFrame) {
        self.last = self.buf.clone();
        // Detail portraits are large (9 parts); evict them before the board
        // re-uploads so the two never share the 32KB OBJ VRAM budget.
        self.sprite_cache.evict_prefix("detail:");
        self.gfx.set_background_palette(15, &TEXT_PALETTE);
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
        let e0 = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e0);

        Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e0, &frame.header, 0, 0);
        Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e0, &frame.action_count, COLS - 6, 0);

        // Stage rows: opponent (rotated 180°) then player, with live success
        // + live set stacked on the right.
        for (row, y) in STAGE_YS.iter().enumerate() {
            let y = *y;
            let is_opp = row == 0;
            let stage = if is_opp { &frame.p2_stage } else { &frame.p1_stage };
            let live = if is_opp { &frame.p2_live } else { &frame.p1_live };
            let live_set = if is_opp { &frame.p2_live_set } else { &frame.p1_live_set };

            for (i, slot) in stage.iter().enumerate() {
                let xi = if is_opp { 2 - i } else { i };
                let x = 1 + STAGE_PITCH * xi as i32;
                self.draw_slot_flipped(&font_ts, &icon_ts, &ui_ts, e0, slot, x, y, SlotKind::Stage, is_opp);
            }
            // Live/success zone (top 3 rows of stage row)
            for (i, slot) in live.iter().enumerate() {
                let xi = if is_opp { 2 - i } else { i };
                let x = INFO_X + LIVE_PITCH * xi as i32;
                self.draw_slot_flipped(&font_ts, &icon_ts, &ui_ts, e0, slot, x, y, SlotKind::Live, is_opp);
            }
            // Live card set zone (bottom 3 rows of stage row)
            for (i, slot) in live_set.iter().enumerate() {
                let xi = if is_opp { 2 - i } else { i };
                let x = INFO_X + LIVE_PITCH * xi as i32;
                self.draw_slot_flipped(&font_ts, &icon_ts, &ui_ts, e0, slot, x, y + 3, SlotKind::Live, is_opp);
            }
        }

        // Hand window; a gold badge marks more cards off-screen right.
        for (i, slot) in frame.hand.iter().enumerate() {
            let x = HAND_PITCH * i as i32;
            self.draw_slot_flipped(&font_ts, &icon_ts, &ui_ts, e0, slot, x, HAND_Y, SlotKind::Hand, false);
        }
        if frame.hand_more {
            self.ui_front.set_tile((COLS - 1, HAND_Y), &ui_ts, TileSetting::new(UI_BADGE, e0));
        }

        // Cursor: hand or stage (depending on L-cycled focus). The white
        // triangle shares the gold badge's tile, so it sits on the card's
        // right side overlapping the badge (drawn after badges, so it wins).
        if let Some(w) = frame.hand_cursor {
            let x = HAND_PITCH * w as i32;
            let (mx, my) = Self::badge_tile(x, HAND_Y, SlotKind::Hand, false);
            self.ui_front.set_tile((mx, my), &ui_ts, TileSetting::new(UI_MARKER, e0));
        }
        if let Some(idx) = frame.own_stage_cursor {
            let x = STAGE_START_X + STAGE_PITCH * idx as i32;
            let (mx, my) = Self::badge_tile(x, STAGE_YS[1], SlotKind::Stage, false);
            self.ui_front.set_tile((mx, my), &ui_ts, TileSetting::new(UI_MARKER, e0));
        }
        if let Some(idx) = frame.opp_stage_cursor {
            // Opponent cards are drawn mirrored (2 - i, like the 3DS far
            // side), so the cursor must mirror too or it points at the
            // wrong card.
            let x = STAGE_START_X + STAGE_PITCH * (2 - idx) as i32;
            let (mx, my) = Self::badge_tile(x, STAGE_YS[0], SlotKind::Stage, true);
            self.ui_front.set_tile((mx, my), &ui_ts, TileSetting::new(UI_MARKER, e0));
        }

        // Action bar pinned to the bottom (single 16px line; hint lives in header).
        let bar = alloc::format!("> {}", frame.action_line);
        Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e0, &bar, 0, BAR_Y);

        self.present();
    }

    /// [`Display::draw_slot`] with an explicit 180° flag (opponent rows).
    fn draw_slot_flipped(
        &mut self,
        font_ts: &TileSet,
        icon_ts: &TileSet,
        ui_ts: &TileSet,
        e: TileEffect,
        slot: &Slot,
        x: i32,
        y: i32,
        kind: SlotKind,
        flipped: bool,
    ) {
        let Some(card_no) = slot.card_no.as_deref() else {
            return;
        };
        // Resolve art. Waited stage cards rotate the full stage front 90° CW
        // to fill the 48px box (like 3DS tapped cards fill their slot);
        // waited hand/live slots keep the small pre-rotated front centred.
        let (tiles, gw, gh, tag, rot_cw): (&[u8], usize, usize, &str, bool) = if slot.hidden {
            (BACK_FRONT, 3, 2, "back", false)
        } else if slot.waited && matches!(kind, SlotKind::Stage) {
            match STAGE_FRONTS.iter().find(|f| f.card_no == card_no) {
                Some(f) => (f.tiles, kind.grid_w(), kind.grid_h(), "stagew", true),
                None => {
                    // Missing baked art must degrade to text, never to an
                    // invisible slot — a silent `return` here once hid
                    // waited cards entirely.
                    log::debug!("missing stage front for waited {}", card_no);
                    Self::blit_line(
                        &mut self.ui_front,
                        font_ts,
                        icon_ts,
                        e,
                        card_no,
                        x,
                        y + kind.rows() / 2,
                    );
                    return;
                }
            }
        } else if slot.waited {
            match WAITED_FRONTS.iter().find(|f| f.card_no == card_no) {
                Some(f) => (f.tiles, WAIT_GRID.0, WAIT_GRID.1, "wait", false),
                None => {
                    log::debug!("missing waited front for {}", card_no);
                    Self::blit_line(
                        &mut self.ui_front,
                        font_ts,
                        icon_ts,
                        e,
                        card_no,
                        x,
                        y + kind.rows() / 2,
                    );
                    return;
                }
            }
        } else {
            let fronts = kind.fronts();
            match fronts.iter().find(|f| f.card_no == card_no) {
                Some(f) => (f.tiles, kind.grid_w(), kind.grid_h(), kind.tag(), false),
                None => {
                    Self::blit_line(
                        &mut self.ui_front,
                        font_ts,
                        icon_ts,
                        e,
                        card_no,
                        x,
                        y + kind.rows() / 2,
                    );
                    return;
                }
            }
        };
        let (box_w, box_h) = kind.px();
        self.push_card(
            tag,
            card_no,
            tiles,
            gw,
            gh,
            box_w,
            box_h,
            kind.parts(),
            x * 8,
            y * 8,
            flipped,
            false,
            rot_cw,
        );
        if slot.actionable {
            let (bx, by) = Self::badge_tile(x, y, kind, flipped);
            // Stage badges sit one tile in on the nudged edge tile; the
            // flipped corner and hand/live badges keep the centred tile.
            let tile = if !flipped && matches!(kind, SlotKind::Stage) {
                UI_BADGE_EDGE
            } else {
                UI_BADGE
            };
            self.ui_front
                .set_tile((bx, by), ui_ts, TileSetting::new(tile, e));
        }
    }

    /// Gold-badge tile for a card at tile (`x`, `y`): top-right corner, or
    /// bottom-left when shown rotated 180° (opponent row). The cursor marker
    /// shares this tile so it sits on the card, overlapping the badge.
    ///
    /// Stage art's bright border ends well inside the 40px grid (the rest is
    /// dark margin), so its badge sits one tile left of the grid edge —
    /// otherwise it floats over padding right of the visible card. Hand/live
    /// art fills its grid, so those keep the last tile.
    fn badge_tile(x: i32, y: i32, kind: SlotKind, flipped: bool) -> (i32, i32) {
        if flipped {
            (x, y + kind.rows() - 1)
        } else if matches!(kind, SlotKind::Stage) {
            (x + kind.cols() - 2, y)
        } else {
            (x + kind.cols() - 1, y)
        }
    }

    /// Render the full-screen Actions view: the buffered action list with a
    /// small hint line at the top. Input stays with the engine so Up/Down/A
    /// drive the list live. No cards here, so the sprite cache is dropped to
    /// free OBJ VRAM for the next screen.
    ///
    /// The engine prints more lines than fit (3 headers + up to 7 actions +
    /// a `..more` line); the window follows the `>` cursor so the list
    /// scrolls fully instead of clipping the selected row away.
    pub fn render_action_text(&mut self) {
        self.last = self.buf.clone();
        self.sprite_cache.clear();
        self.gfx.set_background_palette(15, &TEXT_PALETTE);
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
        let e = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e);

        Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e, "ACTIONS [Sel:Board] [Sta:Menu]", 0, 0);
        // Cursor-following window: title pinned, 8 content rows below it.
        // Two borrow-separated passes over buf, zero allocation: the offset
        // pass only reads, the render pass only draws.
        const VISIBLE: usize = 8;
        let (mut count, mut cursor, mut found) = (0usize, 0usize, false);
        for (i, line) in self.buf.split('\n').enumerate() {
            count = i + 1;
            if !found && line.starts_with(" >") {
                cursor = i;
                found = true;
            }
        }
        let cursor = if found { cursor } else { 0 };
        let max_off = count.saturating_sub(VISIBLE);
        if self.action_offset > max_off {
            self.action_offset = max_off;
        }
        if cursor < self.action_offset {
            self.action_offset = cursor;
            log::debug!("actions scroll up to {}", cursor);
        }
        if cursor >= self.action_offset + VISIBLE {
            self.action_offset = cursor + 1 - VISIBLE;
            log::debug!("actions scroll down to {}", cursor);
        }
        for (i, line) in self
            .buf
            .split('\n')
            .skip(self.action_offset)
            .take(VISIBLE)
            .enumerate()
        {
            let row = 2 + i as i32 * 2;
            if row + 2 > ROWS {
                break;
            }
            let color = if line.contains("Pass") || line.contains("Confirm") {
                TileEffect::new(false, false, 15)
            } else if line.starts_with("  ..") {
                e
            } else {
                e
            };
            Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, color, line, 0, row);
        }

        self.present();
    }

    /// Render a card-detail view: the 96x144 portrait as 9 sprites left of
    /// the text pane. Detail art shares the master sprite palette, so unlike
    /// the old BG path no palette reload (and no backdrop flash) is needed.
    pub fn render_card_detail(&mut self, art: Option<&CardArt>, lines: &[String], scroll: usize) {
        self.last = self.buf.clone();
        // Board sprites would share the 32KB OBJ budget with the 9-part
        // portrait — evict everything EXCEPT this card's portrait, so
        // scrolling (same art, new offset) is a cache hit with no re-upload.
        match art {
            Some(a) => self.sprite_cache.retain_key(
                &alloc::format!("detail:{}:{}x{}:n:n", a.card_no, DETAIL_PX.0, DETAIL_PX.1),
            ),
            None => self.sprite_cache.clear(),
        }
        self.gfx.set_background_palette(15, &DETAIL_TEXT_PALETTE);
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
        let e_text = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e_text);

        if let Some(art) = art {
            log::debug!("detail portrait for {}", art.card_no);
            // Decompress LZ77-compressed tiles (13824 bytes for 96x144 detail art)
            let mut decompressed_tiles = [0u8; 13824];
            crate::card_art_gen::lz77_decompress_wram(art.tiles, &mut decompressed_tiles);
            self.push_card(
                "detail",
                art.card_no,
                &decompressed_tiles,
                12,
                18,
                DETAIL_PX.0,
                DETAIL_PX.1,
                DETAIL_PARTS,
                0,
                DETAIL_Y0 * 8,
                false,
                false,
                false,
            );
        }

        const VISIBLE: usize = 8;
        let end = (scroll + VISIBLE).min(lines.len());
        for (i, line) in lines[scroll..end].iter().enumerate() {
            Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e_text, line, DETAIL_DW as i32 + 1, i as i32 * 2);
        }
        // Scroll indicators
        if scroll > 0 {
            Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e_text, "^", 29, 0);
        }
        if end < lines.len() {
            Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e_text, "v", 29, 18);
        }

        self.present();
    }

    /// Reset VRAM pressure: drop all cached sprites so the next screen
    /// uploads into a free pool. Call on heavy screen transitions (detail
    /// open/close, choice open/close). Deliberately commits NOTHING: an
    /// empty commit was the transition flash (one blank frame). Sprite VRAM
    /// frees on drop (refcounted, no commit needed); BG tiles GC at the next
    /// real present.
    pub fn reset_vram(&mut self) {
        self.sprite_cache.clear();
    }

    /// Queue a card image for the next [`Display::swap_buffers`]. Called by
    /// the `PlatformUi::draw_card_image` impl so generic choice menus show
    /// real card fronts (3DS-style) instead of a text-only list. Coordinates
    /// are in tiles; `selected` draws the cursor badge, `dimmed` dithers
    /// the card dark (unpickable, like the 3DS `disabled` overlay).
    pub fn queue_card_image(
        &mut self,
        card_no: &str,
        x: i32,
        y: i32,
        cols: i32,
        rows: i32,
        selected: bool,
        dimmed: bool,
    ) {
        use alloc::string::ToString;
        self.pending_art.push(PendingArt {
            card_no: card_no.to_string(),
            x,
            y,
            cols,
            rows,
            selected,
            dimmed,
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

        // Bank 15 holds TEXT_PALETTE (white text on dark blue). The detail
        // view overwrites bank 15, so restore it here.
        self.gfx.set_background_palette(15, &TEXT_PALETTE);
        // Menu cards share kinds with the board; detail portraits do not —
        // evict those before uploading.
        self.sprite_cache.evict_prefix("detail:");
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(BOARD_UI, TileFormat::FourBpp) };
        let e_ui = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e_ui);

        let e = TileEffect::new(false, false, 15);
        let mut ty = 0i32;

        for line in self.buf.split('\n') {
            if ty + 2 > ROWS {
                break;
            }
            Self::blit_text(&mut self.ui_front, &font_ts, &icon_ts, e, line, 0, ty, true);
            ty += 2;
        }

        if self.pending_art.is_empty() {
            self.present();
            return;
        }

        // Sort by Y then X for deterministic z-ordering. Take ownership
        // of the queue first: draining while pushing sprites would double-
        // borrow self.
        self.pending_art.sort_by_key(|q| (q.y, q.x));
        let arts = core::mem::take(&mut self.pending_art);
        for q in arts {
            // Stage-size requests (5x6, the choice grid) use the stage
            // fronts; anything else uses the hand-size fronts.
            let (kind, fronts) = if q.cols == 5 && q.rows == 6 {
                (SlotKind::Stage, STAGE_FRONTS)
            } else {
                (SlotKind::Hand, CARD_FRONTS)
            };
            if q.card_no.is_empty() {
                if q.selected {
                    self.ui_front.set_tile((q.x, q.y), &ui_ts, TileSetting::new(UI_BADGE, e_ui));
                }
            } else if let Some(front) = fronts
                .iter()
                .find(|f| f.card_no == q.card_no.as_str())
            {
                // Menu boxes are exactly the queued tile size (choice grid
                // cards have no wait-gap, unlike board stage slots).
                let (box_w, box_h) = ((q.cols * 8) as usize, (q.rows * 8) as usize);
                self.push_card(
                    kind.tag(),
                    &q.card_no,
                    front.tiles,
                    kind.grid_w(),
                    kind.grid_h(),
                    box_w,
                    box_h,
                    kind.parts(),
                    q.x * 8,
                    q.y * 8,
                    false,
                    q.dimmed,
                    false,
                );
                if q.selected {
                    // Gold edge badge on top-right of card, drawn after
                    // dimming so the cursor stays visible. The nudged tile
                    // straddles the art's visible border (see UI_BADGE_EDGE);
                    // the centred tile would float over padding.
                    self.ui_front.set_tile((q.x + q.cols - 2, q.y), &ui_ts, TileSetting::new(UI_BADGE_EDGE, e_ui));
                }
            } else {
                Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e, &q.card_no, q.x, q.y);
            }
        }

        self.present();
    }

    pub fn wait(&mut self) {
        busy_wait_for_vblank();
    }
}

/// Card slot geometry: baked grid, on-screen box, sprite tiling, fronts.
#[derive(Clone, Copy)]
enum SlotKind {
    Hand,
    Stage,
    Live,
}

impl SlotKind {
    fn fronts(self) -> &'static [CardFront] {
        match self {
            SlotKind::Hand => CARD_FRONTS,
            SlotKind::Stage => STAGE_FRONTS,
            SlotKind::Live => LIVE_FRONTS,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            SlotKind::Hand => "hand",
            SlotKind::Stage => "stage",
            SlotKind::Live => "live",
        }
    }

    fn px(self) -> (usize, usize) {
        match self {
            SlotKind::Hand => HAND_PX,
            SlotKind::Stage => STAGE_PX,
            SlotKind::Live => LIVE_PX,
        }
    }

    fn grid_w(self) -> usize {
        match self {
            SlotKind::Hand => 3,
            SlotKind::Stage => 5,
            SlotKind::Live => 3,
        }
    }

    fn grid_h(self) -> usize {
        match self {
            SlotKind::Hand => 4,
            SlotKind::Stage => 6,
            SlotKind::Live => 2,
        }
    }

    fn parts(self) -> &'static [(Size, usize, usize)] {
        match self {
            SlotKind::Hand => HAND_PARTS,
            SlotKind::Stage => STAGE_PARTS,
            SlotKind::Live => LIVE_PARTS,
        }
    }

    fn cols(self) -> i32 {
        match self {
            SlotKind::Hand => HAND_CARD.0,
            SlotKind::Stage => STAGE_CARD.0,
            SlotKind::Live => LIVE_CARD.0,
        }
    }

    fn rows(self) -> i32 {
        match self {
            SlotKind::Hand => HAND_CARD.1,
            SlotKind::Stage => STAGE_CARD.1,
            SlotKind::Live => LIVE_CARD.1,
        }
    }
}

/// Detail portrait grid: 12x18 tiles (96x144px) at rows 1-18, left of the
/// text pane.
const DETAIL_DW: usize = 12;
#[allow(dead_code)]
const DETAIL_DH: usize = 18;
/// First tile row of the portrait (18 tall on a 20-row screen).
const DETAIL_Y0: i32 = 1;

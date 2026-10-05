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
use agb::ExternalAllocator;

use crate::board::{BoardFrame, Slot};
use crate::card_art_gen::{CardArt, CardFront, BACK_FRONT, BOARD_UI, CARD_ART, CARD_FRONTS, LIVE_FRONTS, MASTER_PAL, STAGE_FRONTS, WAITED_FRONTS, lz77_decompress_wram};
use crate::font_tiles_gen::{FONT_GLYPHS, FONT_TILES};
use crate::texticons_gen::{TEXTICON_GLYPHS, TEXTICON_TILES};

/// Diagnostic line that actually reaches an emulator console.
///
/// Do NOT use `log::debug!` here. Nothing in this port installs a logger
/// (`log`'s `set_logger`/`set_max_level` are gated behind
/// `target_has_atomic = "ptr"`, which ARMv4T does not have, so they are
/// compiled out entirely), and with no logger the `log` facade's
/// `max_level()` is `Off`. Every `log::debug!` therefore expanded to a
/// no-op: the sprite cache, the VRAM budget and every "missing front"
/// fallback were silently invisible, which is why the card-art fault
/// could not be localised from a build.
///
/// `agb::println!` does transmit, but one line per call blocks on the
/// mgba debug port, so these stay behind the diag level (`SELECT`-held
/// verbose, see `diag::set_verbose`) rather than spraying per frame.
macro_rules! dlog {
    ($($arg:tt)*) => {
        if $crate::diag::want_debug() {
            $crate::diag::note(format_args!($($arg)*));
        }
    };
}

/// Screen is 240x160 = 30 cols x 20 rows of 8px tiles. Text glyphs are
/// 16x16 (2x2 tiles, 12px font) 窶・each text line is 2 tile-rows.
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
/// 4px gap each side, so tapped (wait-state) cards rotated 90ﾂｰ to 48px wide
/// exactly fill their box without touching neighbours 窶・like the 3DS square
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

/// agb's 4bpp BG tile pool, in tiles.
///
/// agb splits 0x0600_0040-0x0600_8000 into an affine region (510 tiles) and a
/// regular region (512 tiles), and `alloc_for_regular` ends in
/// `.expect("Ran out of video RAM for tiles")` -- a panic, not a slowdown.
/// `VRAM_MANAGER.gc()` only runs inside `GraphicsFrame::commit()`, so while
/// frame N is being built frame N-1's tiles are still resident: only half the
/// pool is available to a single frame.
///
/// The baked font alone is 952 glyphs = 840 fullwidth (2x2 = 4 tiles) + 112
/// halfwidth (2 tiles) = 3,584 tiles, 3.5x the whole pool. So the font must
/// never be uploaded wholesale through the shared allocator, and a single
/// frame's text must be capped below this.
const BG_TILE_POOL: usize = 1022;
const BG_TILE_BUDGET_PER_FRAME: usize = BG_TILE_POOL / 2;

/// Upper bound on text rows any one screen blits.
///
/// A 2x2 Japanese glyph costs 4 tiles, so 9 rows x 30 columns of ability text
/// can reference well over a thousand distinct tiles on its own. Truncating by
/// row count alone (the old `ty + 2 > ROWS` check) does not bound that.
const MAX_TEXT_ROWS: i32 = 8;

/// Layering (GBA: lower number wins; OBJ beats a BG of the SAME number, so
/// every layer gets its own number):
/// front text/badges P0 > card sprites P1 > back zone fill P2.
const SPRITE_PRIO: Priority = Priority::P1;

/// Card pixel boxes (on-screen footprint; art is centred inside).
/// Stage is 48px wide (not 40): the gap fits tapped cards rotated 90ﾂｰ.
const HAND_PX: (usize, usize) = (24, 32);
const STAGE_PX: (usize, usize) = (48, 48);
const LIVE_PX: (usize, usize) = (24, 16);
/// Waited (tapped) art grid: 4x3 tiles = 32x24px, centred in the slot box.
const WAIT_GRID: (usize, usize) = (4, 3);
const DETAIL_PX: (usize, usize) = (96, 144);

/// Sprite tilings per card box: (hardware size, x-offset, y-offset) in px.
/// Every entry is a legal GBA sprite size; gutters (sprite area outside the
/// card box) are filled with palette index 0 = transparent.
///
/// These deliberately tile the full `*_PX` box even where the art is narrower,
/// matching the geometry in place since the "actual sprite refactor"
/// (916f8851). A tighter tiling does save OBJ VRAM -- hand 1024B->768B and
/// live 1024B->384B -- but it was never verified to rasterise identically, and
/// a rendering fault is far more expensive than the bytes. Reintroduce only
/// with a side-by-side capture proving the output matches.
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

/// Presents to skip before diag-shot captures, so the filmstrip shows the
/// menus and board rather than the blank first screen.
const SHOT_SKIP: u32 = 40;

/// Upper bound on cached card placements. Board steady state is ~28 keys;
/// anything far above that is a screen-transition leak, so drop everything
/// and re-upload (one hitch beats an OBJ-VRAM OOM panic).
const SPRITE_CACHE_CAP: usize = 96;

/// Full-screen text + sprite-composited cards.
///
/// * `ui_back` (P2): opaque zone fill, always behind cards.
/// * card sprites (P1): all card fronts/backs, multi-Object per card.
/// * `ui_front` (P0): text, badges, cursors 窶・transparent elsewhere so
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
    /// Which screen last pushed art, so a switch can prime agb's sprite pins.
    screen: Screen,
    /// Frames of art suppression left after a screen switch. See
    /// [`Display::enter_screen`].
    priming: u8,
}

/// The sprite-art "screens". Two consecutive renders on the same `Screen`
/// share a cache generation; a change means the old screen's placements are
/// about to be replaced wholesale.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// No card art at all (mode select, deck select, title).
    None,
    /// Full-screen buffered text + queued menu cards.
    Menu,
    /// The detail portrait view (one very large placement).
    Detail,
    /// The live board.
    Board,
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
/// its unrotated self) does NOT fit 窶・its upload fails and the card would
/// vanish instead of rotating. Two rules keep the budget exact:
/// - `begin_frame`/`end_frame` retain only placements shown this frame;
///   stale variants die at the frame edge, never accumulate.
/// - callers pass `evict` keys replaced by the new upload (waited stage art
///   evicts its unrotated twin first), so transitions never exceed steady
///   state even mid-frame.
/// One cached card placement: all parts plus the frame stamp for the
/// end-of-frame retention sweep, and the OBJ-VRAM bytes the parts occupy.
struct CachedPlacement {
    vrams: Vec<SpriteVram>,
    last_used: u32,
    /// `vrams.len()` is a part count, not a byte count 窶・the sizes differ per
    /// kind (stage is 4 parts, live is 1). Budgeting needs the real bytes.
    bytes: usize,
    /// Where this placement was last drawn, for `diag-trace` reporting only.
    last_at: (i32, i32),
}

/// OBJ VRAM budget for cached card art.
///
/// The GBA has 32,768 B of sprite character VRAM and agb's sprite allocator
/// owns exactly that (`SPRITE_ALLOCATOR` = 0x06010000 + 32KB); there is no
/// paging. Card art must be 8bpp 窶・MASTER_PAL is 240 colours and a 4bpp OBJ
/// bank holds 16 窶・and an 8bpp tile is 64 B, double a 4bpp one.
///
/// Measured worst case for a full 1v1 board (verified against `BoardFrame` in
/// board.rs, not estimated):
///
///     6 stage   4 parts/ 36 tiles = 2304B each = 13,824
///     6 live    1 part /  8 tiles =  512B each =  3,072
///     6 live-set (face-down)         512B each =  3,072
///     10 hand   2 parts/ 16 tiles = 1024B each = 10,240
///                                          total = 30,208
///
/// plus one extra face-down placement for the flipped opponent row (1024
/// above once deduped to one key) gives a measured steady state of 31,232B
/// against 32,768B of VRAM. That is the whole game in 95% of the arena, so the
/// budget is set just under the hardware ceiling: `evict_for` then only trims a
/// genuine overflow, and the ~1KB of slack is what absorbs a transition 窶・agb
/// holds `previous_frame_sprites` alive across a commit, so for one frame the
/// old and new placements coexist.
const SPRITE_VRAM_BUDGET: usize = 31 * 1024;

/// OBJ VRAM a placement's parts will occupy. 8bpp: 64 bytes per tile.
fn placement_bytes(parts: &[(Size, usize, usize)]) -> usize {
    parts.iter().map(|(size, _, _)| size.size_bytes_256()).sum()
}

struct CardSpriteCache {
    sprites: BTreeMap<String, CachedPlacement>,
    palette: PaletteVramMulti,
    /// Sum of every cached placement's `bytes`. Maintained on insert/remove so
    /// the budget check is O(1).
    vram_bytes: usize,
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
    /// One-time index into the baked front tables: table positions sorted by
    /// `card_no`. All baked tables (`CARD_ART`, `CARD_FRONTS`, `STAGE_FRONTS`,
    /// `LIVE_FRONTS`, `WAITED_FRONTS`) are appended in the same completion
    /// order by `tools/bake_card_art.py`, so a single order serves every
    /// table 窶・1809 x u16 (~3.6KB once) instead of a ~1800-entry linear
    /// string scan per card per frame (~22 scans/frame on the board).
    /// If the baker ever emits per-table orders, build one order per table.
    front_order: Vec<u16>,
}

/// 16-bit FNV-1a of a card number, used only by the `diag-cardid` build to
/// give every card a distinguishable on-screen identity.
fn card_fnv16(card_no: &str) -> u16 {
    let mut h: u16 = 0x811c;
    for b in card_no.as_bytes() {
        h ^= *b as u16;
        h = h.wrapping_mul(0x0193);
    }
    h
}

/// Write the sprite-cache key for one placement into `buf`.
///
/// Single source of truth for the key format. `get_or_upload`, its
/// twin-eviction path and `render_card_detail`'s `retain_key` must all
/// agree: they used to format the key separately and drifted, so the
/// detail key carried a spurious trailing `:n`, never matched anything,
/// and `retain_key` wiped the cache every frame instead of retaining the
/// portrait (a full 13,824B re-inflate + re-upload per frame while
/// scrolling). Format in one place so it cannot drift again.
fn write_placement_key(
    buf: &mut String,
    tag: &str,
    card_no: &str,
    box_w: usize,
    box_h: usize,
    dimmed: bool,
) {
    use core::fmt::Write as _;
    let dim_c = if dimmed { 'd' } else { 'n' };
    let _ = write!(buf, "{}:{}:{}x{}:{}", tag, card_no, box_w, box_h, dim_c);
}

/// Owned [`write_placement_key`], for the off-hot-path callers.
fn placement_key(
    tag: &str,
    card_no: &str,
    box_w: usize,
    box_h: usize,
    dimmed: bool,
) -> String {
    let mut k = String::new();
    write_placement_key(&mut k, tag, card_no, box_w, box_h, dimmed);
    k
}

/// Binary search for `card_no` in a baked table via the shared
/// [`CardSpriteCache::front_order`]. Misses degrade to text (never to an
/// invisible slot), same as the old linear scan.
fn find_in_table<'t, T>(
    order: &[u16],
    table: &'t [T],
    card_no: &str,
    key: fn(&T) -> &'static str,
) -> Option<&'t T> {
    let pos = order
        .binary_search_by(|&i| key(&table[i as usize]).cmp(card_no))
        .ok()?;
    let hit = &table[*order.get(pos)? as usize];
    debug_assert_eq!(key(hit), card_no);
    Some(hit)
}

impl CardSpriteCache {
    fn new(_gfx: &mut Graphics) -> Self {
        // Build 256-colour master palette from baked MASTER_PAL.
        // Index 0 is forced magenta at bake time and is the OBJ
        // transparent index, so no art pixel may use it (verified: the bake
        // emits zero index-0 pixels; gutters are filled with 0 here).
        // PaletteMulti is 16 Palette16 entries (16*16=256 colours).
        // Use Box::leak to get 'static lifetime for PaletteMulti::new.
        dlog!(
            "sprite palette master[0]={:02x}{:02x} (expect magenta 1f7c)",
            MASTER_PAL[0],
            MASTER_PAL[1]
        );
        let palettes: Box<[Palette16; 16]> = Box::new(core::array::from_fn(|_| Palette16::new([Rgb15::BLACK; 16])));
        let mut palettes = palettes;
        for i in 0..240 {
            // `diag-spriteraw` replaces the ramp with a colour that reads the
            // 8bpp palette index straight back off the screen: the low 3 bits
            // of the index go to red, the next 3 to green, the top 2 to blue.
            // Two pixels look identical on the baseline ROM, but here their
            // colours differ exactly when their indices differ, which is what
            // separates "the tile data is wrong" from "the palette is wrong".
            #[cfg(feature = "diag-spriteraw")]
            let v = if i == 0 {
                Rgb::new(255, 0, 255).to_rgb15()
            } else {
                let idx = i as u8;
                Rgb::new(
                    (idx & 0x07).wrapping_mul(36),
                    ((idx >> 3) & 0x07).wrapping_mul(36),
                    ((idx >> 6) & 0x03).wrapping_mul(85),
                )
                .to_rgb15()
            };
            #[cfg(not(feature = "diag-spriteraw"))]
            let v = Rgb15::new(MASTER_PAL[i * 2] as u16 | ((MASTER_PAL[i * 2 + 1] as u16) << 8));
            let palette_idx = i / 16;
            let colour_idx = i % 16;
            palettes[palette_idx].update_colour(colour_idx, v);
        }
        let palette_multi = Box::leak(Box::new(PaletteMulti::new(Box::leak(palettes))));
        let palette = PaletteVramMulti::new(palette_multi);
        let mut key_buf = String::new();
        key_buf.reserve(64);
        let mut twin_buf = String::new();
        twin_buf.reserve(64);
        // One-time sort of table positions by card_no (see `front_order`).
        // All baked tables share the baker's completion order, so index one
        // and serve all; lengths must agree or lookups alias the wrong art.
        debug_assert_eq!(CARD_FRONTS.len(), STAGE_FRONTS.len());
        debug_assert_eq!(CARD_FRONTS.len(), LIVE_FRONTS.len());
        debug_assert_eq!(CARD_FRONTS.len(), WAITED_FRONTS.len());
        debug_assert_eq!(CARD_FRONTS.len(), CARD_ART.len());
        let mut front_order: Vec<u16> = (0..CARD_FRONTS.len().min(u16::MAX as usize) as u16).collect();
        front_order.sort_by(|&a, &b| CARD_FRONTS[a as usize].card_no.cmp(&CARD_FRONTS[b as usize].card_no));
        dlog!("front index built for {} cards", front_order.len());
        Self {
            sprites: BTreeMap::new(),
            vram_bytes: 0,
            palette,
            frame: 1,
            touched: false,
            key_buf,
            twin_buf,
            front_order,
        }
    }

    /// Indexed front lookup (binary search, no per-frame linear scan).
    fn find_front(&self, table: &'static [CardFront], card_no: &str) -> Option<&'static CardFront> {
        let hit = find_in_table(&self.front_order, table, card_no, |f| f.card_no);
        if hit.is_none() {
            dlog!("missing front for {}", card_no);
        }
        hit
    }

    /// Indexed detail-art lookup for the card-detail screen.
    fn find_art(&self, card_no: &str) -> Option<&'static CardArt> {
        let hit = find_in_table(&self.front_order, CARD_ART, card_no, |a| a.card_no);
        if hit.is_none() {
            dlog!("missing detail art for {}", card_no);
        }
        hit
    }

    /// Start a frame. The id wraps after ~2 years at 60fps; on wrap the
    /// whole cache is dropped once so no stale stamp can survive.
    fn begin_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.frame == 0 {
            self.sprites.clear();
            self.vram_bytes = 0;
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
        self.recount_bytes();
        if self.sprites.len() != before {
            dlog!(
                "sprite retain: {} -> {} keys ({}B vram)",
                before,
                self.sprites.len(),
                self.vram_bytes
            );
        }
    }

    /// Upload one sprite part: sample the baked tile grid (centred in the
    /// card box, optionally 90ﾂｰ CCW for the wait-state, and dither-dimmed) via
    /// `set_pixel` so rectangular sizes use the correct OBJ 1D-mapping layout.
    /// Gutter pixels (sprite area outside the card box) stay index 0.
    ///
    /// The 180ﾂｰ opponent-side turn is deliberately absent: it is an OAM
    /// hflip+vflip, so rasterising it here would only duplicate VRAM.
    #[allow(clippy::too_many_arguments)]
    fn upload_part(
        tiles: &[u8],
        grid_w: usize,
        grid_h: usize,
        box_w: usize,
        box_h: usize,
        dimmed: bool,
        rot_ccw: bool,
        size: Size,
        dx: usize,
        dy: usize,
        palette: &PaletteVramMulti,
        id_pattern: Option<u16>,
    ) -> Option<SpriteVram> {
        let art_w = grid_w * 8;
        let art_h = grid_h * 8;
        // Rotated (wait-state) footprint: dimensions swap, like the 3DS
        // tapped path which rotates 90ﾂｰ CCW to fill the slot.
        let (rw, rh) = if rot_ccw { (art_h, art_w) } else { (art_w, art_h) };
        // Centre the art inside the card box. Negative only if art overflows
        // the box, in which case the outer pixels clip to transparent.
        let ox = (box_w as i32 - rw as i32) / 2;
        let oy = (box_h as i32 - rh as i32) / 2;
        let (sw, sh) = size.to_width_height();
        // Stage the rasterised part in EWRAM, not agb's default IWRAM arena.
        //
        // `DynamicSprite256::new` allocates its pixel buffer from
        // `InternalAllocator`, whose whole arena is IWRAM minus whatever agb's
        // own `.iwram` section (IRQ handlers, VBlank/DMA, panic renderer) already
        // occupies -- 32KB total, shared with the BIOS interrupt stack at the
        // very top. A single stage part needs 1,024B and a detail part 2,048B,
        // so once that arena is nearly full `new` hits its
        // `.expect("should be able to allocate sprite buffer")` and the panic
        // path dies in the panic renderer itself: the machine faults writing
        // through a garbage pointer and ends up executing from 0x040007F4.
        // That was the "crashes immediately or right after the mulligan"
        // symptom, and it is invisible with `diag-noart` because that build
        // never reaches this line.
        //
        // `ExternalAllocator` is the same arena agb's global allocator uses
        // (256KB of EWRAM), and it returns a `Result` instead of panicking, so
        // genuine exhaustion now drops one card instead of killing the game.
        let mut sprite = match DynamicSprite256::try_new_in(size, ExternalAllocator) {
            Ok(s) => s,
            Err(_) => {
                dlog!("no EWRAM for {}B sprite staging", size.size_bytes_256());
                return None;
            }
        };
        // Gutters (sprite area outside the card box) must be index 0 =
        // transparent. clear() it explicitly: `new` does not guarantee
        // zero-fill, and stale buffer contents here read as confetti.
        sprite.clear(0);
        for sy in 0..sh {
            for sx in 0..sw {
                let bx = dx + sx;
                let by = dy + sy;
                // Outside the card box: transparent gutter.
                if bx >= box_w || by >= box_h {
                    continue;
                }
                // `diag-cardid`: paint a per-card-number pattern instead of
                // the art, so the reported "five different card numbers,
                // five identical flowers" becomes a direct read of whether
                // the upload/placement path is handing every card the same
                // tile block. The pattern is derived from a 16-bit FNV hash
                // of the card number, split so both halves are visible:
                // bits 8..16 pick the base colour, bits 0..8 are an 8-row
                // barcode across the top-left of every part. Two cards look
                // alike on screen only if they agree in both, which a 16-bit
                // hash makes vanishingly unlikely across a 1905-card set.
                //
                // Rotation is deliberately ignored here: an ID tag that
                // rotates with the card would be unreadable when compared
                // against a neighbouring placement.
                if let Some(h) = id_pattern {
                    let base = (1 + ((h >> 8) % 119)) as u8;
                    let alt = (121 + ((h >> 8) % 119)) as u8;
                    let bit = (h >> (by / 3).min(7)) & 1;
                    let v = if bx < 8 && by < 24 && bit != 0 { alt } else { base };
                    if dimmed && (bx + by) % 2 == 0 {
                        continue;
                    }
                    sprite.set_pixel(sx, sy, v);
                    continue;
                }
                // Box pixel -> art pixel (art is centred in the box via ox/oy).
                // A 180-degree turn is NOT rasterised here: it is an OAM
                // hflip+vflip applied in `push_card`, which costs nothing and
                // lets the flipped placement share the unflipped one's VRAM.
                // Only the genuine 90-degree wait rotation needs pixels here.
                let (fx, fy) = (bx as i32, by as i32);
                // 90ﾂｰ CCW (wait-state, like 3DS tapped): the rotated frame
                // pixel (fx-ox, fy-oy) samples unrotated (W-1-y, x), i.e.
                // art top-left lands at box bottom-left.
                let (ax, ay) = if rot_ccw {
                    (art_w as i32 - 1 - (fy - oy), fx - ox)
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
                dlog!("sprite VRAM full, will evict and retry");
                None
            }
        }
    }

    /// Get (or upload) all parts of one card placement. `twin_tag` names the
    /// mutually-exclusive transform of the same art (board `stage` <->
    /// `stagew`): on a miss the twin is dropped FIRST, since there is no
    /// room to hold both twins.
    ///
    /// Allocation discipline: cache HITS allocate nothing 窶・the key formats
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
        rot_ccw: bool,
        px: i32,
        py: i32,
        twin_tag: Option<&str>,
    ) -> Option<&[SpriteVram]> {
        // Box dims are part of the key: the board stage box (48px, with the
        // wait gap) and the menu stage box (40px) share tag/art but centre
        // differently  Ealiasing them shifts art by the gap.
        // `flipped` is deliberately NOT part of the key: a 180-degree turn is
        // an OAM hflip+vflip, so the flipped and unflipped placements are the
        // same VRAM tile block. Keying on it (and rasterising it) minted a
        // private duplicate of every opponent-side card's art.
        self.touched = true;
        self.key_buf.clear();
        write_placement_key(&mut self.key_buf, tag, card_no, box_w, box_h, dimmed);
        if let Some(p) = self.sprites.get_mut(self.key_buf.as_str()) {
            p.last_used = self.frame;
        } else {
            // REMOVED: was `agb::println!("SPRITE miss ...")`, a TEMP crash
            // trace. It fired on every cache miss and `agb::println!` blocks
            // on the mgba debug port per character, so it throttled the frame
            // budget on exactly the frames that were already missing.
            //
            // Baked tiles arrive as uniform BIOS streams  Enormalize once
            // per miss into scratch; cache hits never touch `tiles` at
            // all, so steady frames pay nothing. Scratch fits the largest
            // grid (detail 12x18x64); every caller is smaller or equal.
            // Single-threaded, synchronous use only (same contract as the
            // buffer itself): the slice never escapes this branch.
            let expected = grid_w * grid_h * 64;
            let tiles: &[u8] = if expected <= TILE_SCRATCH_LEN {
                let scratch: &mut [u8] = unsafe {
                    core::slice::from_raw_parts_mut(
                        core::ptr::addr_of_mut!(TILE_SCRATCH).cast::<u8>(),
                        TILE_SCRATCH_LEN,
                    )
                };
                // Codec follows the blob kind, and "detail" is the only tag the
                // detail portrait is ever pushed with (render_card_detail). The
                // per-frame art takes the BIOS SWI; see decompress_art for why
                // the split is on this boundary and not somewhere cheaper.
                if tag == "detail" {
                    decompress_art(tiles, &mut scratch[..expected]);
                } else {
                    decompress_lz77(tiles, &mut scratch[..expected]);
                }
                &scratch[..expected]
            } else {
                tiles
            };
            if let Some(tt) = twin_tag {
                self.twin_buf.clear();
                write_placement_key(&mut self.twin_buf, tt, card_no, box_w, box_h, dimmed);
                // Miss path only: clone the twin key out to end the buffer
                // borrow before mutating the map. Goes through `drop_key` so
                // `vram_bytes` stays truthful -- a bare `remove` here leaked
                // the twin's bytes into the counter forever.
                let twin: String = self.twin_buf.as_str().into();
                self.drop_key(&twin);
            }
            if self.sprites.len() >= SPRITE_CACHE_CAP {
                dlog!(
                    "sprite cache over cap ({} keys), evicting LRU",
                    self.sprites.len()
                );
                self.evict_over_cap(SPRITE_CACHE_CAP / 2);
            }
            // The parts this placement needs, and what they will cost. The
            // budget is enforced BEFORE uploading: discovering the overrun
            // from a failed `try_to_vram` means the whole placement has to be
            // torn down and reattempted.
            let cost = placement_bytes(parts);
            let key: String = self.key_buf.as_str().into();
            if !self.evict_for(cost, &key) {
                dlog!("no sprite VRAM for {} ({}B)", key, cost);
                return None;
            }
            let mut vrams = Vec::with_capacity(parts.len());
            // `diag-cardid`: the pattern is a property of the card number, so
            // it is computed once per placement (not per part) and every part
            // of a card gets the same one, which is what makes a duplicated
            // or aliased tile block obvious.
            let id_pattern = if cfg!(feature = "diag-cardid") {
                Some(card_fnv16(card_no))
            } else {
                None
            };
            for &(size, dx, dy) in parts {
                match Self::upload_part(
                    tiles,
                    grid_w,
                    grid_h,
                    box_w,
                    box_h,
                    dimmed,
                    rot_ccw,
                    size,
                    dx,
                    dy,
                    &self.palette,
                    id_pattern,
                ) {
                    Some(v) => vrams.push(v),
                    // Budgeted up front, so this is now genuinely unexpected
                    // (the real visible set is larger than the budget allows).
                    // Drop just this card; a mass clear here is what made a
                    // full board look like noise.
                    None => {
                        dlog!("sprite upload OOM for {} ({}B)", key, cost);
                        return None;
                    }
                }
            }
            self.vram_bytes += cost;
            self.sprites.insert(
                key,
                CachedPlacement {
                    vrams,
                    last_used: self.frame,
                    bytes: cost,
                    last_at: (px, py),
                },
            );
        }
        // `diag-trace`: report a placement only when it moves. In steady state
        // the same card sits in the same slot every frame, so this fires on
        // screen switches and on anything that drags a card to a new slot --
        // which is exactly the "card drawn across several stage slots"
        // signature. Logging every frame would also stall the run, since
        // `agb::println!` blocks on the mGBA debug port per character.
        #[cfg(feature = "diag-trace")]
        if let Some(p) = self.sprites.get_mut(self.key_buf.as_str()) {
            if p.last_at != (px, py) {
                p.last_at = (px, py);
                agb::println!(
                    "[place] {} {} {} {} -> {},{} dim={} rot={} vram={}",
                    tag,
                    card_no,
                    p.vrams.len(),
                    box_w,
                    px,
                    py,
                    dimmed,
                    rot_ccw,
                    self.vram_bytes
                );
            }
        }
        // Both shared borrows of disjoint fields 窶・no clone needed.
        self.sprites
            .get(self.key_buf.as_str())
            .map(|p| p.vrams.as_slice())
    }

    /// Evict least-recently-used placements until the cache holds fewer than
    /// `cap` keys. A count-based backstop for a pathological number of tiny
    /// placements; the byte budget is the real bound, since keys differ in
    /// cost by 27x (live 512B .. detail 13824B) and so a count says nothing
    /// about whether the art fits in VRAM.
    fn evict_over_cap(&mut self, cap: usize) {
        while self.sprites.len() > cap {
            // Same live-placement guard as `evict_for`: never free a tile
            // block that the frame being built already queued objects for.
            let Some(oldest) = self
                .sprites
                .values()
                .filter(|p| p.last_used < self.frame)
                .map(|p| p.last_used)
                .min()
            else {
                return;
            };
            self.sprites.retain(|_, p| p.last_used > oldest);
            self.recount_bytes();
        }
    }

    /// Evict least-recently-used placements until `need` more bytes fit under
    /// `SPRITE_VRAM_BUDGET`, or until only `keep` remains. Returns false if
    /// it could not free enough, in which case the caller must drop the
    /// placement it was trying to add.
    ///
    /// Allocation-free: `last_used` is a frame id, so "oldest" is a MIN over
    /// values and a single `retain` drops every tie. That is what we want 窶・    /// evicting a whole frame's worth at once is what actually clears the
    /// pressure, where one-at-a-time would only make `active_sprites` grow
    /// again next frame.
    fn evict_for(&mut self, need: usize, keep: &str) -> bool {
        while self.vram_bytes + need > SPRITE_VRAM_BUDGET {
            // Never consider a placement stamped with the frame being built.
            // `push_card` has already queued `Object`s into `active_sprites`
            // for it, and agb frees a sprite's tile block as soon as the last
            // `SpriteVram` reference drops -- so evicting here would hand the
            // same VRAM to the next upload while the queued objects still draw
            // it. The card then renders in slots it was never placed in, which
            // is what "one card is split across several stage slots" looked
            // like. If only live placements remain we cannot make room, and
            // the caller drops the card instead of corrupting the frame.
            let Some(oldest) = self
                .sprites
                .iter()
                .filter(|(k, p)| k.as_str() != keep && p.last_used < self.frame)
                .map(|(_, p)| p.last_used)
                .min()
            else {
                return false;
            };
            self.sprites
                .retain(|k, p| k == keep || p.last_used > oldest);
            self.recount_bytes();
        }
        true
    }

    /// Drop one placement by key, keeping `vram_bytes` exact. Returns true if
    /// it was dropped.
    ///
    /// Refuses while the placement is live in the frame being built (same
    /// reasoning as [`CardSpriteCache::evict_for`]). Callers on the miss path
    /// used to call `HashMap::remove` directly, which freed the VRAM without
    /// ever decrementing `vram_bytes` -- the counter only ever grew, so
    /// `evict_for` progressively over-evicted and eventually could not make
    /// room at all, which is how whole rows of art went blank after the first
    /// mulligan.
    fn drop_key(&mut self, key: &str) -> bool {
        match self.sprites.get(key) {
            Some(p) if p.last_used < self.frame => {
                self.sprites.remove(key);
                self.recount_bytes();
                true
            }
            _ => false,
        }
    }

    /// Bytes of OBJ tile VRAM this cache currently accounts for.
    fn vram_bytes(&self) -> usize {
        self.vram_bytes
    }

    /// Recompute the cached byte total. Only runs on the transition paths
    /// (retain/clear/evict), never per frame, so the O(n) walk is cheaper than
    /// the risk of the incremental counter drifting.
    fn recount_bytes(&mut self) {
        self.vram_bytes = self.sprites.values().map(|p| p.bytes).sum();
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
        dlog!("retaining sprite key {}", keep);
        self.sprites.retain(|k, _| k == keep);
        self.recount_bytes();
    }

    /// Drop cached placements whose key starts with `prefix` (e.g. detail
    /// portraits before the board re-uploads into the freed OBJ VRAM).
    fn evict_prefix(&mut self, prefix: &str) {
        // Retain in place: the old collect-into-Vec<String> allocated one
        // String per dead key on every board frame (detail close). Runs on
        // transitions only, but there is no reason to ever allocate here.
        let mut dead = 0usize;
        self.sprites.retain(|k, _| {
            let drop_it = k.starts_with(prefix);
            dead += drop_it as usize;
            !drop_it
        });
        if dead > 0 {
            dlog!("evicting {} sprite keys ({})", dead, prefix);
        }
        self.recount_bytes();
    }

    fn clear(&mut self) {
        if !self.sprites.is_empty() {
            dlog!("clearing {} sprite keys", self.sprites.len());
            self.sprites.clear();
        }
        self.vram_bytes = 0;
    }
}

/// 4bpp text/board palette (bank 15). Entries 7..=15 are the texticon colours,
/// mirrored by `tools/bake_texticon_tiles.py` PALETTE_TARGETS 窶・keep in sync.
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

/// Speckle-hunt palette for the `diag-bgspeck` ROM.
///
/// `tools/bake_font_tiles.py` only ever emits palette indices **1 and 2**
/// inside a font tile: `pack_4bpp_tile` fills the tile with `0x22` (both
/// nibbles = index 2, the zone fill) and sets foreground pixels to index 1
/// (white). So *any* index 3..=15 appearing inside a glyph is foreign data
/// written by something else.
///
/// This palette keeps 1 = white and 2 = black (so real glyphs read as
/// white-on-black and stay legible) and paints everything else screaming
/// magenta. That turns the reported "rainbow speckles inside individual
/// glyphs" from a subtle colour judgement into an unambiguous marker: if a
/// glyph has a magenta pixel in it, the font tile's VRAM was clobbered, and
/// its neighbours' worth of magenta tells you which other tile took the hit.
#[cfg(feature = "diag-bgspeck")]
static BG_SPECKLE_PALETTE: Palette16 = {
    let magenta = Rgb::new(255, 0, 255).to_rgb15();
    let mut p = [magenta; 16];
    p[0] = Rgb15::BLACK;
    p[1] = Rgb15::WHITE;
    p[2] = Rgb15::BLACK;
    Palette16::new(p)
};

/// Palette bank 15 for the board and free-form menu screens.
fn text_pal() -> &'static Palette16 {
    #[cfg(feature = "diag-bgspeck")]
    {
        &BG_SPECKLE_PALETTE
    }
    #[cfg(not(feature = "diag-bgspeck"))]
    {
        &TEXT_PALETTE
    }
}

/// Palette bank 15 for the card-detail view.
fn detail_pal() -> &'static Palette16 {
    #[cfg(feature = "diag-bgspeck")]
    {
        &BG_SPECKLE_PALETTE
    }
    #[cfg(not(feature = "diag-bgspeck"))]
    {
        &DETAIL_TEXT_PALETTE
    }
}

/// Text palette for the card-detail view, on bank 15 (the OBJ sprite palette
/// lives in a separate hardware block, so the two never collide). Matches
/// 3DS COL_CARD_OPAQUE dark panel + white text. Entries 7..=15 mirror
/// TEXT_PALETTE so baked texticon tiles render the same in both views.
static DETAIL_TEXT_PALETTE: Palette16 = const {
    let mut palette = [Rgb15::BLACK; 16];
    palette[0] = Rgb15::BLACK;
    palette[1] = Rgb15::WHITE;
    palette[2] = Rgb::new(26, 35, 50).to_rgb15(); // zone fill behind text
    // Entries 3/4/5 mirror TEXT_PALETTE: baked texticons use gold (4) and
    // green (5), and without these any such pixel renders BLACK on detail
    // screens (the "black lines" in ability text 窶・e.g. green hearts).
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

/// Scratch for normalizing baked tiles on sprite-cache misses (see
/// `get_or_upload`). Sized for the largest grid (detail 12x18x64).
/// Single-threaded GBA, synchronous use only 窶・no reentrancy.
const TILE_SCRATCH_LEN: usize = 13824;
static mut TILE_SCRATCH: [u8; TILE_SCRATCH_LEN] = [0; TILE_SCRATCH_LEN];

/// Raw 4bpp board UI tiles (6 tiles x 32 bytes = 192). The baker emits a
/// standard LZ77 stream for it like everything else, but `TileSet::new`
/// needs `&'static` bytes, which a stack buffer can't provide. Decompress
/// once into .bss.
static mut UI_TILES_BUF: [u8; 192] = [0; 192];
static mut UI_TILES_READY: bool = false;

/// Board UI tiles, decompressed on first use. See [`decompress_lz77`].
fn board_ui_tiles() -> &'static [u8] {
    unsafe {
        if !UI_TILES_READY {
            let mut dst = [0u8; 192];
            decompress_lz77(BOARD_UI, &mut dst);
            UI_TILES_BUF = dst;
            UI_TILES_READY = true;
        }
        &*core::ptr::addr_of!(UI_TILES_BUF)
    }
}

/// Reused inflate state.
///
/// `miniz_oxide::inflate::core::DecompressorOxide` is ~10.4KB (three 3456-byte
/// Huffman tables), so it cannot be a local 窶・the GBA stack is a few KB. It
/// lives in .bss and is re-`init`ed per call, so nothing carries between blobs.
///
/// Do NOT replace this with `inflate::decompress_to_vec_zlib`. That helper
/// allocates `Box::<DecompressorOxide>::default()` 窶・this 10.4KB struct 窶・plus
/// an output Vec on EVERY call (`inflate/mod.rs:191-193`). The sprite cache
/// misses often (it is VRAM-bounded and LRU-evicts), so that churn exhausts
/// the 256KB EWRAM heap within seconds of play and the render state falls
/// apart. The low-level `decompress()` below allocates nothing at all.
///
/// `None` until first use, because `DecompressorOxide::new()` is not a `const
/// fn` and so cannot appear in a `static` initializer. The `UnsafeCell` is
/// what lets the GBA's single thread mutate it; a bare `static mut` would need
/// a `&mut` to a mutable static, which is the UB the compiler warns about.
struct ArtInflaterCell(core::cell::UnsafeCell<Option<ArtInflater>>);

// Safe: the GBA is single-threaded and every access is a short, non-reentrant
// borrow that ends before `decompress_art` returns.
unsafe impl Sync for ArtInflaterCell {}

static ART_INFLATE: ArtInflaterCell = ArtInflaterCell(core::cell::UnsafeCell::new(None));

type ArtInflater = miniz_oxide::inflate::core::DecompressorOxide;

/// Fill `dst` from a baked DETAIL blob (a zlib stream).
///
/// Deliberately the ONLY thing that goes through software inflate. The detail
/// portrait is 13,824B and 73% of the art, so it is the one blob where the
/// better ratio is worth a slow decode 窶・and it is only decompressed when a
/// card is actually opened, after which `retain_key` keeps it cached and
/// scrolling is a pure cache hit.
///
/// Everything drawn every frame (board stage/live/wait/hand fronts, menu
/// cards, UI) goes through [`decompress_lz77`] and the BIOS SWI instead. An
/// earlier revision ran miniz_oxide on those too, and the main-phase frame
/// could not finish decoding in time and presented half-drawn BGs: the
/// mulligan screen (a handful of placements) kept up, the board (~28) did not.
///
/// LZ77 is the worse *ratio* choice (see tools/bake_card_art.py) but the
/// front blobs are small (384-1920B), so the absolute loss is modest, and
/// they are the ones that must be fast.
fn decompress_art(src: &[u8], dst: &mut [u8]) {
    use miniz_oxide::inflate::core::inflate_flags::{
        TINFL_FLAG_PARSE_ZLIB_HEADER, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    };
    use miniz_oxide::inflate::core::decompress;
    use miniz_oxide::inflate::TINFLStatus;

    let d: &mut ArtInflater = unsafe {
        let slot = &mut *ART_INFLATE.0.get();
        slot.get_or_insert_with(ArtInflater::new)
    };
    // The state is reused across calls, so it must not carry a half-finished
    // block from the previous blob into this one.
    d.init();

    // TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF is required: without it inflate
    // demands a power-of-two output buffer, and none of our grid sizes are.
    let (status, _in_consumed, out_written) = decompress(
        d,
        src,
        dst,
        0,
        TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    );

    // A malformed blob leaves `dst` as whatever was there. The BIOS LZ77 path
    // this replaced also had no integrity check (it silently produced garbage),
    // so this is no worse 窶・but it must not panic: a panic in this no_std
    // build halts the machine, and one bad blob should cost one blank sprite,
    // not the game.
    if status != TINFLStatus::Done || out_written != dst.len() {
        agb::println!(
            "[ART] inflate FAILED {:?} {}B -> {}B (got {}B); sprite left blank",
            status, src.len(), dst.len(), out_written
        );
        dst.fill(0);
    }
}

/// Fill `dst` from a baked FRONT/UI blob (a BIOS LZ77 stream).
///
/// The per-frame path: native ARM via SWI 0x11, zero allocation, zero software
/// work. `dst` must fit the declared size; callers size it from the grid
/// (`tiles * 64`).
fn decompress_lz77(src: &[u8], dst: &mut [u8]) {
    lz77_decompress_wram(src, dst);
}

impl<'a> Display<'a> {
    pub fn new(mut gfx: Graphics<'a>) -> Self {
        // Bank 15 carries the 4bpp text/UI palette for both BGs. The 256-col
        // card palette lives in OBJ palette RAM via PaletteVramMulti (a
        // separate hardware block), so no per-frame BG palette writes are
        // needed and the backdrop never flashes.
        gfx.set_background_palette(15, text_pal());
        // Wait one VBlank to ensure palette is active before first frame.
        busy_wait_for_vblank();
        // Identify the ROM in the log before anything else, so a capture is
        // never mistaken for the baseline. Diagnostic builds differ from the
        // control in exactly one way each; this line is how you tell them
        // apart on a screenshot.
        crate::diag::announce();
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
            screen: Screen::None,
            priming: 0,
        }
    }

    /// Note which screen is about to push art, and prime agb's sprite pins
    /// when that screen changes.
    ///
    /// agb keeps the previous frame's sprites alive until the *second*
    /// `gfx.frame()` after they were shown (`Oam::frame` swaps them into
    /// `previous_frame_sprites` and only drops them a frame later). OBJ VRAM
    /// is 32KB, of which agb's bump allocator can ever reach 32,704 bytes, and
    /// a full board already wants ~30KB -- so on a switch the outgoing screen's
    /// tiles are still pinned while the incoming screen uploads, and every
    /// `try_to_vram` fails. That is what blanked the board right after the
    /// mulligan: the uploads returned `Err`, `upload_part` yielded `None`, and
    /// the slot simply went empty.
    ///
    /// Two art-free frames are enough. Frame 1 of the new screen drops the
    /// outgoing screen's stale pins; frame 2's `gfx.frame()` frees the last of
    /// them, so the frame after that builds against a completely empty
    /// allocator.
    ///
    /// Switches where the outgoing screen held nothing (board -> actions, or
    /// any menu -> menu) skip priming entirely, so the common moves never cost
    /// a blank frame.
    fn enter_screen(&mut self, screen: Screen) {
        if self.screen == screen {
            return;
        }
        let pinned = self.sprite_cache.vram_bytes();
        if pinned > 0 {
            self.sprite_cache.clear();
            self.priming = 2;
            dlog!("screen switch: priming art, {}B was pinned", pinned);
        }
        self.screen = screen;
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
    ///
    /// Writes a 2x2 footprint, so the caller MUST have checked that
    /// `ty + 1 < ROWS` and `tx + 1 < COLS` first. `set_tile` does not bounds
    /// check: the BG tile number is a 10-bit field, so an out-of-map index
    /// aliases into the next screenblock instead of failing. See `blit_text`.
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
        let mut spent = 0usize;
        let mut draw_ch = |bg: &mut RegularBackground, ch: char, tx: i32, ty: i32| -> i32 {
            let (_, cols) = Self::glyph(ch);
            // Clip on BOTH axes before handing anything to `set_tile`.
            //
            // A 2x2 glyph occupies (tx..tx+2, ty..ty+2), so a glyph whose
            // origin sits on the last column or row still writes one tile
            // past the visible map. That used to be unmitigated on Y: only X
            // was checked, so the `wrap` path in `swap_buffers` 窶・the
            // free-form screen that renders the whole action log 窶・could walk
            // `ty` off the bottom of the 32x32 map. A BG map entry's tile
            // number is 10 bits (GBATEK), so index 1024 and up does not
            // bounds-check anywhere: it aliases into the following
            // screenblock and silently rewrites the other background's
            // tilemap and whatever tile blocks sit after it. No panic, no
            // log line 窶・just progressive screen corruption that worsens as
            // the log grows.
            if ty < 0 || ty + 1 >= ROWS {
                return cols as i32;
            }
            if tx < 0 || tx + cols as i32 > COLS {
                if !wrap {
                    return cols as i32; // clipped: consume without drawing
                }
                return -(cols as i32); // signal wrap needed
            }
            // Tile-pool guard. `set_tile` routes through
            // `VRAM_MANAGER.add_tile`, and the pool is finite: 1,022 tiles
            // total, half of it still held by the previous frame because gc
            // only runs at commit. Overrunning it is not a glitch but
            // `.expect("Ran out of video RAM for tiles")`, i.e. a panic that
            // hands the machine to agb's bitmap-3 panic renderer. That is the
            // main-menu corruption and the deck-select crash.
            //
            // Once the budget is gone we still advance `tx` so the layout does
            // not collapse, we just stop spending tiles.
            if spent >= BG_TILE_BUDGET_PER_FRAME {
                return cols as i32;
            }
            spent += if cols >= 2 { 4 } else { 2 };
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
            // line's end 窶・emitting them starts the new line indented (the
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
    /// frame are simply absent from OAM 窶・no manual clearing, no ghosts.
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
        // Snapshot the frame that was actually presented. Must come after the
        // commit, or OAM still holds the previous frame.
        #[cfg(feature = "diag-shot")]
        crate::shot::maybe_capture(SHOT_SKIP);
        // One priming frame is spent per `gfx.frame()` above, so the outgoing
        // screen's pins are gone by the time the next frame builds. See
        // `enter_screen`.
        if self.priming > 0 {
            self.priming -= 1;
        }
    }

    /// Push all sprite parts of one card placement into `active_sprites`.
    /// `tiles` is the baked 8bpp grid (`grid_w` x `grid_h` tiles); the art is
    /// centred in the `box_w` x `box_h` px box at (`px`, `py`).
    ///
    /// Returns `false` when nothing was drawn (art disabled, priming frame, or
    /// no room in OBJ VRAM) so the caller can degrade to the card number
    /// instead of leaving the slot blank.
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
        rot_ccw: bool,
    ) -> bool {
        // Board stage twins share art but never show together; name the
        // other so a miss evicts it first (no room for both).
        let twin_tag: Option<&str> = match tag {
            "stage" => Some("stagew"),
            "stagew" => Some("stage"),
            _ => None,
        };
        if !crate::diag::art_enabled() {
            return false;
        }
        // Priming frame after a screen switch: agb still pins the outgoing
        // screen's tiles, so any upload here would fail anyway. Skip the
        // allocation entirely and report success so the caller does not paint
        // a card number over a slot that is merely mid-transition.
        if self.priming > 0 {
            return true;
        }
        // Disjoint fields: the cache borrow and the sprite-list push coexist
        // with no intermediate Vec clone.
        match self.sprite_cache.get_or_upload(
            tag, card_no, tiles, grid_w, grid_h, box_w, box_h, parts, flipped,
            dimmed, rot_ccw, px, py, twin_tag,
        ) {
            Some(vrams) => {
                for (vram, &(_, dx, dy)) in vrams.iter().zip(parts.iter()) {
                    let mut obj = Object::new(vram.clone());
                    obj.set_pos((px + dx as i32, py + dy as i32));
                    // A 180-degree rotation is hflip|vflip in OAM attributes,
                    // not pixels. Rasterising it (which this used to do)
                    // cost a separate VRAM upload for every opponent-side
                    // slot: 3 stage + 6 live = 9,984B of byte-for-byte
                    // duplicates, and because `flipped` was also in the cache
                    // key the flipped copy could never share the unflipped
                    // placement's VRAM. The hardware does it for free.
                    if flipped {
                        obj.set_hflip(true);
                        obj.set_vflip(true);
                    }
                    obj.set_priority(SPRITE_PRIO);
                    self.active_sprites.push(obj);
                }
                true
            }
            None => {
                dlog!("dropping card {} (no sprite VRAM)", card_no);
                false
            }
        }
    }

    /// Render the integrated board: sprite card art between the back fill
    /// and the front text/UI layer.
    pub fn render_board_frame(&mut self, frame: &BoardFrame) {
        self.last = self.buf.clone();
        self.enter_screen(Screen::Board);
        // Detail portraits are large (9 parts); evict them before the board
        // re-uploads so the two never share the 32KB OBJ VRAM budget.
        self.sprite_cache.evict_prefix("detail:");
        self.gfx.set_background_palette(15, text_pal());
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(board_ui_tiles(), TileFormat::FourBpp) };
        let e0 = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e0);

        Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e0, &frame.header, 0, 0);
        Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e0, &frame.action_count, COLS - 6, 0);

        // Stage rows: opponent (rotated 180ﾂｰ) then player, with live success
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

    /// [`Display::draw_slot`] with an explicit 180ﾂｰ flag (opponent rows).
    fn draw_slot_flipped(
        &mut self,
        font_ts: &TileSet,
        icon_ts: &TileSet,
        ui_ts: &TileSet,
        e: TileEffect,
        slot: &Slot<'_>,
        x: i32,
        y: i32,
        kind: SlotKind,
        flipped: bool,
    ) {
        let Some(card_no) = slot.card_no.as_deref() else {
            return;
        };
        // Resolve art. Waited stage cards rotate the full stage front 90ﾂｰ
        // CCW to fill the 48px box (like 3DS tapped cards fill their slot);
        // waited hand/live slots keep the small pre-rotated front centred.
        let (tiles, gw, gh, tag, rot_ccw): (&[u8], usize, usize, &str, bool) = if slot.hidden {
            (BACK_FRONT, 3, 2, "back", false)
        } else if slot.waited && matches!(kind, SlotKind::Stage) {
            match self.sprite_cache.find_front(STAGE_FRONTS, card_no) {
                Some(f) => (f.tiles, kind.grid_w(), kind.grid_h(), "stagew", true),
                None => {
                    // Missing baked art must degrade to text, never to an
                    // invisible slot 窶・a silent `return` here once hid
                    // waited cards entirely.
                    dlog!("missing stage front for waited {}", card_no);
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
            match self.sprite_cache.find_front(WAITED_FRONTS, card_no) {
                Some(f) => (f.tiles, WAIT_GRID.0, WAIT_GRID.1, "wait", false),
                None => {
                    dlog!("missing waited front for {}", card_no);
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
            match self.sprite_cache.find_front(fronts, card_no) {
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
        // Face-down slots all draw the SAME `BACK_FRONT` art, but keying them
        // on their own card_no minted one 512B placement per slot 窶・six copies
        // of identical pixels. Key them on a constant instead: the cache then
        // holds one entry (or two, for the flipped opponent row) and the win
        // is handed straight back as VRAM headroom.
        let key_card_no = if slot.hidden { "" } else { card_no };
let drawn = self.push_card(
            tag,
            key_card_no,
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
            rot_ccw,
        );
        // A slot that could not get art still has to say something; a blank
        // box is indistinguishable from "no card here".
        if !drawn {
            Self::blit_line(
                &mut self.ui_front,
                font_ts,
                icon_ts,
                e,
                card_no,
                x,
                y + kind.rows() / 2,
            );
        }
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
    /// bottom-left when shown rotated 180ﾂｰ (opponent row). The cursor marker
    /// shares this tile so it sits on the card, overlapping the badge.
    ///
    /// Stage art's bright border ends well inside the 40px grid (the rest is
    /// dark margin), so its badge sits one tile left of the grid edge 窶・    /// otherwise it floats over padding right of the visible card. Hand/live
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
    /// drive the list live.
    ///
    /// Text-only: no sprites are pushed, so `end_frame` retains the whole
    /// cache (see `end_frame`'s touched gate) and returning to the board is
    /// all cache hits with zero re-upload. Do NOT clear here 窶・the old
    /// clear forced a ~20-card decompress storm on every Actions<->Board
    /// round trip. (Detail/grid screens still reset: the 9-part portrait
    /// genuinely doesn't fit alongside the board in 32KB OBJ VRAM.)
    ///
    /// The engine prints more lines than fit (3 headers + up to 7 actions +
    /// a `..more` line); the window follows the `>` cursor so the list
    /// scrolls fully instead of clipping the selected row away.
    pub fn render_action_text(&mut self) {
        self.last = self.buf.clone();
        self.enter_screen(Screen::None);
        self.gfx.set_background_palette(15, text_pal());
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(board_ui_tiles(), TileFormat::FourBpp) };
        let e = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e);

        Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e, "ACTIONS U/D/A B:Back L/R:pg", 0, 0);
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
            dlog!("actions scroll up to {}", cursor);
        }
        if cursor >= self.action_offset + VISIBLE {
            self.action_offset = cursor + 1 - VISIBLE;
            dlog!("actions scroll down to {}", cursor);
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
        self.enter_screen(Screen::Detail);
        // Board sprites would share the 32KB OBJ budget with the 9-part
        // portrait 窶・evict everything EXCEPT this card's portrait, so
        // scrolling (same art, new offset) is a cache hit with no re-upload.
        match art {
            Some(a) => self.sprite_cache.retain_key(&placement_key(
                "detail",
                a.card_no,
                DETAIL_PX.0,
                DETAIL_PX.1,
                false,
            )),
            None => self.sprite_cache.clear(),
        }
        self.gfx.set_background_palette(15, detail_pal());
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(board_ui_tiles(), TileFormat::FourBpp) };
        let e_text = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e_text);

        if let Some(art) = art {
            dlog!("detail portrait for {}", art.card_no);
            // Pass the baked stream straight through: get_or_upload is the
            // single point that normalizes streams (decompressing here AND
            // there fed pixels back into SWI as a fake stream 窶・the wild
            // jumps on detail open).
            self.push_card(
                "detail",
                art.card_no,
                art.tiles,
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

    /// Indexed detail-art lookup for menus (binary search, no linear scan).
    pub fn find_card_art(&self, card_no: &str) -> Option<&'static CardArt> {
        self.sprite_cache.find_art(card_no)
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

    /// `(bytes of OBJ VRAM held, cached placement count)`.
    /// OBJ VRAM is the binding constraint on this port: card art is 8bpp
    /// (64 B/tile) because MASTER_PAL is 240 colours, and the GBA has only
    /// 32,768 B of sprite character VRAM with no paging. A 1v1 board showing
    /// both players' hands needs ~32,256 B of that, so this number going over
    /// `SPRITE_VRAM_BUDGET` is the single most useful thing to watch while
    /// the board misbehaves. Surfaced on screen by the board view.
    pub fn vram_stats(&self) -> (usize, usize) {
        (self.sprite_cache.vram_bytes, self.sprite_cache.sprites.len())
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
        self.enter_screen(Screen::Menu);

        // Bank 15 holds TEXT_PALETTE (white text on dark blue). The detail
        // view overwrites bank 15, so restore it here.
        self.gfx.set_background_palette(15, text_pal());
        // Menu cards share kinds with the board; detail portraits do not 窶・        // evict those before uploading.
        self.sprite_cache.evict_prefix("detail:");
        let font_ts = unsafe { TileSet::new(&FONT_TILES.0, TileFormat::FourBpp) };
        let icon_ts = unsafe { TileSet::new(&TEXTICON_TILES.0, TileFormat::FourBpp) };
        let ui_ts = unsafe { TileSet::new(board_ui_tiles(), TileFormat::FourBpp) };
        let e_ui = TileEffect::new(false, false, 15);

        self.clear_layers(&ui_ts, e_ui);

        let e = TileEffect::new(false, false, 15);
        let mut ty = 0i32;

        for line in self.buf.split('\n') {
            // `ty + 2 > ROWS` bounded rows but not tiles: each 2x2 Japanese
            // glyph costs 4 tiles out of a 1,022-tile pool that is half-full
            // with the previous frame. MAX_TEXT_ROWS is the real limit.
            if ty + 2 > ROWS || ty / 2 >= MAX_TEXT_ROWS {
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
            } else if let Some(front) = self.sprite_cache.find_front(fronts, &q.card_no) {
                // Menu boxes are exactly the queued tile size (choice grid
                // cards have no wait-gap, unlike board stage slots).
                let (box_w, box_h) = ((q.cols * 8) as usize, (q.rows * 8) as usize);
                let drawn = self.push_card(
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
                if !drawn {
                    Self::blit_line(&mut self.ui_front, &font_ts, &icon_ts, e, &q.card_no, q.x, q.y);
                }
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

/// Implement PlatformUi for Display so menu functions can use it directly.
impl rabuka_engine::game::platform_ui::PlatformUi for Display<'_> {
    fn clear_screen(&mut self) {
        self.clear();
    }
    fn println(&mut self, text: &str) {
        self.println(text);
    }
    fn swap_buffers(&mut self) {
        self.swap_buffers();
    }
    fn poll_input(&mut self) {
        // Not directly available on Display; menu.rs handles input separately
    }
    fn just_pressed_a(&self) -> bool { false }
    fn just_pressed_b(&self) -> bool { false }
    fn just_pressed_up(&self) -> bool { false }
    fn just_pressed_down(&self) -> bool { false }
    fn just_pressed_start(&self) -> bool { false }
    fn wait_vblank(&mut self) {
        busy_wait_for_vblank();
    }
    fn reset_vram(&mut self) {
        self.reset_vram();
    }
    /// NOTE: single-frame renderer, NOT a blocking menu 窶・Display has no
    /// input, so this cannot wait for buttons. UI flows must use
    /// `menu::show_detail_screen` with a real `InputSource` (routing here
    /// flashed one frame and "closed" instantly). Kept only to satisfy
    /// the trait; nothing calls it.
    fn show_detail_screen(
        &mut self,
        _gs: &rabuka_engine::game_state::GameState,
        _art_card_no: Option<&str>,
        header: &[String],
        body: &str,
    ) {
        let lines: Vec<String> = header.iter().cloned()
            .chain(if !header.is_empty() && !body.trim().is_empty() { Some(String::new()) } else { None })
            .chain(Self::wrap_pane(body, 17))
            .collect();
        self.render_card_detail(None, &lines, 0);
    }
}

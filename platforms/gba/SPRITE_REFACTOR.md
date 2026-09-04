# GBA Display: Sprite-Based Card Rendering (as built)

## Layering (GBA: lower number wins; OBJ beats a BG of the SAME number)

| Layer | Type | Priority | Purpose |
|-------|------|----------|---------|
| Front text/UI | `RegularBackground` 4bpp (`ui_front`) | P0 | Text, badges, cursors; cleared with transparent tile 4 |
| Card art | `Object` sprites 8bpp | P1 | All card fronts/backs, multi-sprite per card |
| Back fill | `RegularBackground` 4bpp (`ui_back`) | P2 | Opaque zone fill (`UI_EMPTY`), always behind cards |

One priority number per layer — never share a number between a BG and
sprites you want ordered. `present()` shows back, then sprites, then front.

## Sprites: one card = multiple legal-size Objects

`agb::display::object::Size` only allows 12 sizes
(8/16/32/64 squares + 16x8, 32x8, 32x16, 64x32, 8x16, 8x32, 16x32, 32x64).
There is no 40x48 / 24x32 / 96x144 sprite, so each card is tiled
(`display.rs` `*_PARTS` tables):

| Card | Box px | Parts |
|------|--------|-------|
| Hand 24x32 | 24x32 | 2x S16x32 |
| Stage 40x48 art | 48x48 (pitch 6) | S32x32 + S16x32 + S32x16 + S16x16 |
| Live 24x16 | 24x16 | S32x16 |
| Waited 32x24 | centred in slot box | slot box tiling |
| Detail 96x144 | 96x144 | 6x S32x64 + 3x S32x16 (9 parts) |

Stage boxes are 48px wide at 48px pitch: untapped 40px art centres with a
4px gap each side, and wait-state cards rotated 90° to 48px wide exactly
fill their box — even all-waited rows touch but never overlap (like the
3DS square stage slots).

Upload via `DynamicSprite256::set_pixel` (per-pixel, correct under OBJ
1D mapping for rectangular sizes) — never `data_mut` tile-block copies,
which assume BG tile order. `clear(0)` after `new()`; gutters stay index 0.

## Palette: index 0 is transparent on OBJ

`tools/bake_card_art.py::build_palette` forces palette index 0 = magenta
(`DUMMY_RGB`, rgb15 `0x7C1F`) and index 1 = `PAD_RGB`. Baked art contains
zero index-0 pixels (verified); sprite gutters are filled with 0. If the
bake ever regresses (index 0 = black again), all dark card pixels turn
into holes showing the back fill through. Detail art shares the master
palette — no per-card palette reload, no backdrop flash (BG and OBJ
palettes are separate hardware blocks; writing art colours to the BG
palette does nothing for sprites).

## Opponent flip

180° rotation is pre-rotated into cached pixels (mirror in box space,
keyed `:f`), not `hflip+vflip` — multi-part anchor math stays trivial.

## Rotations (match the 3DS `ctru_shim.c` directions)

C2D positive angles render clockwise on screen (y-down), so:

* 3DS waited `+90°` == PIL `rotate(-90)`: the GBA waited bake and the
  stage wait-state upload (`rot_cw`, tag `stagew`) both rotate CW.
* 3DS portrait-box `-90°` ("other way from the board's waited cards") ==
  PIL `rotate(+90)`: the bake's `maybe_upright` for landscape (live) art
  in hand/stage/detail boxes rotates CCW.

## Cursor

The white triangle shares the gold badge's tile (`badge_tile`: top-right,
bottom-left when flipped 180°), so it sits on the card's right side
overlapping the badge, drawn after badges so it wins.

## VRAM budget (OBJ VRAM = 32 KB)

Board worst case ~30 KB (10 hand x 1 KB + 6 stage x 2.3 KB + 12 live x
0.5 KB); detail portrait ~13.5 KB. Steady state holds exactly the visible
set: entries stamp the frame id on every touch and `end_frame` drops the
rest, so stale flips/dims/rotations never accumulate. Rotated twins evict
each other first (no room for both). Upload uses `try_to_vram`: on OOM the
cache is dropped and retried once, then the card is skipped (debug-logged)
instead of panicking. Cap: 96 cached placements.

## Heap discipline (256 KB EWRAM, 60 fps churn)

Cache hits allocate nothing: keys format into reused buffers, lookups are
`&str`, slices iterate without cloning. Only misses allocate (owned key +
VRAM upload) — transitions, never steady frames. `reset_vram` drops the
cache with NO empty commit (the blank frame was the transition flash;
sprite VRAM frees on drop, BG tiles GC at the next real present).

## Badges

`BOARD_UI` 4 = transparent clear; 5 = edge badge (gold diamond nudged ~3px
right). Stage/menu badges sit one tile left of the grid edge on tile 5 so
the diamond straddles the art's visible border; the centred tile would
float over padding. Cursor marker shares the badge tile.

## Dimmed / selected

Dimmed (unpickable) = checkerboard dither to transparent at upload time
(keyed `:d`), showing the dark back fill — mirrors the 3DS disabled
overlay at zero ROM cost. Selected = gold badge tile on the front layer
(top-right; bottom-left when flipped).

## Success criteria

- [x] Zero `set_tile` calls for card art (sprites only)
- [x] No manual clear loops for art; absent from OAM = invisible
- [x] No VRAM OOM on transitions (evict + try_to_vram + cap)
- [x] < 60 objects/frame typical (limit 128)
- [x] `cargo check` + release ROM build pass

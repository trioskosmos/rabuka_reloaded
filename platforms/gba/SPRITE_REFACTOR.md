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
| Stage 40x48 | 40x48 | S32x32 + S16x32 + S32x16 + S16x16 |
| Live 24x16 | 24x16 | S32x16 |
| Waited 32x24 | centred in slot box | slot box tiling |
| Detail 96x144 | 96x144 | 6x S32x64 + 3x S32x16 (9 parts) |

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

## VRAM budget (OBJ VRAM = 32 KB)

Board worst case ~30 KB (10 hand x 1 KB + 6 stage x 2.3 KB + 12 live x
0.5 KB); detail portrait ~13.5 KB. The two never coexist: `render_board_frame`
evicts `detail:*` keys, `render_card_detail` / `render_action_text` clear
the cache, `reset_vram` clears + commits an empty frame. Upload uses
`try_to_vram`: on OOM the cache is dropped and retried once, then the card
is skipped (debug-logged) instead of panicking. Cap: 96 cached placements.

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

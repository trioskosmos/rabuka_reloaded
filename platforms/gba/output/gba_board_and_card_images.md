# GBA: Card Images & a Proper Board — Research & Plan

Status: **implemented (Phase 1 + Phase 2)** — see §11 Implementation Status.
Audience: anyone building the GBA front-end. Companion to the 3DS
`docs/3ds/VISUAL_DESIGN.md`.

---

## 1. The ask

The GBA port currently renders a full-screen **text list of actions** (see
`platforms/gba/src/display.rs` + `engine/src/game/platform_ui.rs`). The 3DS has
a graphical board with card art (`draw_card_image`, `CardAtlas`, `render_board`).
The goal is to bring a real board to the GBA like a GBA Yu-Gi-Oh! game: cards
sitting in zones, a selected card shown enlarged, things moving around — instead
of a black screen of text.

This doc lays out the hardware realities, the options for card art, a concrete
board layout, and a phased plan. It is deliberately not a full spec: several
decisions (below) need the user's call first.

---

## 2. Current state

| | GBA | 3DS |
|---|---|---|
| Board | text action list only | graphical `render_board` |
| Card art | none | WebP → PNG(192px) → tex3ds `.t3x` atlases → `CardAtlas` |
| Renderer | tiled `RegularBackground` text | top-screen queues + sprites/bitmaps |
| Input | D-pad + A/B + L/R | touch + buttons |

3DS image pipeline (`platforms/3ds/scripts/convert_cards.py`):
`web_ui/img/cards_webp/*.webp` → resized PNG (long edge 192) →
`romfs/cards/*.t3x` (tex3ds) → `cards_manifest.json` (card_no → atlas+index).

Source art lives in `web_ui/img/cards_webp`. `cards/images/BP07`, `NSD02` are
currently **empty** (not a usable source today).

---

## 3. GBA hardware constraints (the hard limits)

- **Screen:** 240×160 px.
- **VRAM:** 96 KB total. Tile modes (0–2): 64 KB background + 32 KB sprite
  tiles. Bitmap modes use VRAM differently.
- **Backgrounds:** up to 4 regular backgrounds (Mode 0), tile-based, palette
  driven. 4bpp = 16 colors per palette × 16 palettes (256 total per BG plane);
  8bpp = a single shared 256-color palette.
- **Sprites/objects:** 128 max. Sizes 8×8 → 64×64 (regular), up to 128×128 with
  affine scaling/rotation (affine objects cost extra OAM slots, so far fewer).
  Object tile VRAM is only 32 KB. Object palettes: 512 bytes (256 colors;
  16 palettes × 16 for 4bpp, one 256-color table for 8bpp).
- **Color:** backgrounds and sprites are **palette-indexed** — you cannot
  display arbitrary RGB like the 3DS. All card art must be **quantized** to a
  limited palette, and art sharing the screen shares palette space.
- **ROM:** carts up to 16–64 MB. Fine for baking many card images; the binding
  constraint is **VRAM + palettes at runtime**, not ROM size.

### The core problem
Card art is full-color. The GBA can only show a handful of colors at once.
Real GBA card games ship heavily down-sampled, low-res card art with a shared
palette. This is the single biggest design constraint.

---

## 4. What "a board" means for this game (engine → screen)

From the engine (`engine/src/core/zones.rs`, `constants.rs`, `player.rs`), each
player owns:

- **Stage:** 3 slots (left / center / right) — `STAGE_SIZE = 3`, plus
  `under_cards` stacked beneath each.
- **Hand:** a pile of cards.
- **Energy zone:** up to `MAX_ENERGY_CARDS = 12`.
- **Main deck:** draw pile.
- **Waitroom:** discard/wait pile.
- **Success live card zone:** up to `VICTORY_CARD_COUNT = 3` — the win condition.
- **Live card zone:** up to `MAX_LIVE_CARDS = 3` (during live set phase).

So a GBA board must surface (mirrored for both players): 3 stage slots, hand,
energy, deck, waitroom, and the success/live zone. Not all need art —
deck/waitroom/hand can be **piles** (stack icon + count), while stage and
success/live are the "cards in play" that benefit most from being actual cards.

### 4.1 Card ↔ action linkage (adapted from 3DS)

The 3DS board isn't just decorative — it is *action-aware* (`docs/3ds/VISUAL_DESIGN.md`):
- Cards that currently have **valid actions** get a gold border / highlight.
- The action list groups actions **by card** (e.g. "Play member X", "Use ability
  on Y"); valid-target cards are highlighted on the board.
- Selecting a card surfaces the actions relevant to it.

The GBA has **no touch**, so we emulate this with a **cursor**:
- D-pad moves a cursor across zones/hand; the highlighted card is the "focused"
  card.
- Cards with valid actions are marked (gold border/tint) so the player can see
  what's actionable at a glance.
- The bottom bar shows the actions for the focused card (or the global action
  list when no card is focused), preserving today's `PlatformUi` flow.

This keeps the existing text action list as the driver of what you can do, while
the board shows *where* those actions apply — exactly the 3DS relationship,
minus touch.

---

## 5. Card images — options and tradeoffs

Scope insight: the GBA only ever loads the **two selected decks'** cards
(`run_embedded_game` → `load_deck_cards`). We can bake card art for all decks
into ROM but **only load the active match's cards into VRAM**. So at any moment
we need sprites/tiles for ~50–100 cards, not all 2526. That makes full per-card
art feasible in ROM and the active set small enough for VRAM.

### 5.1 Palette strategy (decide first)
- **(a) 8bpp shared palette** for all card sprites: one 256-color table for the
  whole game's card art. Consistent, but a 256-color global palette is a
  compromise for all cards at once.
- **(b) 4bpp per-card 16-color palettes**: each card gets up to 16 colors from
  the 16-slot object palette. Better fidelity *per card*, but 16 colors is
  crude and 16 palette slots must be managed.
- **(c) 8bpp shared palette for card BACKS** (uniform) + **4bpp per-card** for
  fronts. Common hybrid.

### 5.2 Options for rendering a card
- **A. Card backs only + text labels.** All in-play cards are the same card-back
  sprite/tile; the name/label is text under it. Cheapest, cleanest, zero art
  quantization. Great starting point; a real Yu-Gi-Oh board shows backs with
  tiny face-down-card look. Selected card revealed via detail view.
- **B. Tiled-background mini cards.** Draw card fronts into a background
  tilemap (each card = a cluster of tiles in a BG). Whole field as one BG.
  No object cap, but repositioning/moving cards means rewriting BG tiles (like
  the text renderer does today). Good for a mostly-static field; weak for
  "cards moving around".
- **C. Sprite cards (recommended for "moving around").** Each card = an agb
  `DynamicSprite16/256` object with `set_position` (agb `object::sprites`).
  Native to how GBA Yu-Gi-Oh handles a board: you move a card by moving its
  sprite. Caveats: 128-object cap and 32 KB sprite VRAM mean you show a subset
  (stage + hand row + selection) and recycle sprites for piles.
- **D. Full bitmap detail view (Mode 3/5).** For the **selected** card, show the
  full-color art big (Mode 3 = 240×160 16bpp, or Mode 5 160×128 15bpp). One
  card at a time, no palette quantization of the displayed art beyond 15-bit.
  This is the "wow" close-up and sidesteps palette limits for the important
  reveal.

### 5.3 Recommendation (phased, de-risks each step)
1. **Phase 1 — board + card backs + text (Options A + C).** Field rendered on a
   tiled background; in-play cards as a shared card-back sprite; names/counts as
   text. Navigation preserved (action list can become a bottom overlay).
2. **Phase 2 — detail view (Option D).** R (card stats) → full bitmap close-up
   of the selected card's art with the stat/ability text. Real visual payoff,
   minimal risk.
3. **Phase 3 — on-board fronts (Option C + 5.1b/c).** Bake per-deck card fronts
   as 4bpp sprites with per-card palettes (or shared 8bpp). Replace backs with
   fronts for revealed cards.
4. **Phase 4 — polish.** Highlight/move animation via affine sprites, tap-to-move
   feedback, animated selection.

---

## 6. Reference: how GBA Yu-Gi-Oh / similar games do it

Real GBA card games (Yu-Gi-Oh! World Championship 2006, Sacred Cards) use:
- A **Mode 0/1 tiled field background** with zone outlines.
- **Small card sprites** (≈32×48) placed in zones; face-down = shared card back.
- A **selected-card panel** in a corner showing enlarged art + stats (often a
  separate background layer or a bitmap region).
- **Cards move by updating sprite positions** (smooth, feels alive).
- Art is **pre-quantized at build time** — never at runtime.

This validates the C+D combination: sprites for the field, a bitmap/panel for
the selected card. Homebrew GBA card projects confirm the object cap is workable
if piles/counters are drawn as stacked sprites or small tiles rather than one
sprite per physical card.

---

## 7. Proposed 240×160 board layout

Mirrors the 3DS zone stack (`docs/3ds/VISUAL_DESIGN.md`), compressed to the
160px-tall GBA screen, with a cursor instead of touch. Card ≈ 34×48 (stage),
≈ 34×28 (hand/live).

```
 y 0 ┌────────────────────────────────────────────┐
     │ Turn 3 · MAIN · P1(AP)   P1H:5 E:4  P2H:4  │  header (h=16)
 y16 ├────────────────────────────────────────────┤
     │ OPP success  [ok][ok][ ]   OPP wait:3       │  h=16
 y32 │    [P2 stage] [P2 stage] [P2 stage]  L/C/R  │  opponent members
 y64 ├─────────────── field divider ──────────────┤
 y80 │    [P1 stage] [P1 stage] [P1 stage]  L/C/R  │  player members
     │    (energy/under count under each)          │
 y112│ P1 E:[▪][▪][▪][▫]  deck:12  wait:2  suc:1   │  energy + piles
 y128│ [P1 hand ......................]  (scroll)  │  hand row (h=20)
 y148│ ────────────────────────────────────────────│
 y150│ > Play [LL-001] to stage center  (cursor)   │  action bar (h=10)
 y160└────────────────────────────────────────────┘
```

- **Cursor** highlights a card in any zone; valid-action cards get a gold tint.
- **Selected card** is enlarged in a Mode 3/5 detail view (Phase 2).
- **Piles** = stacked card-back sprites + count label.
- **Hand** scrolls horizontally; its highlighted card shows in the action bar.
- The **action bar** (bottom) is the existing text action list — it drives
  input, while the board shows where each action applies (see 4.1).

3DS zone ordering (header → live → stage+utility → energy → hand) maps directly;
we fold "utility" (deck/wait/success counts) into the energy row because the GBA
is one 240×160 screen instead of two.

---

## 8. Phased implementation plan (each phase independently shippable)

**Phase 1 — field + backs + text (board exists, action-aware)**
- [x] Board view: header, P1/P2 stats, stage + hand zones with real card numbers.
- [x] Cursor: D-pad moves across stage ⇄ hand (2D cursor); focused card marked `>`.
- [x] Action view: full-screen action list (Up/Down + A), toggled with **Select**.
- [x] Card↔action linkage groundwork: R on a focused board card opens its detail.
- [x] Input: Select toggles Board ⇄ Action; board view consumes D-pad so it doesn't
      fight the action list; R = detail in board view, L/R = text/stats in action view.

**Phase 2 — selected-card detail (art appears)**
- [x] `tools/bake_card_art.py`: reuse the 3DS `.card_png_cache/*.png` + manifest
      (card_no = PNG basename, no re-deriving the mapping); resize to 96×128,
      quantize to 240 colours, emit `platforms/gba/src/card_art_gen.rs`.
- [x] 8bpp background art renderer (`display.rs::render_card_detail`) with text on
      palette bank 15 (reserved) so art and text never collide.
- [x] R on the focused board card opens its art + name + stat line; A/B/L/R closes.
- [x] `build_gba.bat` runs the art bake (via `py -3` for PIL).

**Phase 3 — on-board fronts (real cards)**
- [ ] Bake per-deck fronts as 4bpp sprite tiles + per-card 16-color palettes
      (or shared 8bpp) into ROM.
- [ ] Swap card-back sprite for front when a card is face-up.
- [ ] Load only the active match's cards into sprite VRAM.

**Phase 4 — motion & polish**
- [ ] Affine sprites for smooth place/move/attack animations.
- [ ] Selection glow, zone highlighting, count badges.

Build hooks: new python bake (mirror `bake_font_tiles.py` / `bake_deck_cards.py`
pattern), invoked from `platforms/gba/build_gba.bat`.

---

## 9. Decisions & current direction

Recorded decisions (user-confirmed):
- **Art source:** `web_ui/img/cards_webp` (same as the 3DS pipeline).
- **First deliverable:** Phase 1 (field + card backs + Mode 3/5 selected-card
  detail), then on-board fronts later.
- **Board ↔ action list:** not a replacement — a **bottom action bar** tied to
  the focused card, mirroring the 3DS card↔action relationship (§4.1).
- **Palette strategy:** look to the 3DS visual design for ideas and adapt.
  Note: the 3DS is full RGB (no palette limit), so its *color language* (zone
  boxes, gold highlight for actionable, color-coded action types) carries over,
  but the GBA still needs a **quantized palette** for sprite art — default
  recommendation remains per-card 4bpp (16-color) fronts over a shared card
  back, unless a shared 8bpp proves good enough.

Still open:
- Card size on board (~34×48 recommended).
- Whether Phase 3 fronts are 4bpp-per-card vs shared 8bpp (prototype both).
- How much of the 3DS color palette (gold/blue/green/pink/purple) to reproduce
  with the limited 4bpp per-background palettes.

---

## 10. References (code paths)

- GBA renderer: `platforms/gba/src/display.rs`, `bin/rabuka_gba.rs`
- GBA font bake: `tools/bake_font_tiles.py`, `platforms/gba/assets/`
- GBA board layout/cursor: `platforms/gba/src/board.rs`
- GBA card art bake + data: `tools/bake_card_art.py`, `platforms/gba/src/card_art_gen.rs`
- 3DS board/atlas: `platforms/3ds/src/game/render.rs`, `ui/grid.rs`,
  `ui/card_atlas.rs`, `scripts/convert_cards.py`
- 3DS design doc: `docs/3ds/VISUAL_DESIGN.md`
- Zone model: `engine/src/core/zones.rs`, `core/constants.rs`, `core/player.rs`
- Shared UI flow: `engine/src/game/platform_ui.rs`
- agb sprite/object API: `~/.cargo/registry/.../agb-0.25.0/src/display/object/`

---

## 11. Implementation Status

What is now built and running on the GBA ROM.

### 11.1 Board view vs Action view (one-screen adaptation of the 3DS two screens)

The 3DS uses two screens (board on top, actions on bottom). The GBA has one
240×160 screen and no touch, so the two are separated into **full-screen views
toggled with Select**:

- **Board view** (default): header, P1/P2 stat lines, P2 stage, P1 stage, and
  hand — each showing real card numbers. A 2D cursor (`>` marker) moves with
  the D-pad: Up/Down switches between the stage row and hand row, Left/Right
  moves between slots. `R` opens the focused card's art + stats.
- **Action view**: the full action list (header + actions with ` >` markers).
  Up/Down navigates, `A` executes, `L` = action text, `R` = action card stats.
  `Select` returns to the board.

Engine integration: `PlatformUi::render_board` now returns `bool` — `true` when
the renderer consumed the frame's input (board view handles the whole D-pad
itself, so Up/Down don't double-fire the action list); `false` in action view so
the normal engine action loop drives it. Default (`false` + `swap_buffers`)
keeps the DS/PSP/SNES text ports unchanged.

### 11.2 Card art

- `tools/bake_card_art.py` reuses the 3DS pipeline output — `.card_png_cache`
  PNGs and `cards_manifest.json`. Because every `cards.json` card_no IS the PNG
  basename (and `bake_deck_cards.py` already resolves deck numbers to
  `cards.json` numbers), there is **no card_no→image mapping to re-derive**.
- Bakes the 182 deck-referenced cards to 96×128 8bpp tiles + 240-colour rgb15
  palette (`card_art_gen.rs`).
- `display.rs::render_card_detail` shows the art on an 8bpp background with the
  stat text on palette **bank 15** (indices 240–255 reserved) so art (0–239)
  and text never collide.
- Triggered by **R** in Board view on the focused card; closes on A/B/L/R.

### 11.3 Font glyph fixes

- `tools/bake_font_tiles.py`: clamped glyph placement into the 16×16 cell and
  added a blank→Noto fallback. This fixed short glyphs (`{`, `}`, `|`, `_`) that
  were previously baked as blank tiles (vertical centering pushed them out of
  the cell), so `{{icon_*.png|X}}` markers in ability text render.

### 11.4 Remaining (from the plan)

- Phase 3 — on-board card **fronts** (replace the number markers with real card
  sprites in the field).
- Actionable-card highlighting (cards that currently have valid actions).
- Cleaner board graphics (zone boxes / card backs) now that the view separation
  is in place.

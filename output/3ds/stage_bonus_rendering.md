# Stage Card Bonus Rendering — 3DS Implementation Plan

## Current State

### Web UI (JS, ~130 lines in CardRenderer.js:1190-1295)
- Creates `.bonus-badge` divs overlaid on stage/live cards
- Shows 4 types: additive (`+N`), set/override (`N`), trigger icons (ability gained), heart transform
- Icon images: `icon_blade.png`, `heart_XX.png`, `icon_score.png`, `icon_energy.png`, `jyouji.png`, `live_success.png`, etc.
- All data comes pre-computed from the engine's `CardDisplay` JSON

### Engine (Rust — already complete)
- `GameModifiers` in `engine/src/core/game_modifiers.rs` stores per-card modifier maps:
  - `blade_modifiers: HashMap<i16, ModifierEntry>`
  - `heart_modifiers: HashMap<i16, HashMap<HeartColor, ModifierEntry>>`
  - `score_modifiers: HashMap<i16, ModifierEntry>`
  - `cost_modifiers: HashMap<i16, ModifierEntry>`
- `card_to_display_full()` in `engine/src/game/display.rs` serializes all bonus fields into `CardDisplay`:
  - `bonus_blade`, `bonus_hearts`, `bonus_score`, `bonus_cost` (additive)
  - `set_blade`, `set_hearts`, `set_score`, `set_cost` (absolute overrides)
  - `bonus_triggers` (ability gained icons: jyouji, live_success, live_start, toujyou)
  - `heart_transform`
- `ModifierEntry` has `additive` and `set` fields; `total()` returns `set` if set, else `additive`
- **No engine changes needed**

### 3DS (current — no bonus badges)
- `CardSlot` struct in `ctru_shim.c:67` has NO bonus fields — only `stat_text[128]` for live cards
- `_3ds_board_set_stage(i, active, atlas, idx, landscape, tapped)` — only sets card image + tapped state
- `_3ds_board_set_live_stats(i, score, stat_text)` — sets score + need hearts for live cards
- Live cards render stat texticons at the **bottom** of the card (`_3ds_draw_label_icons`)
- **Stage cards show zero bonus information**

## Implementation Plan

### 1. ctru_shim.c — Add bonus field to CardSlot (~2 lines)
```c
typedef struct {
    bool active;
    char atlas[64];
    int index;
    bool landscape;
    bool tapped;
    bool flipped;
    int score;
    char stat_text[128];
    char bonus_text[128]; // NEW: icon markup for bonus badges
} CardSlot;
```

### 2. ctru_shim.c — Add FFI function (~5 lines)
```c
void _3ds_board_set_stage_bonuses(int i, const char* bonus_text) {
    if (i < 0 || i >= 3) return;
    if (bonus_text) {
        strncpy(p_board.stage[i].bonus_text, bonus_text, 127);
        p_board.stage[i].bonus_text[127] = '\0';
    } else {
        p_board.stage[i].bonus_text[0] = '\0';
    }
}
```

### 3. ctru_shim.c — Render bonuses on stage cards (~10 lines)
In `draw_section()`, after `_3ds_draw_card_at()` for each stage card, draw bonus texticons **on top** of the card (top-left corner):
```c
if (pb->stage[si].active && pb->stage[si].bonus_text[0]) {
    _3ds_draw_label_icons(pb->stage[si].bonus_text, st_x + 3, sy + 3, COL_GOLD, 0.35f);
}
```

### 4. rabuka_3ds.rs — Compute bonus markup for stage cards (~30 lines)
After `fill_player_board!`, iterate each player's 3 stage cards, look up modifiers from `gs.mods`, build texticon markup string:
- `icon_blade.png;+N` for blade bonus
- `heart_XX.png;+N` for heart bonuses
- `icon_score.png;+N` for score bonus
- `icon_energy.png;+N` for cost bonus
- `jyouji.png;` for trigger badges (no number)
- Concatenate into one string, pass to `_3ds_board_set_stage_bonuses()`

### Overflow Handling
- `_3ds_draw_label_icons` already handles overflow — draws texticons left-to-right and stops at string end
- For too many bonuses, clip — only render what fits within the card width
- No scrolling needed
- Format uses existing `{icon;width;label}` markup from `card_stat_line()`

## JS vs Rust Summary

| Component | Language | Lines | Status |
|-----------|----------|-------|--------|
| Modifier storage + calculation | Rust | engine | Done |
| Display serialization (CardDisplay) | Rust | engine | Done |
| Bonus badge DOM creation | JS | CardRenderer.js | Exists |
| 3DS CardSlot.bonus_text | C | ctru_shim.c | Needs ~2 lines |
| 3DS _3ds_board_set_stage_bonuses | C | ctru_shim.c | Needs ~5 lines |
| 3DS stage bonus rendering | C | ctru_shim.c | Needs ~10 lines |
| 3DS bonus computation + markup | Rust | rabuka_3ds.rs | Needs ~30 lines |

**Total new code: ~50 lines** (all in 3DS platform, zero engine changes)

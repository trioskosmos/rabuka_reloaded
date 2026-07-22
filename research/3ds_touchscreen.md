# 3DS Touchscreen Implementation

## Hardware

- Bottom screen only is touch-sensitive (320×240 pixels)
- Read via `hidTouchRead()` → `touch_pos.px, touch_pos.py`
- Non-blocking: returns last known state, no interrupt needed
- Active stylus or finger, single-point only (no multi-touch)

## Current 3DS Port Implementation

### C Shim (`ctru_shim.c`)

- `_3ds_scan_input()` calls `hidTouchRead()` each frame, stores in global `touch_pos`
- `_3ds_touch_down()` returns `true` if `touch_pos.px != 0 || touch_pos.py != 0`
- `_3ds_touch_read(px, py)` writes `touch_pos.px, touch_pos.py` to the provided pointers

### Rust (`rabuka_3ds.rs`)

**Touch handling flow** (lines 1951–2079):
1. Each frame, if `_3ds_touch_down()` returns true:
   - `touch_tap_count += 1`
   - Read coordinates via `_3ds_touch_read(&mut tx, &mut ty)`
2. **Phase 1 — Board zone tap** (lines 1995–2079, general case):
   - If touch is within a board zone (hand, stage, live energy), set `viewing_card` to show card details on top screen
   - Tapping the same card again dismisses the detail
3. **Phase 2 — Action overlay tap** (lines 1958–1993):
   - Only fire if `!cli_mode && ty < 240 && !acts_cache.is_empty()`
   - Computes the overlay region: right-aligned, 180px wide, bottom of screen
   - Maps touch Y to action list item, sets `cur` to the tapped action's flat index
   - Scroll indicators (▲/▼) are handled (tap above/below the visible list scrolls)

**Current issues:**
- The Phase 2 overlay region is computed in Rust but RENDERED on the top screen (via `_3ds_top_queue_text`) — the bottom screen board rendering doesn't include the overlay panel
- The C overlay API (`_3ds_board_set_action_overlay_state`, `_3ds_board_set_action_overlay_text`) exists but is never called from Rust — `overlay_count` is always 0, so the C renderer never draws the panel
- The Rust overlay coordinate math (ox=138, 180px wide, 16px per line) does NOT match the C overlay rendering dimensions (right-aligned, 210px wide, 24px per line)

## C Overlay Infrastructure (Unused)

**Data structures** (`ctru_shim.c`, lines 146–151):
```c
#define MAX_OVERLAY_LINES 16
#define OVERLAY_LINE_LEN  48
static char  overlay_lines[MAX_OVERLAY_LINES][OVERLAY_LINE_LEN];
static int   overlay_count = 0;
static int   overlay_selected = -1;
static int   action_idx_map[MAX_OVERLAY_LINES];
```

**API functions** (`ctru_shim.c`, lines 378–410):
| Function | Purpose |
|----------|---------|
| `_3ds_board_set_action_overlay_state(count, selected)` | Set number of lines + which is selected |
| `_3ds_board_set_action_overlay_text(index, text)` | Set text for one overlay line (max 48 chars) |
| `_3ds_board_set_overlay_action_idx(line, action_idx)` | Map display line → flat action index |
| `_3ds_board_get_overlay_action_idx(line)` | Reverse lookup: display → flat index |
| `_3ds_board_get_overlay_selected()` | Read which line is currently selected |

**Rendering** (`ctru_shim.c`, lines 760–777):
- Only draws when `overlay_count > 0`
- Right-aligned dark panel (210px wide, 24px per line) at bottom of bottom screen
- Selected line has green tint + `>` prefix
- Touch on a line updates `overlay_selected` and sets `overlay_touched = true`

**To activate the bottom-screen overlay:**
1. Replace top-screen action list rendering (lines 2872–2963) with calls to populate the C overlay API
2. Match the overlay dimensions (210px wide, right-aligned, 24px per line) or update both to match
3. Read result via `_3ds_board_get_overlay_selected()` or `_3ds_board_get_overlay_action_idx()`

## Stage Area Selection (Left/Center/Right)

**Current approach** (just added):
- PlayMemberToStage actions are grouped by card in `display_order` via `group_areas[]`
- DPAD LEFT/RIGHT cycles the selected area
- The selected area shows `[L:cost]` while others show ` L:cost `
- Press A to execute with the currently selected area

**Future improvement** (web server reference):
- The web server renders 3 separate clickable buttons per card: Left/Center/Right
- Each button shows the area name + cost after reductions
- Double-baton-touch pairs are shown as a separate row of buttons
- On the 3DS, this could be implemented as:
  - Bottom-screen action overlay with touch areas for L/C/R
  - Or visual stage slots on the bottom screen that the player taps directly

## Text Wrapping

**Current implementation:**
- CLI mode: `_3ds_text_add_top()` appends to a text buffer, rendered line-by-line by the C layer
- GUI mode: `_3ds_top_queue_text(x, y, color, scale, text)` renders a single line at position (x, y)
- The C text renderer (`draw_text` in ctru_shim.c) uses `fontCalcGlyphPos` and does NOT do automatic word wrap

**Issues:**
- Long action descriptions overflow past the 400px top screen width
- The "card detail" view truncates at 28 characters by taking `.chars().take(28)`
- Ability text (`source_ability`) is already truncated to 28 chars in the action list
- The bottom screen overlay has 48-char limit per line

**Fixes needed:**
- Manual wrap: split long lines at a space near the 400px boundary before passing to `_3ds_top_queue_text`
- Or: render multi-line text by splitting on spaces and calling `_3ds_top_queue_text` for each fragment
- The CLI mode text buffer supports newlines; line splitting can use `\n` insertion

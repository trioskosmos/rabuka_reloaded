# Changes to re-apply to working tree (git restore state)

# Changes made so far

## Rust: `engine_3ds/src/bin/rabuka_3ds.rs`

### 1. Step::Play — add hand_offset + viewing_card fields
- Added `usize` (hand_offset) and `Option<i16>` (viewing_card_id) to Play variant (line 208-209)

### 2. Step::Play(gs, ...) creation — add initial values
- Line 566: `Step::Play(gs, 0, Vec::new(), true, true, atlas, vs_ai, false, 0, None)`

### 3. Step::Play destructuring — add mut hand_offset, mut viewing_card
- Added `mut hand_offset, mut viewing_card` after `mut detail_mode`

### 4. AI block — fix is_ai_turn
- `is_ai_turn = *vs_ai && gs.active_player().id != gs.player1.id`
- Always clears `acts_cache` + sets dirty

### 5. DPAD — hand scrolling
- DPAD RIGHT (0x10): scroll right
- DPAD LEFT (0x20): scroll left
- Clamped by `visible_hand_slots()`

### 6. Touch handler — poll via _3ds_touch_down()
- Hit-tests hand/stage/live zones
- Toggles viewing_card (same card dismisses, empty dismisses)
- Uses correct slot widths: hand=card_h*0.711, stage=card_h*0.711, live=card_h*1.41

### 7. Hand rendering — visible_hand_slots based
- `visible_hand_slots()` computes ~5-6 slots using `card_h*0.711 + 2` spacing
- Renders from `hand_offset`

### 8. Top screen — viewing_card / ability queue display below stats
- Shows [T] next to P1/P2 when viewing_card is active
- Shows card name + abilities (wrapped at 36) or ability_queue text

### 9. Action list — multi-line scrollable
- max_vis=6, scroll window centered on cursor
- wrap_text splits per line, individual _3ds_text_add_top calls
- Shows ▼▲ indicators for overflow
- "AI is thinking..." replaces action list during AI turns

### 10. Step::Play return — passes hand_offset, viewing_card
- Line 1150-1161: full Step::Play reconstruction

## C: `engine_3ds/src/ctru_shim.c`

### 11. Added _3ds_keys_held, _3ds_touch_read, _3ds_touch_down
- `_3ds_keys_held()` calls `hidKeysHeld()`
- `_3ds_touch_read()` calls `hidTouchRead()`
- `_3ds_touch_down()` checks px/py != 0 (not KEY_TOUCH bit)

### 12. Added board zone query functions
- `_3ds_board_set_section_rect(y0, h, opponent)` — stores for hit-testing
- `_3ds_board_get_zone_y(zone_type)` — live=0, stage=1, energy=2, hand=3
- `_3ds_board_get_zone_h(zone_type)` — returns zone height (same % as draw_section)

### 13. Card aspect ratio preservation
- `_3ds_draw_card_at` now uses uniform scale (min(sx,sy)) and centers image in slot
- All zones compute slot width from card height × correct ratio (0.711 portrait, 1.41 landscape)
- Stage: `st_slot_w = (stage_h-4) * 0.711`
- Hand: `h_slot_w = (hand_h-4) * 0.711`  (was `hand_h * 0.65`)
- Live: `live_slot_w = (live_h-4) * 1.41` (was hardcoded 40/32)
- Energy: `e_w = e_sz * 0.711` (was `e_sz * 0.7`)

### 13. _3ds_render_board — calls _3ds_board_set_section_rect
- Each draw_section is preceded by `_3ds_board_set_section_rect(y0, h, opponent)`

# Still needing verification/test on hardware:
1. DPAD LEFT/RIGHT hand scrolling (0x10/0x20 codes)
2. Touch screen (using _3ds_touch_down direct poll, no KEY_TOUCH)
3. Card aspect ratios look correct (no stretching, proper proportions in all zones)
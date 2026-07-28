# Duplicate Code Cleanup Plan

Generated from codebase scan on 2026-07-28.

## Completed

### Phase 1: Engine-internal quick wins
- [x] 1a. Remove `parse_heart_color` wrapper in `zones.rs` → 27 callers updated to `crate::card::parse_heart_color`
- [x] 1b. Fix `prevent_baton` duplication in `display.rs` → compute once, assign to both fields
- [x] 1c. Add `HeartColor::as_str()` to `card.rs`
- [x] 1d. Remove `heart_color_to_str` in `display.rs` → replaced with `.as_str()`
- [x] 1g. Move `DECK_CARD_FILES` to engine `deck_parser.rs` → removed from 4 platforms

### Phase 2: Move shared functions to engine
- [x] 2a. `settle_auto` → engine `game_setup.rs`, replaced in Wii/DC/DS/PSP (3DS/DS variants left as-is due to OS yield / unused display param)
- [x] 2b. `execute_action` → engine `game_setup.rs`, replaced in Wii/DC/DS/PSP
- [x] 2c. `load_two_decks` → engine `deck_parser.rs`, replaced in PSP/DS/Wii/DC
- [x] 2d. `test_ai_vs_ai` → engine `game_setup.rs`, replaced in 3DS/PSP/DS

### Phase 3: PlatformUi trait
- [x] 3a. Created `PlatformUi` trait in `engine/src/game/platform_ui.rs` (no_std-gated)
- [x] 3b. Shared functions: `ai_turn`, `show_result`, `select`, `menu_select`
- [ ] 3c. `handle_choice` — not yet ported (biggest, ~180 lines per platform, different SelectCard/SelectTarget handling)
- [ ] 3d. `human_turn` — not yet ported (~70 lines per platform, different visible count / truncation)
- [ ] 3e. Implement `PlatformUi` for Wii/DC/DS/PSP Display+Input pairs
- [ ] 3f. Replace platform-specific `ai_turn`/`show_result`/`select`/`menu_select` with trait calls

## Not consolidated (intentionally skipped)
- 1e. `DcHasher`/`PspHasher`/`SimpleHasher` — different multipliers (31 vs 131), different visibility (pub(crate) vs local), different use cases
- 1f. Platform `DeckEntry` (name + cards) vs engine `DeckEntry` (card_no + quantity) — different structs
- 3DS `settle_3ds` — has OS yield logic (`_3ds_main_loop`), can't use generic version
- Engine bin `settle_automatic` — uses explicit phase matching instead of `is_automatic_phase`

## Remaining work
- Implement `PlatformUi` for each platform
- Port `handle_choice` and `human_turn` to shared functions
- Replace all platform-specific game loop functions with trait calls
- Remove dead code from platforms (unused imports, duplicate constants)

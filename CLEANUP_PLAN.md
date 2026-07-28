# Duplicate Code Cleanup Plan

Completed 2026-07-28.

## Done

### Phase 1: Engine-internal quick wins
- [x] 1a. Remove `parse_heart_color` wrapper in `zones.rs` → 27 callers updated
- [x] 1b. Fix `prevent_baton` duplication in `display.rs`
- [x] 1c. Add `HeartColor::as_str()` to `card.rs`
- [x] 1d. Remove `heart_color_to_str` in `display.rs`
- [x] 1g. Move `DECK_CARD_FILES` to engine `deck_parser.rs`

### Phase 2: Move shared functions to engine
- [x] 2a. `settle_auto` → engine `game_setup.rs`
- [x] 2b. `execute_action` → engine `game_setup.rs`
- [x] 2c. `load_two_decks` → engine `deck_parser.rs`
- [x] 2d. `test_ai_vs_ai` → engine `game_setup.rs`

### Phase 3: PlatformUi trait + shared game loop
- [x] 3a. Created `PlatformUi` trait in `engine/src/game/platform_ui.rs`
- [x] 3b. Implemented for Wii/DC/DS/PSP (DcUi/WiiUi/PspUi/DsUi wrapper structs)
- [x] 3c. Shared functions: `ai_turn`, `show_result`, `select`, `menu_select`, `human_turn`, `handle_choice`
- [x] 3d. Replaced all 4 platform game loops with trait calls
- [x] 3e. Removed ~1,200 lines of duplicated game loop code

## Commits
- `2ed7149d` - consolidate duplicate code + 3DS fixes + PSP font fix
- `46abb29c` - consolidate duplicate code across engine and platforms (-287 lines)
- `44971cff` - consolidate game loop across 4 platforms via PlatformUi trait (-563 net lines)

## Total impact
- ~1,850 lines of duplicated code eliminated
- 1,829 tests pass
- Single source of truth for game logic in engine

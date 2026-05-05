# Gameplay Test Process

This document explains the process used to create gameplay integration tests 
for `qa_data.json` entries, using the Disotrtion (PL!SP-pb1-023-L) and 
Ayumu/Kanon/Koko (LL-bp1-001-R+) cards as examples.

## Overview

Each test validates a specific QA entry from `cards/qa_data.json`. The QA number 
should be included in the Rust test function name (e.g. `distortion_q97_...`, 
`ayumu_q62_...`) so failures can be traced back to the specific QA.

## Core Principle

The purpose of these tests is to **find and fix faults** in the parser and engine.
Each test should push the engine to its limits — test edge cases, error paths, and 
filter boundaries. If a test passes trivially without exercising real logic, it's
not doing its job. A good test proves:

- **The parser correctly extracts all ability fields** (action, source, destination, 
  conditions, costs, filters, special flags)
- **The engine correctly enforces all filters** (card type, cost, group, count, target)
- **The engine handles edge cases gracefully** (no cards match filter → no-op, not crash)
- **The engine rejects invalid configurations** (no empty area, insufficient energy)

## Step 0: Document Card Abilities

Before writing code, summarize each ability in plaintext:

```markdown
### Card LL-bp2-001-R+ (唐可可＆平安名すみれ＆米女メイ)
3 abilities parsed:

**Ab#0 (常時)**: Cost reduction per hand card.
  Text: 手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚につき、1少なくなる。
  Parsed: modify_cost, per_unit, subtract, location=hand
  ✅ Good.

**Ab#1 (常時)**: Cannot be batted to discard.
  Text: このメンバーはバトンタッチで控え室に置かれない。
  Parsed: restriction, cannot_baton_touch
  ✅ Good.

**Ab#2 (ライブ開始時)**: Discard named characters → blade per card.
  Text: ...を控え室に置いてもよい：ライブ終了時まで、これにより控え室に置いたカード1枚につき、ブレードを得る。
  Parsed: gain_resource, blade, per_unit, live_end, cost={characters, source=hand, optional}
  ✅ Cost parsed correctly. Characters: [唐可可, 平安名すみれ, 米女メイ]
```

## Step 1: Parser Analysis

For a target card:

1. Find the card in `cards/cards.json` and extract its `ability` text
2. Run `extract_card_abilities.py` to generate `cards/abilities.json`
3. Check the parser output in `abilities.json`:
   - Is the ability split correctly (multi-line abilities, cost vs effect via `：`)?
   - Are the action types correct (e.g. `gain_ability`, `change_state`, `modify_score`)?
   - Are conditions properly parsed (location conditions, state conditions, distinct)?
   - Are costs correctly extracted (source, destination, count, optional, characters)?
   - Are special patterns detected (`self_target` for "このカード", `all` for "すべて")?

Example faults found and fixed:
- `_try_sequential` was after `_try_conditional` in handler order → moved
- `_try_distinct` only checked `名前が異なる` but cards use `名前の異なる` → added both
- `normalize_fullwidth_digits` didn't handle `＋`/`−` → added
- `gain_ability` was detected but dispatch table overrode it → skip dispatch for gain_ability
- `execute_move_cards` returned `Err` when no cards matched filters → changed to `Ok(())` (silent skip)
- `card.group` vs `card.unit` mismatch in group filtering → updated to check both
- `re_yell` matched bare `エール` → changed to only match `もう一度エール` / `もう1度エール` (actual "cheer again" patterns). Bare `エール` matched "エールにより公開された..." (from cheered cards) leading to spurious re_yell actions in Poppin' Up!.
- `modify_yell_count` matched bare `エール` → changed to match `(エール in text) AND (枚数 in text OR 数 in text)` to correctly identify "枚数を増やす" (increase card count) vs generic エール mentions.
- `_try_temporal_count` had no default count → added count=1 when text contains `登場` without a number (e.g. "メンバーが登場している場合" = "member has debuted" → at least 1).
- `extract_destination` returned `"stage"` for `いたエリアに登場させる` (debut to vacated area) → added check for `いたエリアに` / `置かれていたエリアに` to return `"same_area"`.
- `extract_source` fallback: `move_cards` with destination=hand and no explicit source → default to `"discard"`. Previously the source was None → engine crashed with "Unknown source zone".
- `_dispatch` rule ordering: `'必要ハート' in t` mapped to `'restriction'` instead of `'modify_required_hearts'` → fixed action type and removed the `restriction_type` workaround.

## Step 2: Engine Fixes

Identify what engine changes are needed to support the parsed ability:

| Pattern | Engine fix |
|---------|-----------|
| `max: true` for energy refresh | Added `max` parameter to `execute_change_state` |
| `state_condition` with `resource_type: "energy"` | Updated `evaluate_state_condition` |
| `distinct` + `group_names` for stage cards | Added group filter in `evaluate_location_condition` |
| `self_target` for "このカード" scoping | Parser sets `self_target: True`, engine filters in `execute_modify_score` |
| `gain_ability` effect | Already implemented in engine (`Effect::GainAbility`) |
| LiveStart trigger timing | Moved from `set_live_card` to phase transition to `FirstAttackerPerformance` |
| `card.group` vs `card.unit` mismatch | Updated `card_matches_group_str` and distinct filter to check both |
| Debut count tracking (temporal_condition + count + location=stage + card_type=member_card) | Added `debut_count_this_turn` to Player. Incremented in `handle_play_member_to_stage`. Checked in `evaluate_temporal_condition`. |
| Auto abilities (trigger=自動) never fired | Added `trigger_auto_abilities_for_player` — scans stage for abilities with trigger `"自動"` and enqueues them after each member debut. |
| `draw_until_count` with target_count and hand destination | Draws `target_count - hand.len()` cards. Works automatically. |
| `same_area` destination for self_cost move+debut patterns | `execute_selected_cards_from_zone` in `choice.rs` now handles `"same_area"` using `last_vacated_stage_area`. |
| `position\|destination` choice had no handler | Added handler in `provide_choice_result` that calls `execute_position_change_with_destination` with the chosen position. |
| Position change didn't record movement | Added `record_card_movement` calls for swapped cards in `execute_position_change` and `execute_position_change_with_destination`. |
| `need_heart_modifier` stored but never read during scoring | Added modifier factoring into `calculate_live_score` via new parameter. |
| `BAll` blade hearts skipped in owned_hearts | Changed to count ALL blade as Heart00 (wildcard) for heart requirement satisfaction. |
| Score comparison (`comparison_type: "score"`) checked only success_live_zone | Added `live_card_zone` cards to the score sum, so current live scores are included at LiveSuccess timing. |
| `get_count_for_condition` ignored `comparison_type: "score"` | Added dispatch to `get_count_for_target` for score type, enabling `comparison_condition` with `comparison_target: "opponent"` to work. |
| `per_unit` counting used `matching * per_unit_count` (wrong) | Changed to `(matching / per_unit_count) * count`. For "2 cards per blade", 4 cards → (4/2)*1 = 2 blade. Floor division correct for Japanese "につき" (per X, incomplete groups excluded). |
| LiveSuccess abilities re-fired on re-entry to execute_live_victory_determination | Added `live_success_triggered_this_turn` flag. Only triggers once per turn. |
| `move_live_to_success` didn't check `can_place_card_in_zone` | Added check; cards restricted from success_live_zone go to waitroom instead. |

## Step 3: Test Writing Pattern

### Helper functions

Helper functions (`advance_to_live_card_set_p1`, `advance_to_live_start`, `assert_score`, `assert_energy`) are defined in `helpers.rs` and shared across test files via `mod helpers; use helpers::*;`.

For new test files, either import from helpers.rs or define local copies:

```rust
mod helpers;
use helpers::*;
```

```rust
fn assert_score(game: &TestGame, expected: i32) { ... }
fn assert_energy(game: &TestGame, active: usize, total: usize) { ... }
// Phase advancement
fn advance_to_live_card_set_p1(game: &mut TestGame) { ... }
fn advance_to_live_start(game: &mut TestGame) { ... }
```

### Test planning table

Before writing tests, plan each QA test:

| QA | What it tests | Setup needed | Expected outcome | Edge case / filter |
|----|--------------|--------------|-----------------|-------------------|
| Q186 | Cost reduction by hand count | Keke + 3 filler cards in hand | Cost = base - 3, min 0 | 0-cost at 7+ other cards |
| Q129 | Cost reduction doesn't affect revealed cost totals | Keke in hand, other card reveals costs | Other card's cost requirement unchanged | Cost reduction is card-specific |
| Q89 | Group/unit metadata | Load card | unit is None | — |
| Q62 | Name splits on & | Load card | 3 parts: 可可, すみれ, メイ | — |
| Live ability | Discard named chars → gain blade | Named chars in hand + optional cost | Blade per discarded card | Abilityless cards NOT in choice |

For cost/Q186: use abilityless filler cards (PL!-sd1-010-SD etc.) as "other hand cards"
to verify they don't interfere with the cost calculation.

For live ability: abilityless cards in hand should NOT appear in the discard-prompt choices
(only named characters 唐可可/平安名すみれ/米女メイ should be selectable).

### Test structure

```rust
#[test]
fn cardname_qNN_description() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    
    // 1. Setup: cards in hand, stage, energy zone
    // 2. Phase advancement
    // 3. Trigger ability
    // 4. Assert expected behavior
}
```

### Card selection — prefer abilityless filler cards

Unless the card's OWN ability is what's being tested, use abilityless filler cards 
(like `PL!-sd1-010-SD`, `PL!-sd1-013-SD`, `PL!-sd1-014-SD`) for setup (hand, 
discard, stage). These have empty `ability` fields so they won't trigger unexpected 
abilities during the test. Examples:

| Filler | Cost | Unit | Purpose |
|--------|------|------|---------|
| PL!-sd1-010-SD | 4 | Printemps | General filler (hand, deck, discard) |
| PL!-sd1-013-SD | 4 | lilywhite | General filler |
| PL!-sd1-014-SD | 9 | lilywhite | High-cost filler for stage |
| PL!-sd1-019-SD | 2 | — | Live card for LiveCardSet (type: ライブ) |
| LL-E-001-SD | — | — | Energy card |
| PL!SP-sd1-019-SD | 2 | — | Cost-2 member for discard pool |
| PL!SP-sd1-020-SD | 2 | — | Cost-2 member for discard pool |

### Common Parser Pitfalls

When analyzing the parsed output, watch for these recurring issues:

### Wrong action type from over-broad dispatch rules
The parser's dispatch rules (`_R` list in `parse_action`) match text keywords and set action types. Some keywords are too broad:
- `'エール' in t` matches both "cheer again" (re_yell) AND "from cheered cards" (source context). **Fix:** use more specific patterns like `'もう一度エール' in t`.
- `'必要ハート' in t` matched as `restriction` but should be `modify_required_hearts`. **Fix:** check the dispatch rule maps to the correct engine `EffectAction`.

### Missing critical fields
Always verify these are extracted:
- **`source`**: For `move_cards` to hand with no explicit source, the parser now defaults to `"discard"`. But check if the card had `revealed_remaining` or other special source.
- **`destination`**: For `"このメンバーが置かれていたエリアに"` (debut to vacated area), the destination should be `"same_area"` not `"stage"`.
- **`count`**: For temporal conditions with `"登場している"` (has debuted) but no number, the parser now defaults `count` to 1.
- **`group` vs `unit`**: The `card.group` field comes from series mapping, while `card.unit` is the subunit name (e.g., "5yncri5e!"). Filter conditions should use `card_matches_group_str` which checks both.

### Sequential effects that are actually condition+action
When the card text has "Aする。Bの場合、Cする" (do A. If B, do C), the parser may incorrectly merge B into a condition for C rather than making A a standalone action. Example: WAO-WAO Powerful day!'s "Activate Printemps members. If 3+ wait members became active, +1 score" — the "activate" part should be a `change_state` action, not just a `complex_condition` cause.

### Per-unit formula semantics
The `per_unit_count` field in parsed data means "grouping size" (for each X items), NOT "multiplier" (per item):
- `per_unit_count=2, count=1` → "for each 2 cards, get 1 reward" → reward = (matching / 2) * 1
- `per_unit_count=1, count=2` → "for each 1 card, get 2 reward" → reward = (matching / 1) * 2
- Floor division is correct (incomplete groups don't count).

### LiveSuccess re-fire loop
If a LiveSuccess ability creates a pending choice (e.g., "draw 2, discard 1"), `execute_live_victory_determination` returns early and the card stays in the live zone. On the next pass, LiveSuccess fires again → infinite loop. **Fix:** `live_success_triggered_this_turn` flag prevents re-triggering.

### Key considerations

- **Active phase activates ALL energy for both players!** The Phase::Active handler 
  calls `player1.activate_all_energy()` and `player2.activate_all_energy()`. Tests 
  needing wait energy must set it up AFTER `advance_to_live_card_set_p1` (which 
  passes through Active phase). Wait energy = push to `energy_zone.cards` without 
  incrementing `active_energy_count`.
- **Cost payment doesn't remove cards from energy zone!** `pay_energy` only decrements 
  `active_energy_count`. Cards stay in the zone as wait energy. Only 
  `place_energy_under_member` actually pops cards from the zone. When testing energy 
  counts, check `active_energy_count` for spent energy and `cards.len()` for 
  cards-under-member.
- **Per-unit `gain_resource` requires engine formula `(count / per_unit_count) * reward`.** 
  The old formula `count * per_unit_count` was wrong for `per_unit_count > 1`. Test 
  with hand sizes that are not multiples of per_unit_count to verify floor division.
- **Reveal cost auto-reveals when `card_ids.len() <= count`.** If there's exactly 1 
  matching card in hand and count=1, no pending choice is created. For tests needing 
  the reveal choice prompt, use more matching cards than the count.
- **`same_area` placement needs `last_vacated_stage_area` from a prior `self_cost`.** 
  Without a self_cost that records the vacated area, `same_area` falls back to 
  `stage_first_empty` (Center priority). Test by placing the original card at 
  RightSide and verifying the new card lands there.
- **`debut_count_this_turn` is only incremented by `play_to_stage`**, NOT by 
  `record_card_appearance`. If you add cards to stage via `add_to_stage`, they won't 
  increment the debut counter. Set `player.debut_count_this_turn` directly if needed.
- **LiveStart timing**: LiveStart abilities were moved from `set_live_card` to the 
  phase transition to `FirstAttackerPerformance`. Call 
  `advance_to_live_start(&mut game)` after `set_live_card()` to trigger them.
- **Member vs Live cards**: Cards with type `ライブ` can be set as live cards. 
  Cards with type `メンバー` go on the stage. Putting a member card in the live 
  zone will cause `check_invalid_cards` to remove it during `check_timing`.
- **Optional costs**: If an ability has a cost separated by `：`, the engine 
  processes the cost first (creates `PAY_COST` prompt). Optional costs (もよい) 
  can be skipped.
- **Duplicate card IDs**: Multiple copies of the same card share the same database 
  ID. `get_orientation_modifier`, `add_score_modifier` etc. use the DB ID as the 
  key and thus can't distinguish duplicates.
- **Full-width characters**: Card IDs often use full-width `＋` (U+FF0B) not 
  half-width `+`. Use `\u{ff0b}` in Rust string literals.

### Phase advancement helper reference

| Trigger | Phase to reach | How to get there |
|---------|---------------|------------------|
| 登場 (debut) | Main phase (P1) | `TestGame::new(db)` starts at Main |
| ライブ開始時 (live start) | FirstAttackerPerformance | `advance_to_live_card_set_p1` + `set_live_card` + `advance_to_live_start` |
| ターン終了時 (turn end) | Next turn's main | `game.pass()` 20 times |
| ライブ成功時 (live success) | LiveVictoryDetermination | After performance with successful live |
| 起動 (activation) | Main phase | `TestGame::new` starts at Main; use `activate_ability` |

### Gameplay mechanics that affect test numbers

- **Active phase activates ALL energy**: Both players' energy is refreshed to active
- **Energy phase draws 1 energy**: Each player automatically draws an energy card
- **LiveCardSet pass draws replacement**: When passing from LiveCardSet, each player 
  draws cards equal to their set live cards
- **Live performance cheers:** During FirstAttackerPerformance / SecondAttackerPerformance, cards equal to total blades are cheered from deck
- **Debut abilities trigger in batch**: All debut abilities from a single play event trigger in chronological order
- **Deck refresh**: When deck is empty, waitroom shuffles into deck (but waitroom must have cards)

## Parser/Engine Fix Table (updated per card)

| Card | Fix | Type |
|------|-----|------|
| PL!-pb1-028-L (WAO-WAO) | Added `_try_period_conditional` to effect handlers; fixed `_try_state_change` direction detection (wait→active); added `last_state_change_wait_to_active_count` field to GameState; fixed change_state count=0→all-matching; fixed wait detection (None=active not wait) | Parser+Engine |

## Step 4: Verifying QA Scenarios

For each QA entry:

1. Read the question/answer pair in `cards/qa_data.json`
2. Identify the game rule being tested
3. Determine what engine behavior needs to be verified
4. Write a targeted test that proves the rule

Some QAs test **data-level** rules (does the card exist? does its name contain X?) 
while others test **engine-level** rules (does the conditional effect fire correctly?).
Write data tests for the former, gameplay tests for the latter.

## Step 5: Running Tests

```bash
# Build library
cargo build --lib -j1

# Build and run all tests
cargo test --test gameplay_test -j1 -- --nocapture

# Run a specific test
cargo test --test gameplay_test -j1 -- distortion_q97 --nocapture
```

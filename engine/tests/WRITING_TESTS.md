# Writing Gameplay Tests — Complete Guide

## Philosophy

- **Real cards only.** Every test uses real card numbers from `cards/cards.json` with real abilities from `cards/abilities.json`.
- **Test what the Japanese text says.** The ability text is the spec, not the JSON field names.
- **Tests simulate what a player does.** Play card → activate ability → make choice → verify board. Avoid test-only helpers that bypass the action pipeline.
- **One test per unique ability text pattern.** If the same text appears on 27 cards, test it once.
- **Fix the engine, not the test.** The test encodes expected behavior from the Japanese text + rules. A failing test means a bug in the engine (or parser).
- **Filler cards have zero abilities.** No unexpected triggers, no interference.

---

## TestGame Initial State

```rust
let db = load_real_database();
let mut game = TestGame::new(db);
```

`TestGame::new(db)` starts at:

| Field | Value |
|-------|-------|
| `current_phase` | `Phase::Main` |
| `current_turn_phase` | `TurnPhase::FirstAttackerNormal` |
| `turn_number` | 1 |
| `player1.is_first_attacker` | `true` |

RPS, mulligan, and initial draw (6 cards) are **skipped**. You must manually add cards.

---

## Phase Transition Reference

`game.pass()` calls `advance_phase()`. The transitions are deterministic.

### Normal Turn: `FirstAttackerNormal` / `SecondAttackerNormal`

```
Active → Energy → Draw → Main → (cycle to next player or live)
```

| Phase entered | Side effects |
|-----------|--------------|
| **Active→Energy** | Activates all waited members, refreshes all energy to active |
| **Energy→Draw** | Draws 1 energy from energy deck |
| **Draw→Main** | **Draws 1 card from main deck** (`draw()` from index 0) |
| **Main→(next)** | If `FirstAttackerNormal`: switches to `SecondAttackerNormal`, goes to Active. If `SecondAttackerNormal`: switches to `TurnPhase::Live`, goes to `LiveCardSetFirstAttacker` |

**Critical:** The Draw phase draws from **deck index 0** (the top). If you `insert(0, my_card)` and then `pass()` through the Draw phase, your card gets drawn.

### Live Turn Phase: `Live`

```
LiveCardSetFirstAttacker → LiveCardSetSecondAttacker → FirstAttackerPerformance → SecondAttackerPerformance → LiveVictoryDetermination
```

| Phase entered | Side effects |
|-----------|--------------|
| **LiveCardSetFirstAttacker** | `pass()`: **P1 draws 1 per card in P1's live card zone**, transitions to SecondAttacker |
| **LiveCardSetSecondAttacker** | `pass()`: **P2 draws 1 per card in P2's live card zone**, then enters performance — **LiveStart abilities fire here** for both players, auto-abilities processed |
| **FirstAttackerPerformance** | Resolves the live performance (cheer, scoring) |
| **SecondAttackerPerformance** | Same for second attacker |
| **LiveVictoryDetermination** | Compares scores, clears revealed cards, increments turn, resets to Active in `FirstAttackerNormal` |

### Phase Transition Table (for LiveStart test setup)

| `pass()` count | Phase after | Turn Phase | Side effect on P1 deck |
|---------------|-------------|------------|----------------------|
| 0 | Main | FirstAttackerNormal | — |
| 1 | Active | SecondAttackerNormal | — |
| 2 | Energy | SecondAttackerNormal | — |
| 3 | Draw | SecondAttackerNormal | — |
| 4 | Main | SecondAttackerNormal | **draw 1** |
| 5 | LiveCardSetFirstAttacker | Live | — |
| (set live card) | LiveCardSetFirstAttacker | Live | — |
| 6 | LiveCardSetSecondAttacker | Live | **P1 draws live-zone count** |
| 7 | FirstAttackerPerformance | Live | **P2 draws live-zone count; LiveStart fires — drain choices here** |

Total draws from deck during `advance_to_live_card_set_p1` (5 passes) → **1 draw**.
After set_live_card: pass #6 refills by P1's live zone, pass #7 by P2's
(usually empty) AND fires ライブ開始時. Measure hand deltas between the
set and these passes; a third pass runs the performance phase and shuffles
hand cards (cheer reveal), so stop asserting there.

### Helper implementations

```rust
fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
```

### `check_timing` triggers during phases

`check_timing()` is called during every phase transition (inside `advance_phase`) and during `execute_effect`. It:
- Refreshes both players' decks from waitroom if empty
- Checks victory condition
- Checks invalid live/energy cards
- Checks orphaned under-member cards
- **Processes pending auto abilities** — triggers may fire during phase transitions

---

## Setup Methods

### Zone Setup

| Method | Purpose |
|--------|---------|
| `game.id("PL!S-bp2-009-R")` | Look up a card by card_no → i16 ID |
| `game.add_to_hand(id)` | Put a card in player1's hand |
| `game.add_to_discard(id)` | Put a card in player1's waitroom |
| `game.add_to_stage(area, id)` | Place a card on player1's stage |
| `game.give_energy(n)` | Give player1 n active energy (LL-E-001-SD) |

**Deck:** Index 0 = top of deck. `push()` adds to bottom. `draw()` and `peek_top()` read from index 0. To put a card on top: `game.state.player1.main_deck.cards.insert(0, id)`.

### Actions

| Method | What it does |
|--------|-------------|
| `game.play_to_stage(card_id, MemberArea::Center)` | Play member from hand to stage (pays cost, fires debut) |
| `game.activate_ability(stage_card_id)` | Activate first 起動 ability on a stage card |
| `game.select_indices(&[0])` | Select cards by zone index (for SelectCard choices) |
| `game.select_indices(&[])` | Skip an optional cost / auto-ability (empty = skip) |
| `game.select_option(1)` | Pick option 1 (for SelectTarget choices like pay/skip) |
| `game.has_pending_choice()` | Check if ability queue is waiting for input |
| `game.pass()` | Advance to next phase |

### `game.id()` vs `game.new_id()` vs `game.id_ref()`

```rust
let a = game.id("PL!-sd1-010-SD");      // pops from pre-created pool (5 copies)
let b = game.id("PL!-sd1-010-SD");      // different ID from same template
let c = game.new_id("PL!-sd1-010-SD");  // also different, from counter pool
let d = game.id_ref("PL!-sd1-010-SD");  // stable reference, doesn't consume pool
```

- `id()` consumes from a pre-seeded pool (5 IDs per template). Use for cards you reference by variable.
- `new_id()` falls back to a monotonically increasing counter.
- `id_ref()` peeks the last available ID without consuming. Use for stable references in assertions (`contains(&id_ref(...))`).

---

## Assertions

### Hard assertions — never use `if` guards

```rust
// ❌ WRONG: silently tolerates wrong prompt types
if game.has_pending_choice() {
    game.select_indices(&[0]);
}

// ✅ CORRECT: assert the expected choice type
assert!(game.has_pending_choice(), "Expected SelectCard for discard");
assert_eq!(game.pending_choice_type(), Some("SelectCard"));
game.select_indices(&[0]);
```

### Verify card locations after effects

```rust
assert!(player.stage.stage.contains(&card_id), "on stage");
assert!(!player.hand.cards.contains(&card_id), "not in hand");
assert!(!player.waitroom.cards.contains(&card_id), "not in discard");
```

### Verify state changes (wait/active)

```rust
assert_eq!(
    game.state.mods.get_orientation_modifier(card_id),
    Some(&"wait".to_string()),
    "card should be waited"
);
```

### Verify both players in cross-player effects

```rust
assert!(p1.stage.stage.contains(&p1_card), "P1 should get their card");
assert!(p2.stage.stage.contains(&p2_card), "P2 should get their card");
```

### Verify prompt chain exhaustively

```rust
// Prompt 1: P1 selects card from discard
assert_eq!(game.pending_choice_type(), Some("SelectCard"), "P1: discard select");
game.select_indices(&[0]);

// Prompt 2: P1 chooses position
assert_eq!(game.pending_choice_type(), Some("SelectPosition"), "P1: position");
game.select_option(1);

// No more prompts
assert!(!game.has_pending_choice(), "No remaining prompts");
```

### Choice inspection

```rust
let choice = game.state.ability_queue.is_waiting_for_choice().cloned().unwrap();
match &choice {
    Choice::SelectCard { zone, card_type, count, allow_skip, .. } => {
        assert_eq!(zone, "looked_at");
        assert_eq!(card_type.as_deref(), Some("live_card"));
    }
    _ => panic!("unexpected choice type"),
}
```

---

## Common Mistakes

### 1. Deck-top card consumed by Draw phase

```rust
// ❌ WRONG: The card at index 0 gets drawn during advance_to_live_card_set_p1
game.state.player1.main_deck.cards.insert(0, my_test_card);
advance_to_live_card_set_p1(&mut game);  // 1 draw happens → my_test_card is in hand now!

// ✅ CORRECT: One filler shields the test card
fill_decks(&mut game);
game.state.player1.main_deck.cards.insert(1, my_test_card);
// Deck: [filler_0, test_card, filler_1, ...]
// Passes draw filler_0 → test_card is now at deck[0]
```

### 2. Insert vs push — deck orientation

```rust
game.state.player1.main_deck.cards.insert(0, card);   // puts on TOP (index 0)
game.state.player1.main_deck.cards.push(card);         // puts on BOTTOM (end)
```

### 3. `fill_decks` baseline ordering

`fill_decks()` pushes 20 cards. If you capture `let deck_before = ...` BEFORE calling fill_decks, the baseline is 0. Any assertion using `deck_before` after fill_decks will be wrong. Capture baselines AFTER fill_decks, or use relative comparisons.

### 4. Not clearing pending choices loop

```rust
// ❌ WRONG: might miss a prompt
game.activate_ability(card_id);
game.select_indices(&[0]);

// ✅ CORRECT: drain all prompts with safety counter
let mut safety = 0;
while game.has_pending_choice() && safety < 30 {
    safety += 1;
    game.select_indices(&[0]);
}
```

### 5. Multi-ability card creates mixed choice types

Cards with 2+ abilities (e.g. 朝香果林 with draw+wait AND select+place) create chained choices:
- `SelectAutoAbility` — which abilities to trigger
- `SelectCard` — change_state (wait opponent member)
- `SelectCard` — card selection from discard
- `SelectTarget` / `SelectPosition` — any-order deck placement

A while loop without handling unknown types causes infinite loops. Always add an `else { skip }` fallback:

```rust
while game.has_pending_choice() && safety < 30 {
    safety += 1;
    if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
        game.select_indices(&[]);
    } else {
        game.select_indices(&[0]); // select first available
    }
}
```

### 6. Forgetting opponent's state

```rust
assert!(p1.hand.cards.contains(&card), "P1 should have the card");
assert!(!p2.hand.cards.contains(&card), "P2 should NOT have the card");
```

### 7. Adding debug-only injected functions

```rust
// ❌ WRONG: bypasses debut triggers
fn my_setup(game: &mut TestGame) {
    game.state.player1.stage.stage[0] = my_card;
}

// ✅ CORRECT: use standard actions
fn my_setup(game: &mut TestGame) {
    game.add_to_hand(my_card);
    game.give_energy(5);
    game.play_to_stage(my_card, MemberArea::LeftSide);
}
```

If you must manipulate state directly (e.g. opponent's stage), add a comment explaining why.

### 8. Phase-advance draws make absolute counts unreliable

The helper functions advance through turn phases. During these passes, Draw-phase draws occur, making absolute deck/hand counts unpredictable for assertions. Prefer asserting relative deltas (e.g. waitroom decreased by N, deck top has the expected card ID).

---

## Behavioral Contracts by Trigger Type

### 起動 (Activation)
- Cost is paid immediately when the ability button is clicked
- If cost is `self_cost stage→discard`: the card moves to discard, no intermediate choice
- If cost requires selection (e.g. `hand→discard`): a choice prompt must show immediately
- After cost is paid, the effect runs

### 登場 (Debut)
- Triggers automatically when the card is placed on stage (after paying energy cost)
- If cost is optional: player must be prompted to pay or skip
- After cost (pay/skip), the effect runs
- Always use `game.play_to_stage()` — this properly fires the debut trigger

### 自動 (Auto)
- Triggers automatically when the condition is met
- If there is a cost, the player must confirm before paying
- No choice of "when" — the ability either fires or doesn't

### 常時 (Constant)
- Always active while the card is on stage
- Modifies game state continuously (blade/heart modifiers)
- Does not use the ability queue

### ライブ開始時 (Live Start) / ライブ成功時 (Live Success)
- Trigger when the live phase starts / when live succeeds
- Cost (if any) is optional unless stated otherwise
- Effects like `gain_resource` apply temporary modifiers (duration: until live end)

### Selection from Discard/Waitroom
- **Never automatic** when more matching cards exist than count — player must choose
- If exactly `count` matching cards, auto-select is acceptable
- Choice must filter by `card_type` — wrong-type cards must not appear
- Group/name/cost filters must be respected

### Look-and-Select
- `デッキの上からN枚見る` looks at top N cards (index 0 = top)
- Player selects from the looked-at cards, NOT from the deck directly
- Unselected looked-at cards go to discard (unless specified otherwise)

---

## Edge Cases & Engine Details

### Opponent wait state must be manually set

There's no helper for this:

```rust
game.state.player2.stage.stage[i] = card_id;
game.state.mods.add_orientation_modifier(card_id, "wait");
```

If the opponent member is already waited, `change_state` actions targeting it will skip (no eligible targets).

### Dynamic count resolution ≠ strict count

The dynamic count reference `"相手のステージにいるウェイト状態のメンバー"` resolves to ALL opponent stage members (non-empty slots), not just waited ones. `resolve_dynamic_count` at `draw.rs:160` only checks for `"ステージ"` + `"メンバー"` substrings.

Also, `execute_select()` had a strict check `available < count → skip` that fired for max-mode selections when count exceeded available cards (e.g. 3 waited but only 1 matching card in discard). This silently skipped the entire selection. Fixed to: skip only when `available == 0`, cap count = min(count, available).

### Card group matching depends on series

`cards.json` has `group_name: null` for all cards. Group matching works through `card_matches_group_str` which checks `unit`, `group`, card name fragments, and **series** via `card_series_matches_group`. For example, `PL!N-sd1-010-SD` has series `"ラブライブ！虹ヶ咲学園スクールアイドル同好会"` which matches group `"虹ヶ咲"`.

### `select_indices` behavior for non-stage zones

For discard/selected_cards zones, `filtered_indices` is `None`, so indices map directly to waitroom positions. Index 0 always selects the first card in the waitroom — the filtering (by group/type/etc.) already happened during choice creation.

### Area Selection

When a card says "select an area different from current", the engine creates an `area_select` choice. The selected area is used by the subsequent `position_change`:

```rust
game.select_option(0);  // picks first available position
```

### Center Position Requirement

Cards with `{{center.png|センター}}` in the cost text require Center position:

```rust
// SUCCESS
game.state.player1.stage.stage = [-1, card_id, -1];
game.activate_ability(card_id);

// FAILURE
game.state.player1.stage.stage = [card_id, -1, -1];
let err = game.try_activate_ability(card_id).unwrap_err();
assert!(err.contains("position"));
```

### Use Limit Testing

Activation abilities with `use_limit: 1` can only be used once per turn:

```rust
game.activate_ability(card_id);
while game.has_pending_choice() { game.select_indices(&[0]); }
let err = game.try_activate_ability(card_id).unwrap_err();
assert!(err.contains("use_limit") || err.contains("already used"));
```

### Rotation Pattern (003-R)

For `position_change` with `multiple_targets=true` and `position` field:
```
Before: [A, B, C] at [left, center, right]
After:  [B, C, A]
```

### Mulligan Selection

Tracked in `game_state.mulligan_selected_indices`. The display sends `mulligan_selection` to the UI; tests verify the bitmask matches.

---

## Filler Card Reference

`fill_decks()` adds 20 filler cards to each player's deck. Fillers have zero abilities and zero triggers.

| card_no | type | cost | notes |
|---------|------|------|-------|
| `PL!-sd1-010-SD` | member | 4 | most common filler |
| `PL!-sd1-020-SD` | live | N/A (score 2) | live card |
| `PL!-sd1-021-SD` | live | N/A (score 3) | alternative live |
| `LL-E-001-SD` | energy | — | standard energy card |

Always put at least 2 filler cards in zones (deck, discard, hand) to prevent edge cases with empty-zone detection and refresh mechanics.

---

## Standard Test Templates

### LiveStart ability

```rust
fn test_live_start_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    fill_decks(&mut game);
    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            game.select_indices(&[0]);
        }
    }

    // Assert results
    assert!(game.state.player1.stage.stage.contains(&karin));
}
```

### Activation ability with discard→hand

```rust
fn test_activate_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ruby = game.id("PL!S-bp2-009-R");
    let target = game.id("PL!-sd1-021-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(ruby);
    game.add_to_discard(target);
    game.add_to_discard(filler);
    game.give_energy(3);
    game.play_to_stage(ruby, MemberArea::Center);
    game.activate_ability(ruby);
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);
    assert!(game.state.player1.hand.cards.contains(&target));
    assert!(game.state.player1.waitroom.cards.contains(&filler));
}
```

---

## Debugging Tips

- **"play_to_stage failed: Could not pay N energy"** — Check `game.give_energy(N)`.
- **"No pending choice"** — Auto-resolved (only 1 valid target). Add more valid targets.
- **Wrong index in `select_indices`** — Indices refer to original zone positions.
- **Test passed but behavior seems wrong** — Check `rules/rules.txt` and `cards/qa_data.json`.
- **Missing JSON field in abilities.json** — Fix the parser in `card_loader.rs` or `parser.py`.
- **Infinite loop in while loop** — Add safety counter and `else { skip }` fallback.
- **Deck assertion off by 1** — Phase-advance draws consumed a card; use relative deltas.

---

## Checking Your Test Count

```bash
cargo test --test run_all 2>&1 | grep "test result"
```

You should see a few thousand `passed; 0 failed`. The exact count varies as tests are added, but zero failures is the invariant.

---

## Coverage Inventory (automated)

Do not hand-edit coverage docs — they are generated from `cards/abilities.json` + the test suite:

```bash
python cards/test_inventory.py          # regenerate all
python cards/test_inventory.py --check  # CI: verify fresh
```

Outputs:

| File | Content |
|------|---------|
| `engine/tests/TEST_COVERAGE.md` | By trigger/action/condition/set + gap tables (wraps old `coverage_report.py`) |
| `docs/ABILITY_MATRIX.md` | Trigger×action matrix + prioritized gaps |
| `engine/tests/TEST_INVENTORY.json` | Machine-readable per-ability rows (for tooling) |
| `engine/tests/TEST_INVENTORY.md` | Human-readable per-ability index |

`depth` is inferred automatically:

- **L0** — `game.id("PL!…")` appears
- **L1** — plus `assert` in covering file
- **L2** — plus negative hint in file/test name (`cannot`, `negative`, `immune`, `already_waited`, `zero_tested`)
- **+choice** — plus `has_pending_choice` / `SelectCard` signals

Optional override: add a comment above a test to pin depth:

```rust
/// @covers PL!N-bp7-021-N depth=L2
#[test]
fn mia_cost_reduction_blocked_when_not_debut() { ... }
```

The alias `python cards/coverage_report.py` still works (deprecated shim → `test_inventory.py`). Legacy scripts in `cards/ability_docs_scripts/` (`generate_report.py`, `cross_reference_tests.py`, etc.) are now deprecated shims to the same command — use the single `test_inventory.py` entry point.

---

## Involved Patterns (recurring — reuse, don't re-derive)

These are non-obvious setups that keep coming up. Copy them.

### A. Triggering an each_time / auto ability directly (no live-phase scaffolding)

Many each_time abilities watch a zone change (e.g. "a card is placed from your deck to
your discard by a live-success ability"). Driving them through the full live phase is
fragile. Instead, run the **real TAS scan** with a recorded movement event:

```rust
for &cid in &moved {
    game.state.push_movement_event(cid, "deck", "discard", Some(cause_card), "p1", true);
}
game.state.trigger_auto_abilities_for_player(&game.state.player1.id.clone());
game.state.process_pending_auto_abilities(&game.state.player1.id.clone());
```

`push_movement_event` sets both `recently_moved_cards` and `turn_movements` (with source/
destination), so movement `Location` conditions evaluate correctly. The TAS scan enqueues
the each_time with `trigger_moved_cards` = the moved batch. See
`bp7_like_a_treasure_optional_test.rs` / `bp7_mia_optional_recover_test.rs`.

Then drain the choice(s), matching on `Choice::SelectTarget { target, .. } if target == "conditional_optional"`:

```rust
let mut guard = 0;
while game.has_pending_choice() && guard < 40 {
    guard += 1;
    match game.get_pending_choice() {
        Choice::SelectTarget { target, options, .. } if target == "conditional_optional" => {
            game.select_choice_option(if accept { 1 } else { 0 });
        }
        Choice::SelectCard { count, .. } => {
            if *count > 0 { game.select_indices(&[0]); } else { game.select_indices(&[]); }
        }
        _ => break,
    }
}
```

### B. Making PLAYER2 act (opponent effects)

`TestGame` starts with player1 as first attacker / active player, so all `play_to_stage`
and `activate_ability` helpers act as player1. To make an OPPONENT (player2) effect, flip
`is_first_attacker`:

```rust
fn set_active(game: &mut TestGame, p1_active: bool) {
    game.state.player1.is_first_attacker = p1_active;
    game.state.player2.is_first_attacker = !p1_active;
}
```

Then `activate_ability(p2_card)` / `play_to_stage(p2_card, ...)` act as player2, and the
ability's controller (`gs.ability_master_id()`) is player2. Player2 needs their own energy
(`player2.energy_zone.cards.push(e); add_active(n)`). Shared helper:
`bp7_wait_immunity_helpers::set_active`.

### C. Wait-immunity (BP07 G4) — 松浦果南 `PL!S-bp7-003-R＋` ab#1 option 1

Choosing option 1 grants "相手の効果によってはウェイトしない" (owner's Aqours members with
blade ≤ 3 are immune to the OPPONENT's wait effects). To test: player2 establishes immunity
on their own 果南, then player1's wait ability targets it → blocked.

```rust
use crate::test_modules::bp7_wait_immunity_helpers::*;
let p2_kanan = p2_establish_wait_immunity(&mut game); // player2 plays 果南, option 1
// ... player1's wait ability targets player2's 果南 ...
assert!(!is_waited(&game, p2_kanan), "opponent wait must be blocked");
```

The same immunity is verified against 5 diverse wait abilities across existing wait-test
files: 朝香果林 (blade-limit 起動), 矢澤にこ (opponent-waits-own), 高坂穂乃果 (cost-limit),
西木野真姫 (opponent-own, BiBi), 園田海未 (debut cost-limit).

### D. Regenerating abilities.json / parser output

After editing the Python parser, regenerate everything the engine consumes:

```bash
cd cards/ability_extraction && python extract_card_abilities.py   # rewrites cards/abilities.json
cd ../../engine && cargo test                                     # bytecode/abilities_gen.rs auto-regen'd
```

`extract_card_abilities.py` also re-runs `compile_abilities.py` (regenerates
`engine/src/ability/abilities_gen.rs`). Re-parsing changes the whole corpus, so a tiny parser
edit can shift OTHER abilities — always run the full suite and diff the relevant card's JSON
in `cards/abilities.json`.

### E. "…したとき" (when you do so) consequence gating

`conditional_on_optional{optional_action, conditional_action}` — on accept the engine runs
`conditional_action`. The consequence is gated on the move actually happening via
`AbilityResolver.last_move_moved_any` (set by `execute_move_cards`); a following
`ModifyScore` or self-recover move is skipped when the preceding move moved nothing. Tests:
`bp7_like_a_treasure_optional_test.rs`, `bp7_mia_optional_recover_test.rs`.

### F. Firing a real ability deterministically (no live-phase scaffolding)

Instead of threading a card through the live phase, you can enqueue a **specific
real ability** on a card and drive it straight into the resolver. This is the
most reliable way to test an effect in isolation while still using the real
card + real ability:

```rust
use rabuka_engine::core::types::AbilityTrigger;

fn fire_debut(game: &mut TestGame, card_no: &str, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card.resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("card should have a 登場 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text), // ability_id, e.g. "PL!X-...-R_登場…"
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()), // source_card_id (card_no)
        Some(card_id),                  // explicit_card_id (the copy on the board)
        None,                           // trigger_moved_cards
        None,                           // triggering_member_id
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}
```

`trigger_auto_ability`'s full signature (engine/src/core/game_state/abilities.rs:729):

```
trigger_auto_ability(
    ability_id: String,                       // "<card_no>_<full_text>"
    trigger_type: AbilityTrigger,             // Debut | Auto | LiveStart | LiveSuccess | …
    player_id: String,                        // "p1"
    source_card_id: Option<String>,           // the card_no string
    explicit_card_id: Option<i16>,            // the copy id; None → found by card_no
    trigger_moved_cards: Option<SmallVec<[i16; 4]>>,
    triggering_member_id: Option<i16>,
)
```

The `ability_id` format is `"{card_no}_{full_text}"`. Match it against
`card.resolved_abilities()` to get the exact string. See
`bp7_q267_rinna_mill_refresh_test.rs` for a full example (fires RINA's real
登場 mill-7). This is far less fragile than driving the live phase.

### G. Reading back a zone-change (asserting source/destination)

Zone-change autos ("…から…に置かれたとき") read `game.state.turn_movements`. The
engine records a real movement with BOTH source and destination zones, and the
condition filters on them. To assert which zone a card actually came from:

```rust
// The engine recorded a real deck→discard (or hand→discard) move:
let mv = game.state.turn_movements.last().expect("a movement was recorded");
assert_eq!(mv.moved_card_id, mia);
assert_eq!(mv.source_zone, "deck");   // THE crux — was it really deck, not hand?
assert_eq!(mv.dest_zone, "discard");
```

`MovementEvent` fields: `moved_card_id`, `source_zone`, `dest_zone`,
`cause_card_id`, `cause_player_id`, `effect_only`, `timestamp`. When the
condition's `source` is set (e.g. `deck`), the engine filters
`turn_movements` by `source_zone`, so a hand→discard event (`source_zone="hand"`)
must NOT match a `deck→discard` trigger. See
`live_card_zone_movement_test.rs::source_dest_condition_matches_turn_movements`
for the canonical read-back assertions.

`turn_movements` is cleared at turn start (`clear_card_movement_tracking`), so
capture baselines per-turn when a test crosses phases.

### H. Multiple abilities in one test — driving the chain

Cards with 2+ abilities (or a 登場 ability that itself spawns a second ability)
produce a chain of heterogeneous choices. The robust idiom is a **guarded loop
that dispatches on choice type**, never an unguarded sequence of
`select_indices`:

```rust
fn drain_choices(game: &mut TestGame, want_recover: bool) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]), // trigger which autos
            Choice::SelectTarget { target, options, .. }
                if target == "conditional_optional" => {
                    // "may do X, if you do then Y" — accept/skip
                    game.select_choice_option(if want_recover { 1 } else { 0 });
                }
            Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" => {
                    game.select_option(1); // pay the optional cost
                }
            Choice::SelectCard { count, .. } => {
                if *count > 0 { game.select_indices(&[0]); } else { game.select_indices(&[]); }
            }
            Choice::SelectPosition { .. } => game.select_option(0),
            _ => break, // unexpected type — stop so the guard never spins
        }
    }
}
```

Key rules for multi-ability tests:
- **Dispatch on `Choice` variant, not just zone string.** Use a `match
  game.get_pending_choice() { … }` with a `_ => break` fallback (the WRITING_TESTS
  §5 warning about infinite loops applies doubly here).
- **`conditional_optional`** gates a follow-up action on the move actually
  happening; options[1] = do it, options[0] = skip.
- **`pay_optional_cost:skip_optional_cost`** is the "may pay X" cost gate.
- **Assert the chain exhaustively when the order is deterministic**, but use the
  guarded loop when the order/number of prompts varies.
- To test one ability in a multi-ability card without the others interfering,
  fire the specific ability via pattern F above.

### I. Reading a card's parsed effect during debugging

`cards/abilities.json` is grouped by unique ability text. Quick inspect with python:

```bash
cd cards && python -c "import json; d=json.load(open('abilities.json',encoding='utf-8')); [print(json.dumps(a['effect'],ensure_ascii=False)) for a in d['unique_abilities'] if any('PL!X-...' in c for c in a.get('cards',[]))]"
```

This is debugging/verification, NOT a test — never assert on the JSON in a test.

# Gameplay Test Framework

Tests live in `tests/gameplay_test.rs`. Helpers are in `tests/helpers.rs`.
Filler cards (zero abilities) are in `tests/data/cards.json` (190 cards).

## Philosophy

- **Real cards only.** Every test uses real card numbers from `cards/cards.json` with real abilities from `cards/abilities.json`.
- **Tests simulate what a player does.** Play card → activate ability → make choice → verify board.
- **Filler cards have zero abilities.** No unexpected triggers, no interference. 150 abilityless members + 40 abilityless lives.
- **Japanese ability text is the spec.** Expected behavior is derived from the Japanese ability text and the rules (`rules/rules.txt`, `cards/qa_data.json`), not from JSON field names.
- **One test per unique `full_text`.** 602 unique ability texts exist. Testing each once covers all cards — no need to retest the same text on 27 different cards.
- **Fix the engine, not the test.** When a test fails, the engine has a bug. The test encodes what should happen based on the Japanese text + rules.
- **parser.py may need fixes too.** If the test reveals missing JSON fields (e.g., `cost_limit` vs `total_cost_limit`), the parser in `card_loader.py` or `card_parser.py` may need updating to produce correct AbilityEffect data.

## Expected Behavior Reference

These are the behavioral contracts for each trigger/effect pattern, derived from the Japanese text and the rules.

### 起動 (Activation)
- Cost is paid immediately when the ability button is clicked
- If cost is `self_cost stage→discard`: the card moves to discard, no intermediate choice
- If cost requires selection (e.g., `hand→discard`): a choice prompt must show immediately
- After cost is paid, the effect runs. If the effect requires a choice, a new prompt appears

### 登場 (Debut)
- Triggers automatically when the card is placed on stage (after paying energy cost)
- If cost is optional (e.g., `手札を1枚控え室に置いてもよい`): player must be prompted to pay or skip
- After cost (pay/skip), the effect runs
- `自分のデッキの上からカードをN枚見る` = peek top N cards of deck
- `その中から1枚を手札に加え` = player chooses 1 of the peeked cards → hand
- `残りを控え室に置く` = remaining peeked cards → discard

### 控え室からの選択 (Selection from Discard)
- **Never automatic.** If there are more matching cards than the count, the player must choose.
- If there are exactly `count` matching cards, auto-select is acceptable (no ambiguity).
- The choice must filter by `card_type` (live_card, member_card) — wrong-type cards must not appear in the choice.

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

### Discard/Waitroom Operations
- `控え室に置く` (put in waiting room) is the standard discard destination
- `控え室から手札に加える` (add from waiting room to hand) always requires a choice if multiple targets exist
- `控え室からメンバーカード` / `ライブカード` — card_type filter must be respected

### Look-and-Select
- `デッキの上からN枚見る` looks at the top N cards of the deck (index 0 = top)
- The looked-at cards are shown to the player
- The player selects from the looked-at cards, NOT from the deck directly
- Unselected looked-at cards go to discard (unless specified otherwise)

## Filler Card Reference

All cards in `data/cards.json` have no entry in `abilities.json` — they have **zero abilities/triggers**. Use them freely.

### Members (150 available, sample):

| card_no | cost | blade | hearts |
|---------|------|-------|--------|
| `PL!-sd1-010-SD` | 4 | 1 | heart01, heart03 |
| `PL!-sd1-013-SD` | 4 | 1 | heart05 |
| `PL!-sd1-014-SD` | 9 | 1 | heart02 |
| `PL!N-sd1-015-SD` | 4 | 1 | heart01, heart03 |
| `PL!SP-sd1-013-SD` | 4 | 1 | heart04, heart05 |
| `PL!SP-sd1-018-SD` | 4 | 1 | heart01, heart02 |
| `PL!S-pb1-010-N` | 5 | 0 | heart00 |
| `PL!N-pb1-025-N` | 4 | 1 | heart01, heart03 |
| `PL!-bp3-015-N` | 4 | 1 | heart03 |
| `PL!S-bp5-012-N` | 2 | 1 | heart02 |

### Lives (40 available, sample):

| card_no | score | need_heart |
|---------|-------|------------|
| `PL!-sd1-020-SD` | 2 | heart01, heart03 |
| `PL!-sd1-021-SD` | 3 | heart01, heart03, heart06 |
| `PL!S-PR-022-PR` | 12 | heart01-06 |
| `PL!S-bp2-019-L` | 17 | heart01-05 |
| `PL!-bp3-020-L` | 10 | heart01-06 |
| `PL!-pb1-033-L` | 9 | heart01, heart02, heart03 |

### Energy

Standard energy card: `LL-E-001-SD`. Always loaded from the real database.

## TestGame Helper

```rust
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;
mod helpers;
use helpers::*;

let db = load_real_database();
let mut game = TestGame::new(db);
```

### Zone Setup

| Method | Purpose |
|--------|---------|
| `game.id("PL!S-bp2-009-R")` | Look up a card by card_no → i16 ID |
| `game.add_to_hand(id)` | Put a card in player1's hand |
| `game.add_to_discard(id)` | Put a card in player1's waitroom (discard) |
| `game.add_to_stage(area, id)` | Place a card on player1's stage |
| `game.give_energy(n)` | Give player1 n active energy (LL-E-001-SD) |

**Deck convention:** Index 0 = top of deck. `push()` adds to bottom. `draw()` and `peek_top()` read from index 0.
To put a card on top: `game.state.player1.main_deck.cards.insert(0, id)`.

**Filler card excess:** Always put a few extra abilityless filler cards in zones (deck, discard, hand)
that aren't part of the test. This prevents edge cases where empty zones trigger unexpected game mechanics
(e.g., deck refresh detection, hand size checks). A good rule: at least 2 filler cards in deck + discard.

### Actions

| Method | What it does |
|--------|-------------|
| `game.play_to_stage(card_id, MemberArea::Center)` | Play member from hand to stage (pays cost) |
| `game.activate_ability(stage_card_id)` | Activate first 起動 ability on a stage card |
| `game.select_indices(&[0])` | Select cards by zone index (for SelectCard choices) |
| `game.select_indices(&[])` | Skip an optional cost (empty = skip) |
| `game.has_pending_choice()` | Check if ability queue is waiting for input |
| `game.pass()` | Advance to next phase |

### Assertions

```rust
// Board state
assert_eq!(game.state.player1.stage.get_area(MemberArea::Center), Some(card_id));
assert!(game.state.player1.hand.cards.contains(&card_id));
assert!(game.state.player1.waitroom.cards.contains(&card_id));

// Choice inspection
let choice = game.state.ability_queue.is_waiting_for_choice().cloned().unwrap();
match &choice {
    Choice::SelectCard { zone, card_type, count, allow_skip, .. } => {
        assert_eq!(zone, "looked_at");
        assert_eq!(card_type.as_deref(), Some("live_card"));
    }
    _ => panic!("unexpected choice type"),
}
```

## Standard Test Pattern

```rust
#[test]
fn what_ability_does() {
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

## Tested Abilities (602 unique texts, work in progress)

| # | Pattern | JP text summary | Card | Status |
|---|---------|----------------|------|--------|
| 1 | 起動 + move_cards (self stage→discard, search discard→hand live) | 起動：このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。 | 黒澤ルビィ PL!S-bp2-009-R | ✅ |
| 2 | 起動 + move_cards (self stage→discard, search discard→hand member) | 起動：このメンバーをステージから控え室に置く：自分の控え室からメンバーカードを1枚手札に加える。 | 園田海未 PL!-sd1-002-SD | ✅ |
| 3 | 登場 + look_and_select (optional hand→discard, peek top 3, choose 1) | 登場：手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。 | 園田海未 PL!-sd1-011-SD | ✅ |
| 4 | 登場 + sequential (draw 1, discard 1) | 登場：カードを1枚引き、手札を1枚控え室に置く。 | TBD | ⬜ |
| 5 | ライブ開始時 + pay_energy + gain_resource | ライブ開始時[E]支払ってもよい：ライブ終了まで、[B][B]を得る。 | TBD | ⬜ |
| 6 | 登場 + look_and_select (peek top 3, reorder on deck) | 登場：自分のデッキの上からカードを3枚見る。その中から好きな数を好きな順番でデッキの上に置き、残りを控え室に置く。 | TBD | ⬜ |
| ... | Continue down the list of 602 unique texts | | | ⬜ |

## Bugs Found & Fixed During Tests

| Bug | Location | How the test caught it |
|-----|----------|----------------------|
| `allow_skip` path missing `"discard"` handler | `choice.rs:97` | Test #1: selecting a discard card caused infinite loop (softlock) |
| `matching_indices` limits to `count` results | `move_cards.rs:20` | Test #1: choice prompt never fired for count=1 effects |
| `prompt_card_choice` hardcodes `card_type: None` | `move_cards.rs:48` | Test #2: choice didn't filter to member_card |
| `provide_ability_choice_result` re-runs `resolve_ability` after choice | `game_state.rs:807` | Test #3: optional cost caused infinite re-prompt loop |
| `looked_at_cards` not persisted between resolver instances | `turn.rs`, `game_state.rs` | Test #3: looked-at cards lost when second resolver was created |
| `peek_top` vs `deck_top` source mismatch | `effects.rs:918` | Test #3: `execute_look_at` matched `"deck"` but ability JSON uses `"deck_top"` |

## Debugging

- **"play_to_stage failed: Could not pay N energy"** — Check `game.give_energy(N)`.
- **"No pending choice"** — Auto-resolved (only 1 valid target). Add more valid targets.
- **Wrong index in `select_indices`** — Indices refer to original zone positions. Print the zone to check.
- **Test passed but behavior seems wrong** — The test encodes expected behavior from the JP text. Check `rules/rules.txt` and `cards/qa_data.json` for the actual rule.
- **Missing JSON field in abilities.json** — If the test needs a field that isn't being parsed (e.g., `cost_limit` vs `total_cost_limit`), fix the parser in `card_loader.rs` or `card_parser.py`.

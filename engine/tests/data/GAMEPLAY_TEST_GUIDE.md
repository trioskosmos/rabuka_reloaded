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

## How to Write a Test

1. Find the ability pattern in the table below (sorted by card count, most common first)
2. Check what the Japanese text says the ability should do
3. Pick a real card from the `cards` array in `abilities.json`
4. Check the card's cost in `cards.json`
5. Call `game.give_energy(cost + 1)` (1 extra for safety)
6. Set up filler cards in relevant zones (discard, deck, stage)
7. Play the card, trigger the ability, make choices, assert state
8. If you hit an engine bug, fix it, then continue

**Rules for writing tests:**
- Use `game.id("card_no")` — never use raw i16 IDs
- Always put extra filler cards in zones (avoids edge cases)
- `give_energy(N)` works for any N — high-cost cards just need more
- For opponent's board: `game.state.player2.stage.set_area(area, id)`
- For choices with multiple triggers: handle them in order (cost first, then effect)
- If `has_pending_choice()` is false after activating, either:
  - The effect auto-resolved (exactly 1 valid target) — this is correct behavior
  - Or there was an error (check stderr for "Failed to execute" messages)

## Test Plan (most common unique ability texts)

| # | JP text summary | Card | Cost | Energy needed | Key engine features tested |
|---|----------------|------|------|---------------|---------------------------|
| 1 | 起動：このメンバーをステージ→控え室：控え室→ライブ1→手札 | 黒澤ルビィ PL!S-bp2-009-R | 2 | 3 | self_cost stage→discard, discard→hand with card_type filter ✅ |
| 2 | 起動：このメンバーをステージ→控え室：控え室→メンバー1→手札 | 園田海未 PL!-sd1-002-SD | 2 | 3 | Same as #1 but member_card filter ✅ |
| 3 | 登場：手札→控え室(opt)：デッキ上3見る→1手札→残り控え室 | 園田海未 PL!-sd1-011-SD | 4 | 5 | optional cost, look_and_select, looked_at_cards persistence ✅ |
| 4 | 登場：1引き、1控え室 | 中須かすみ PL!N-bp1-019-PR | 4 | 5 | sequential (draw→discard) ✅ |
| 5-9 | *(being worked on by other dev)* | | | | |
| 10 | 登場/ライブ開始時：自身ウェイト(opt)：相手のコスト4以下→ウェイト | 星空凛 PL!-PR-007-PR | 4 | 5 | change_state for members, optional change_state cost, opponent targeting ✅ |
| 11 | 登場：手札→控え室(opt)：エネルギー置場→エネルギー1→ウェイト | 唐可可 PL!SP-PR-004-PR | 4 | 5 | energy deck→zone, wait state ✅ |
| 12 | 登場：手札→控え室(opt)：控え室→虹ヶ咲ライブ1→手札 | 上原歩夢 PL!N-bp1-003-R＋ | 10 | 11 | group-filtered search ✅ |
| 13 | 登場：2引き、1控え室 | 夕霧綴理 PL!HS-bp1-006-R＋ | 11 | 12 | sequential (draw 2→discard 1) ✅ |
| 14 | ライブ開始時：ハート色選択→付与 | *(deferred)* | - | - | ⏳ |
| 15 | 登場：2引き、2控え室 | 上原歩夢 PL!N-PR-005-PR | 13 | 14 | sequential (draw 2→discard 2) ✅ |
| 16 | 起動：EE支払う：控え室→ライブ1→手札 | *(TBD)* | ? | ? | pay_energy cost (non-optional) + move_cards |
| 17 | ライブ成功時：2引き、1控え室 | *(TBD)* | - | - | LiveSuccess trigger + sequential |
| 18 | 登場：手札→控え室(opt)：コスト4以下のメンバー2→ウェイト | *(TBD)* | ? | ? | change_state with cost_limit and count=2 |
| 19 | 起動：EE支払う：控え室→メンバー1→手札 | *(TBD)* | ? | ? | pay_energy + move_cards member search |
| 20 | ライブ開始時：E支払う(opt)：ハート色指定→付与 | *(TBD)* | - | - | specify_heart_color + gain_resource |
| 21 | 起動：手札→控え室：2エネルギーアクティブ化 | *(TBD)* | ? | ? | move_cards cost, activate energy |
| 22 | 常時：条件→全体強化 | *(TBD)* | - | - | constant (always-on) modifier |
| 23 | 登場：EE支払う(opt)：左サイドなら2引く | *(TBD)* | ? | ? | conditional effect (position check) |
| 24 | 起動：メンバー公開→コスト合計でスコア変動 | *(TBD)* | ? | ? | reveal + modify_score |
| 25 | 登場：エネルギー11+なら控え室→ライブ1→手札 | *(TBD)* | ? | ? | conditional effect (energy count check) |
| 26 | 登場：2エネルギーアクティブ化 | *(TBD)* | ? | ? | change_state (activate energy) |
| 27 | 常時：全員異名+異グループ→スコア+1 | *(TBD)* | - | - | complex constant modifier |
| 28 | 起動：E支払う：控え室→コスト4以下メンバー1→手札 | *(TBD)* | ? | ? | pay_energy + cost_limit filter |
| 29 | 起動：EEE支払う：控え室→蓮ノ空ライブ1→手札 | *(TBD)* | ? | ? | pay_energy + group filter |
| 30 | ライブ開始時：E支払う(opt)：ライブカード毎に[B] | *(TBD)* | - | - | per_unit gain_resource |

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

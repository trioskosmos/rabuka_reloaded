# BP07 G7 — ミア・テイラー `PL!N-bp7-011-R＋` ab#0: deck→discard zone-source trigger

## The ability (Japanese text is the spec)

> {{jidou.png|自動}}**このカードがデッキから控え室に置かれたとき**、手札を1枚控え室に置いてもよい。そうしたとき、控え室からこのカードを手札に加える。

English: **When this card is placed from your deck to your discard**, you may
discard 1 card from your hand. If you do, add this card from your discard to your hand.

## The critical detail: `デッキから` (from deck)

The trigger is **deck → discard only**. The word `デッキから` ("from the deck")
constrains the **source zone** to `deck`. Any OTHER way the card enters the
discard must NOT trigger the ability — in particular **hand → discard**, which is
the source of an infinite loop if the parser drops the source.

### The bug this guards against

The parsed condition used to look like:

```json
"trigger_event": { "type": "zone_change", "destination": "discard" }
```

No `source`. With `source` missing, the engine treats the trigger as "any card
entering discard" (`source_zone.is_empty() → matches all`). Consequence:

1. A ミア copy sits in **discard**; another copy sits in **hand**.
2. You discard the hand copy → hand→discard → **wrongly triggers** ab#0.
3. ab#0 discards a card and recovers ミア to hand → you discard it again → **repeat forever**.

After the fix the parser emits `source: "deck"`:

```json
"trigger_event": { "type": "zone_change", "source": "deck", "destination": "discard" }
```

Now hand→discard (`source_zone="hand"`) no longer matches the `deck` filter, and
the loop cannot start.

### Second bug (found by real-card testing): DeckTop vs Deck

The parser fix alone was **not enough**. Real mill abilities (e.g. 黒澤ダイヤ's
登場 "自分のデッキの上から5枚を控え室に置く") move cards `deck_top → discard` and
record `source_zone=ZoneId::DeckTop`. But ミア's condition asks for `deck`. With a
strict `==` compare, `DeckTop != Deck`, so a **real mill never triggered ミア**.

Fixed with `ZoneId::matches_source` (engine/src/core/types.rs), one-directional
aliasing:

- condition `deck` matches `Deck` | `DeckTop` | `DeckBottom` — a card milled off
  the top is still "placed from the deck" (the ミア rule).
- condition `deck_top` matches **only** `DeckTop`; condition `deck_bottom`
  matches **only** `DeckBottom` — a top-of-deck trigger must NOT fire for a
  bottom-of-deck move.
- `discard`/`waitroom` remain equivalent (both directions).

All four source filters in `resolve_moved_cards_source` use it. This makes the
trigger correct for the **real** engine path, not just the hand-injected events
the earlier tests used.

## Engine mechanics

- Zone-change autos read `game.state.turn_movements`; each entry has
  `source_zone` + `dest_zone`. The condition filter in
  `resolve_moved_cards_source` requires `m.source_zone == "deck"` (see
  engine/src/ability/condition/card.rs:1967-2066).
- The real engine records the correct source everywhere:
  - hand→discard cost → `source="hand"` (ability/cost.rs:1259)
  - `MoveCards::finalize` → the real `source.to_string()` (ability/move_cards.rs:2224)
- `self_target` restricts to the activating card, so ab#0 only considers ミア
  itself, not any other card that happens to move.

## Real cards used to exercise the zone moves (not test-only injections)

| card_no | ability (Japanese) | zone move | role |
|---|---|---|---|
| `PL!N-bp7-011-R＋` ミア・テイラー ab#0 | 自動 このカードが**デッキから控え室に置かれたとき**… | trigger | the SUT |
| `PL!S-sd1-013-SD` 黒澤ダイヤ ab#0 | 登場 自分のデッキの上からカードを5枚控え室に置く | **deck_top → discard** | positive: real mill puts ミア into discard |
| `PL!N-bp1-014-PRproteinbar` 中須かすみ ab#0 | 登場 カードを1枚引き、手札を1枚控え室に置く | **hand → discard** | negative: discarding a hand ミア copy |

Driving these two real abilities (via `trigger_auto_ability`, pattern F) tests the
zone-source distinction through the real engine recording path, not a faked event.

## Test matrix (each row = a `#[test]`, real-card, real abilities)

All positive/negative cases drive the **real** ability through the real engine
(blackbox, no hand-injected movement events):

| test | Japanese basis | scenario | expected |
|---|---|---|---|
| `mia_real_dia_mill_triggers_ab0` | このカードが**デッキから**控え室に置かれたとき | 黒澤ダイヤ's real 登場 mills 5 (`DeckTop→Discard`); ミア on deck top | ab#0 fires; accept → ミア recovered to hand, 1 hand card discarded |
| `mia_real_dia_mill_decline_no_recover` | 手札を1枚控え室に置いて**もよい** | same mill; decline the optional | no hand discard, ミア stays in discard |
| `mia_real_hand_discard_does_not_trigger` | only デッキから, not 手札から | 中須かすみ's real 登場 draw-1 + discard-1; the discard selects ミア from hand | ab#0 does NOT fire; ミア stays in discard, not recovered |

The decisive assertion in the last test: ミア discarded **from hand** must NOT
present her `conditional_optional` at all. Before the `source` fix (parser) and
the `DeckTop` aliasing fix (engine), this test fails because the engine offers
the recover on a hand discard.

## Anti-loop invariant

The strongest assertion is the **no-infinite-loop** test: with ミア in both discard
and hand, a real hand→discard of the hand copy must leave both copies in the
discard and produce **no** pending `conditional_optional` choice. A buggy parser
(no `source`) makes this test fail, because the engine offers the recover.

## Verification

```bash
# parser fix lands the source field
cd cards && python ability_extraction/extract_card_abilities.py
python -c "import json;d=json.load(open('abilities.json',encoding='utf-8'));[print(json.dumps(a['effect'].get('condition'),ensure_ascii=False)) for a in d['unique_abilities'] if any('PL!N-bp7-011' in c for c in a.get('cards',[])) and a.get('triggers')=='自動']"

# engine suite (all 3 real-card tests in this module)
cd ../../engine && cargo test --test run_all mia_real_ -- --nocapture
```
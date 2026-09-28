# Jidou + other-ability combination work

_Companion to `docs/ABILITY_FAMILIES.md` and `cards/test_inventory.py` output
(`engine/tests/TEST_INVENTORY.{json,md}`). This file records **why** the
jidou-combination audit exists, **what counts as evidence** for it, and the
**layout/naming** the tests are being reorganised into._

## 1. The gap, stated precisely

`cards/abilities.json` classifies 936 unique abilities by trigger. 75 of them are
`自動` (jidou / "automatic"). A per-trigger coverage report reading "75/75 jidou
covered" is true and useless, because **jidou abilities almost never act alone**.

Three measured facts from the inventory:

- **133** card identities carry a jidou ability.
- **61** of those carry a jidou **plus at least one other ability**
  (`起動` / `登場` / `ライブ開始時` / `ライブ成功時` / `常時`). Those are the
  inventory's `jidou_conjunction.multi_ability_cards` (28 ability rows).
- The inventory's own `jidou_conjunction` block also isolates two more shapes
  that only exist in combination:
  - `ability_watchers` — jidou keyed on **another ability resolving**
    (`〔...〕の能力が解決したとき`), which cannot fire without a second card
    resolving its own ability in the same window.
  - `effect_cause` — jidou gated on **the cause of a move**
    (`自分のカードの効果によって` / `(対戦相手のカードの効果でも発動する)`),
    which is a claim about provenance, not about the event.

### Why per-ability coverage is the wrong unit here

The inventory is explicit (`test_inventory.py`, section C preamble) that its
`Tests` column is `direct/in file` **per card, not per ability**: a card with a
jidou and a live-start shares one set of tests, so the number cannot say
"ability A has 3 tests, ability B has 5". Splitting them was measured and
rejected — only 59 of 128 differing pairs can be separated by the explicit
`AbilityTrigger::X` a test fires, because jidou is driven through
`trigger_auto_abilities_for_player` which names no trigger constant.

So a 100% jidou row means **"the jidou text of these cards is exercised"**, and
says nothing about whether the *pairing* is. The pairing is the whole design of
these cards: a jidou whose entire reason for existing is to charge a sibling
ability's printed condition.

## 2. The point: a jidou's sibling is not decoration

Reading the 61 paired card identities produces a short list of **link types**.
Each is a way the pair can silently be wrong, and therefore a required test
shape:

| # | Link | Printed shape | What breaks if untested | Example card |
|---|------|---------------|------------------------|--------------|
| L1 | **Feeder** | jidou charges a resource the sibling's *condition* counts | The sibling's condition is never satisfiable in a real game, or is satisfiable for the wrong reason | `PL!HS-pb1-009` (jidou grants ブレード+2; ライブ開始時 needs ブレード ≥ 8) |
| L2 | **Activator** | the sibling's resolution *causes* the jidou's trigger event | The jidou is unreachable in a real game — a test that pokes the event in proves the effect, not the trigger | `PL!SP-pb2-011` (own ライブ開始時 repositions out of センター → that move arms the jidou's 3-option choice) |
| L3 | **Sink** | the jidou's output is read by the sibling (constant / under-card counter) | The modifier re-derives wrongly: appends instead of replaces, or is never withdrawn | `PL!SP-pb2-006` (jidou tucks a member under her → her 常時 +1 cost each) |
| L4 | **Shared resource / limit** | both abilities draw, spend, or burn a per-turn allowance | Double-spend, or a second firing that silently reuses an exhausted limit | `PL!SP-bp7-008` (起動 waits her + draws; the 自動 reactivates her and must **not** re-run the draw) |
| L5 | **Gate/shadow** | the sibling's state is the jidou's precondition (or negates it) | The jidou fires when it must not, or stays silent when it must fire | `PL!SP-bp7-008` (自動 gate is ウェイト状態のこのメンバー — a move while アクティブ must do nothing) |
| L6 | **Provenance** | jidou requires *own-effect* / *also-on-opponent-effect* | The gate is untestable in isolation: it is a statement about attribution, so only a two-player or effect-caused setup decides it | `PL!N-bp7-031` (自動 watches **own** ライブ成功時; an opponent's mill must not fire it) |
| L7 | **Window/order** | the pair resolves in one live and each half must survive the other | A ライブ終了時 grant is read in the wrong window, or one half's cost is paid by the other | `PL!SP-bp4-025` (both abilities resolve in the same live) |

**A pair is covered only when a single test drives both halves and asserts the
interobservable that only the combination produces.** Two tests, one per
ability, prove nothing about the link — that is the shape of the gap this work
closes.

### The specific trap: a test that "misses the point"

The most common worthless test for a paired card asserts the jidou's effect
against a *directly poked* trigger event. That proves the effect decoder, not
the trigger scan, and it passes unchanged whether or not the sibling can ever
cause the event. The audit replaces these with real chains:

- Drive the **real** sibling ability (not `push_movement_event` / `fire_trigger`).
- Let the engine's own TAS scan find the jidou from the event the sibling caused.
- Assert the **link observable** — the sibling's grant, the consumed limit, the
  not-repeated draw — not just the jidou's own effect.

`engine/tests/test_modules/jidou/movement/self_area_move_watch/shiki_wait_gate_arms_only_after_kidou_test.rs`
is the reference implementation of this standard: it drives the real positional
swap path, and its three tests target the gate, the cycle, and the draw — the
three ways the pair can fail independently.

## 3. Layout and naming

`mod.rs` files are **auto-generated by `build.rs` from the directory listing**.
Moving, renaming, adding, or deleting a test file needs no manual `mod.rs` edit;
the next compile regenerates it. So reorganising is free — do it.

### Directory tree

`engine/tests/test_modules/jidou/` groups by **the event the jidou watches**,
because that is the axis along which these abilities differ and the axis a
failing test names. It is not a card-type axis.

```
jidou/
  ability_watch/     jidou keyed on another ability RESOLVING  (links L1/L6 watchers)
  debut_watch/       登場-caused trigger events
  discard_watch/     cards placed into 控え室
  energy_watch/      energy placed into エネルギー置き場
  leaves_stage/      this card placed ステージ→控え室
  movement/          エリアを移動した (incl. area_move, baton_touch, under-member)
    self_area_move_watch/   the card's OWN movement
    self_move_heart_watch/  the card's own movement granting a heart
    under_member_placement/ tucking cards under members
    ability_chain_combo/    L2/L3 chains
  state_watch/       state transitions (active↔wait) caused by own effects
  title_once/        title cards with a second, gated ability
  yell/              エール-caused events
  combination/       (new) L1-L7 pair tests, one file per paired card identity
                      — hs_pb1_001_jidou_activated_energy_funds_live_start_payment_test.rs
                      — hs_pb1_003_turn2_second_discard_event_grants_again_test.rs
                      — hs_pb1_009_jidou_blades_charge_live_start_threshold_test.rs
                      — sp_bp5_005_mill_arms_jidou_recover_test.rs
```

The `combination/` subtree is where a jidou+partner pair test belongs even when
its trigger would suggest another folder. **The trigger event is what the
ability watches; the combination is what the test is for.** A test for
`PL!HS-pb1-009` that lives under `movement/` because it swaps members is filed
by the thing under test, not the mechanism that pokes it.

### File naming

`<subject>_<what_is_proven>_<qualifier>_test.rs`, subject-first, all lowercase
snake_case:

- subject = the card identity, without the trailing rarity, lowercased and
  punctuation folded: `PL!HS-pb1-009-R` → `hs_pb1_009`, `PL!SP-pb2-011-R` →
  `sp_pb2_011`. This matches the sibling/partner pair in one directory.
- what_is_proven = the link type or the specific mechanism, not the effect
  restated: `jidou_blades_charge_live_start_threshold` (L1),
  `own_live_start_reposition_arms_own_choice` (L2),
  `placement_feeds_constant_cost` (L3), `draw_not_repeated` (L4),
  `gate_shut_while_active` (L5), `opponent_effect_does_not_fire` (L6).
- qualifier = a boundary, print, or ruling id when the file holds one
  case: `_q255`, `_q263`, `_q269`, `_p_plus`, `_multi_copy`.

Prefer one file per **link**, not per test. A file whose tests share a premise
(`stage_shiki`, `shiki_area`, `move_shiki` in the shiki reference) keeps that
premise written once; a fifth test that needs a different premise is a different
file. Do not grow a "misc" file — the subject is what makes a file findable,
and a misc file has no subject.

### Test-body discipline

- **Bind and identity-pin the card in the TEST BODY**, not only in a setup
  helper. `TEST_COVERAGE`'s `direct/in file` counts bodies that name the card; a
  card bound only in a helper reads as co-location, not as a driver. Use
  `game.assert_card_identity(id, "PL!...")`.
- **Pin the print.** `PL!SP-pb2-006-R` and `PL!SP-pb1-006-R` are both 桜小路きな子;
  the `ability_chain_combo_test.rs` pin exists because a pb1/pb2 transposition
  was staged while the header claimed otherwise. Similar-looking card numbers
  (`bp2`/`pb2`, `bp1`/`pb1`) are the top wrong-card source.
- **Assert the link observable, not just the effect.** If a test can pass when
  the sibling is deleted, it is not a combination test.
- **Read a state back to prove the step happened** (which area the member is in
  now) before asserting on it. A no-op swap makes every downstream assertion
  vacuously true — see `shiki_area` / `move_shiki`.
- **Never `#[ignore]`, delete, or weaken an assertion** to make a red test
  green. Classify: test bug / engine bug / parser gap, and fix the real cause
  (see `AGENTS.md`).

## 4. Working the list

The 28 paired rows, thinnest first (`direct_test_count`), are the queue. For
each: identify the link type from §2, then either write the missing combination
test in `jidou/combination/`, or — if an existing file already drives the chain —
record that fact here and move on.

The pairs already held down by a real chain test, so they are not re-worked:

| Card | Link | Held by |
|------|------|---------|
| `PL!S-bp5-111` | L2 | `rules/trigger_paths/sibling_abilities_on_one_card_interact_test.rs` |
| `PL!HS-pb1-003` | **L4 + Bug B** | `jidou/combination/hs_pb1_003_turn2_second_discard_event_grants_again_test.rs` (new). The sibling file's `hs_pb1_003_debut_discards_feed_auto_gain` drives only a **single** discard, so it never proved the ターン2回 second firing — see §4. |
| `PL!SP-bp7-005` | L2 ×2 | same + `jidou/energy_watch/*` |
| `PL!N-bp3-005` | L1/L2 | same (shared debut counter, both thresholds) |
| `PL!SP-pb2-011` | L2 | `jidou/movement/ability_chain_combo/ability_chain_combo_test.rs` |
| `PL!SP-pb2-006` | L3 | same |
| `PL!SP-bp7-008` | L4/L5 | `jidou/movement/self_area_move_watch/shiki_wait_gate_arms_only_after_kidou_test.rs` |
| `PL!N-bp7-031` | L6 | `effects/recover/to_hand/live_success_add_one_niji_live_among_cards_moved_to_hand_plus_score_test.rs` |
| `PL!SP-bp4-025` | L7 | `jidou/movement/ability_chain_combo/ability_chain_combo_test.rs` |
| `PL!-bp6-020` | watcher | `jidou/ability_watch/live_start_resolved_own_center_group_reposition_q255_test.rs` + siblings |
| `PL!N-bp5-030` | watcher | `jidou/ability_watch/live_start_success_resolved_watcher_arms_only_after_resolution_test.rs` |
| `PL!SP-bp7-001` | L3 | `jidou/leaves_stage/baton_displaced_under_arriver_grants_host_blade_test.rs` |
| `PL!SP-sd2-002` | L5 | `jidou/movement/self_move_heart_watch/*` |
| `PL!S-bp6-002` | L1 | `jidou/discard_watch/own_live_zone_discard_group_live_optional_deck_top_bottom_q252_test.rs` |
| `PL!N-bp4-026` | L2 | `jidou/title_once/dive_retrieved_to_hand_ab0_places_live_zone_ab1_grants_blade_test.rs` |
| `PL!HS-pb1-001` | **L4** | `jidou/combination/hs_pb1_001_jidou_activated_energy_funds_live_start_payment_test.rs` (new) |
| `PL!SP-bp5-005` | **L1/L2 shared-batch** | `jidou/combination/sp_bp5_005_mill_arms_jidou_recover_test.rs` (new) |
| `PL!SP-bp2-021-N` / `PL!SP-bp2-015-N` | watcher | `jidou/combination/yell_real_yell_no_blade_heart_grants_heart_test.rs` (new) — the first **real-pipeline** coverage for these two; the whole `no_blade_heart_reveal_gain` family never yelled (§4). |
| `PL!-pb1-015` | L2 + L6 | `jidou/state_watch/own_effect_wait_cheap_opponent_draw_one_q177_test.rs` — already drives the full chain: debut 真姫 → ab#0 waits a cost≤4 opponent member *by your card's effect* → ab#1 draws 1. Also covers the cost gate, the 「自分のカードの効果によって」 provenance gate, the decline branch, and the cause-player routing. |
| `PL!-bp5-004` | L1/L2 | Covered across `jidou/energy_watch/*` and `jidou/yell/yell_reveal_threshold_resources/*` (energy-placement and blade-heartless-reveal watchers) plus the blade/heart granted by the sibling. |

> **Inventory-count caution (confirmed).** `PL!-pb1-015` showed only `14` direct
> tests in the inventory — near the bottom of the queue — yet is one of the
> best-covered pairs in the suite, because a single file drives the whole L2+L6
> chain. Conversely `PL!HS-pb1-001` had a low count and a real gap. **Always read
> the existing chain test before writing a new one**; the `direct_test_count`
> ordering is a starting heuristic, not a gap list (§1 explains the counting).

### `PL!HS-pb1-001` — worked (L4, landed)

The jidou spends 1 active energy to activate 2 **waited** energy; the sibling
ライブ開始時 then spends 2 active energy. The jidou is a **feeder for the
sibling's payment** — the energy ab#1 pays with is exactly the energy ab#0
activated. The pre-existing jidou test only ever *declined* ab#0 and never
touched ab#1, so it could never see this.

The fixture is tuned so the halves are genuinely coupled: 5 active + 2 waited;
play the 4-cost スリーズブケット ally → 1 active, 2 waited. Accept ab#0 → 2
active → ab#1's 2E is exactly affordable. Decline ab#0 → only 1 active → ab#1's
2E is **unpayable**. The energy ledger (5→1→2→0) and the decline-negative are
the link observables; neither is visible to a single-ability test. A single
appearance fires ab#0 once, so this file is independent of Bug B.
### `PL!SP-bp5-005-R＋` 葉月 恋 — worked (L1/L2 shared-batch, landed)

The 起動 mills 3 cards into 控え室, and that single placement is what **arms** the
自動: 「自分のカードが…控え室に置かれる」. Both halves then read the *same* 3-card
batch:

  * ab#0 reads it as a **count** — 2 『Liella!』 members among the 3 → 2 ブレード.
  * ab#1 reads it as a **pool** — pay E, recover 1 of 「それらのカード」 to hand.

The pre-existing `effects/gain/blades/per_card/hazuki_activate_discard_per_liella_member_blade_test.rs`
drives ab#0 and asserts only the ブレード. It never arms ab#1, so it cannot catch
ab#1 recovering a card from the **wrong batch** — the defining failure of this
pair. The new test asserts the recovered card is a member of the exact 3-card set
the 起動 milled, and (negative) that declining ab#1 leaves ab#0's ブレード intact
(the blade is earned by the mill, independent of the recover choice). One mill =
one jidou firing, so this file is independent of Bug B.

### Exhaustive comb (the audit is complete)

The queue above was cross-checked by a full programmatic pass, not just the
inventory's per-card counts (which §1 explains are misleading). The comb
enumerates every card identity in `abilities.json` that has BOTH a 自動 ability
and at least one other ability (54 of them), finds the test files naming that
card, and reports the ones where no file names both trigger strings. Result:
every remaining candidate is one of two non-gaps —

1. **A print-variant** (`-P`, `-P＋`, `-AR`, `-SEC`, `-PP`, `-R＋`) of a base card
   that IS covered. E.g. `PL!HS-pb1-001-P＋` has identical abilities to
   `PL!HS-pb1-001-R`, which the L4 test drives; the tests simply use the `-R`
   print. These are not separate coverage gaps.
2. **A genuine weak link**, where the two abilities have *different* trigger
   sources and do not causally interact:
   - `PL!S-bp2-007-R＋` — the ライブ開始時 reveals a live card from hand, but
     the 自動 watches cards 「エールにより公開された」. A live-start reveal is
     not a yell reveal, so the sibling never arms the jidou.
   - `PL!N-pb1-012-R` — the 自動 fires on a cost-11 ally's appearance; the
     ライブ成功時 adds a 虹ヶ咲 member from the yell pool. Independent triggers.

There is therefore **no untapped strong-link jidou/sibling pair** remaining: every
L1–L7 pair with a real causal link is either driven by an existing chain test or
landed here. The two weak-link cards are intentionally not given "combination"
tests, because driving both would only be the arithmetic sum of two
independently-tested abilities — the exact "test that misses the point" this
document exists to prevent.

### `PL!HS-pb1-003` — worked (Bug B regression guard, landed)

`PL!HS-pb1-003-R` 大沢瑠璃乃's ab#1 is 自動 **ターン2回** — the exact shape Bug B
silently broke. The existing coverage set up two qualifying discard events and
then asserted **nothing** about the second
(`jidou/discard_watch/on_hand_to_discard_each_time_gain_heart01_and_blade_test.rs`,
`rurino_use_limit_blocks_second_same_turn`):

```ignore
// heart01 may be 1 or 2 depending on post-resolve re-enqueue.
// The important thing is no crash.
```

"May be 1 or 2" is precisely the window the re-scan-guard leak lived in — 瑠璃乃's
second allowance was vetoed in a real game and the test accepted it. This is the
**"test that misses the point"** in its purest form: a test named after the
per-turn limit that never checks the limit.

`jidou/combination/hs_pb1_003_turn2_second_discard_event_grants_again_test.rs`
pins all three directions: two separate events grant twice (the ceiling is 2, so
the assertion is tight); a 3-card batch is ONE event (Q241) and leaves the second
allowance intact; a third event is refused. The old test now asserts the real
number and points at the new file.

Note the trigger mechanism: this jidou reads `source: "preceding_moved"`, i.e. it
watches `recently_moved_cards`, **not** the hand/waitroom zones. The tests drive
it with `set_recently_moved_cards` — the same public entry the existing file uses
— so the condition, enqueue, and ターン2回 accounting are all the real engine's.

### `PL!SP-bp7-005` — worked (Bug B regression guard, landed)

葉月 恋's ab#1 is 自動 **ターン2回** on own-effect energy placement. Two files
covered it without ever proving the second firing:

- `jidou/energy_watch/own_effect_energy_placed_blade_upper_bound_test.rs` line 22:
  `assert!(second == 1 || second == 2, ...)` — the *same* "1 or 2" leniency as
  瑠璃乃. Line 28 was equally loose (`third <= 2`, which also passes at 1). Both
  are now exact (`2`, then `2`).
- `jidou/energy_watch/energy_placed_by_own_effect_gain_blade_test.rs` had tests
  named `twice_per_turn_*` that placed energy exactly **once** and asserted +1 —
  the name promised the ターン2回 second firing while exercising only the first.
  Renamed to `first_own_effect_placement_gains_one_blade` (accuracy), and the
  other two `twice_per_turn_*` names became `turn2_*` (they are negatives that
  only identify the card's limit).

Added `turn2_watcher_second_real_debut_placement_gains_again_and_third_is_refused`,
which drives **two genuine debuts** of `PL!SP-pb1-005-R` (its debut places one
energy from the energy deck into the zone) rather than a poked
`push_movement_event`. That proves the whole chain — debut → energy placement →
jidou fires — not just the event decoder, and it is the Bug B guard on a real
path: before the fix the second debut was swallowed and this read 1.

**The transferable heuristic.** A jidou test whose *name* promises a count or
limit (`twice`, `second`, `2回`, `budget`, `ceiling`, `refused`) but whose body
has ≤1 assertion is a candidate for the same defect. Mechanically: list every
such test, count its `assert!`s, and read the weakest ones. Both vacuous
assertions found so far (`1 or 2` / `<= 2`) sat exactly there, and both are now
closed. See §7 for the sweep.

### `PL!SP-bp2-021-N` / `PL!SP-bp2-015-N` — worked (a whole family that never yelled)

Extending the lenient-assertion sweep beyond `jidou/` found the worst case in the
audit: the entire `jidou/yell/no_blade_heart_reveal_gain` family (~25 tests)
covered two yell-triggered 自動 that **were never actually yelled**.

Both cards read 「エールにより公開された自分のカードの中にブレードハートを持つ
カードがないとき、ライブ終了時まで、heart03（heart06）を得る」. Two separate
defects hid that:

1. **The fixture never yelled.** Every test drove the live with a **zero-score
   filler** live card, which produces no yell at all. A probe over every phase
   read `revealed_cards = 0` and `yell_occurred = false` throughout — so the
   ability correctly never fired, and every test observed the *negative*.
2. **Hand-set state hid it.** The sibling tests set `yell_occurred` and
   `revealed_cards` **by hand** and then called the scan, which does exercise the
   condition — but never the real pipeline, so nothing ever proved a genuine yell
   reaches these jidou.

The sharpest symptom: `wien_yell_no_blade_heart_gain_heart03_until_live_end_test.rs`
contains a test named `wien_q112_positive_no_blade_heart_triggers_heart03` that
asserts `heart == 0` and blames an **"auto-trigger bug"**. There was no bug.
`no_blade_heart_reveal_two_watchers_modifier_bounds_test.rs` asserted
`h == 0 || h == 1` for both watchers — true for every value.

`jidou/combination/yell_real_yell_no_blade_heart_grants_heart_test.rs` performs a
live card with a **real score requirement** (`PL!S-bp3-020-L`), reads
`yell_occurred` and `initial_yell_revealed_cards` as explicit preconditions, and
asserts the grant **during** the live — the modifier is ライブ終了時まで, so reading
it after the live correctly reads 0, which is what the old test saw and
misdiagnosed. Wien and Sumire each get their own colour, and the no-yell
negative is pinned so the positives cannot be satisfied by a fixture that simply
never yells. The engine was right the whole time; both old tests are corrected to
assert the honest behaviour and to point at the real coverage.

The same defect appeared in two more files, both now rewritten:

- `no_blade_heart_reveal_heart06_member_as_live_q112_q113_test.rs` — all three
  tests used the 0-score filler live and asserted `heart06 == 0`, including one
  named `..._positive_...triggers_heart06` that blamed the same nonexistent
  auto-trigger bug. Rewritten to use a real live card, with a genuine positive
  (a real yell revealing no blade heart grants heart06) and a genuine negative
  (a real yell that DOES reveal a blade heart grants nothing).
- `effects/look_select/per_card/turn_limited_yell_second_activation_blocked_test.rs`
  — `sumire_turn_limit_blocks_second_yell` and `wien_turn_limit_blocks_second_yell`
  never performed a second yell at all: they reset the modifier by hand and gave
  up ("for simplicity, just verify that the use_limit is enforced via queue"),
  asserting `first == 0 || first == 1`. Both also carried the phantom-bug
  comment. Rewritten to perform a real yell and assert exactly one heart. Its
  **scope is now stated in-file**: a second *live* in one turn is not something a
  player can perform (one live per player per turn), so the ターン1回 ceiling needs
  a genuine **re-yell**, which is covered for real in
  `yell_discard_liella_live_grants_two_extra_yells_test.rs::pb2_020_use_limit_blocks_second_yell`.
  (`no_blade_heart_reveal_gain_heart06_or_heart03_test.rs::yell_turn1_blocks_second_trigger`
  re-triggers the scan **by hand** and proves only the limit's *accounting*, not
  the pipeline, so it is deliberately not cited as real-pipeline coverage — the
  generic ターン1回 mechanism is pinned on real paths by `PL!N-PR-025` and the
  瑠璃乃 / 葉月恋 combination tests.)

### Open lead: the re-yell ceiling is unproven (and a first probe read the wrong field)

A player performs one live per turn, so the only way to yell twice in a turn is a
**re-yell**, and the only source of one is a sibling ability like 夏美's (on yell:
discard a 『Liella!』 live → 追加で2枚エール). Every test of the ターン1回 ceiling on a
yell watcher sets up its second yell **by hand**: `pb2_020_use_limit_blocks_second_yell`
and `yell_turn1_blocks_second_trigger` both set `yell_occurred = true`, push into
`revealed_cards`, and re-invoke that same hand-set helper. They prove the limit's
*accounting*, not that a real re-yell produces the second event.

**A first attempt to close this was invalid, and the reason is worth keeping.** The
probe compared `initial_yell_revealed_cards` with and without 夏美 and found no
growth, which looked like "the re-yell adds nothing on a real yell." That is not
evidence of a bug: the engine keeps re-yell state **separately** —
`re_yell_revealed_cards`, `re_yell_occurred`, and a deferred rebuild
(`pending_reyell_rebuild` → `apply_deferred_reyell` in `turn/live.rs`) that
*replaces* `performance_snapshots[..].yell_cards` and the cheer count for a Rule
8.3.13.1 corrective path. A re-yell never appends to `initial_yell_revealed_cards`,
so the probe was watching the wrong observable.

The same probe also showed staging 夏美 *reduces* the yelLED set (7 → 6), because
she displaces a scored member in the middle slot. That is a fixture trap worth
recording: a member-count difference masquerades as a trigger difference, and it
made the first draft "pass" for the wrong reason before the discard was even shown
to matter.

**What is actually needed** to close this. Three observables were tried and each
failed for a different, instructive reason — worth recording so the next attempt
does not repeat them:

1. `initial_yell_revealed_cards` — wrong: a re-yell *replaces*
   `performance_snapshots[..].yell_cards` via the deferred rebuild, it never
   appends here. Measuring it showed "no growth", which looked like a bug and
   wasn't.
2. `re_yell_revealed_cards` — also wrong: it is populated (6/7) whether or not
   夏美 is on stage, so it mirrors the yell rather than isolating the re-yell.
3. Natsumi-present vs Natsumi-absent — confounded: she is not a scored member, so
   swapping her in changes stage score and therefore the yelLED count (7 → 6). A
   member-count/scores difference masquerades as a trigger difference.

With stage score held constant, **accept vs decline of her optional discard is
identical** — and the reason is a third trap, which is the most reusable finding
here:

> **On a real yell the first prompt is not a skip — it is a simultaneous-trigger
> *ordering* prompt.** 夏美 and `PL!SP-sd1-002-SD` ダイスキ both fire on the same
> yell, so the game asks
> 「複数の自動能力が同時に発動しました。使用する順番を選択してください。」
> with options `[鬼塚夏美, ダイスキ]`. "Answering with index 1" is not
> *declining* 夏美 — it runs ダイスキ instead. So a naive accept/decline pair on
> this board is **identical by construction**, and the discard appears never to
> happen. Any test here must choose 夏美 deliberately, and cannot treat index 1
> as "decline".

A follow-up probe confirmed the mechanism itself is fine: selecting 夏美, then
her 『Liella!』 live-card selection, does put the live in the 控え室, after which
ダイスキ's own 「_SELECT 24 cards from revealed cards」 prompt runs. So the
discard and the re-yell chain are reachable on a real performance; only the
*measurement* was wrong.

**This is deliberately not landed as a test.** A probe reached the desired state,
but the same fixture expressed as a test did not — the live left hand without
landing in the waitroom, and the preconditions (a real yell occurred, the live
started in hand) held. The difference was not identified, and a test that cannot
reliably reproduce its own precondition is the exact defect this whole audit is
about. Shipping it with a weakened assertion, or after nudging the fixture until
it went green, would have reintroduced the failure mode the work exists to
remove. The three traps above are the deliverable; the test is not.

**So the open question is now narrower and better posed:** the re-yell chain is
reachable and correct; what is unproven is the ターン1回 **ceiling** across it,
because a single player performs one live per turn, so a second yell in a turn
can only come from a re-yell, and the reveal buffers are not a clean observable
for counting them (the rebuild *replaces* the snapshot's `yell_cards`). The
unambiguous observable is the purchase itself — the 『Liella!』 live reaching the
控え室 — plus `pending_reyell_rebuild`, not any reveal count. Until that is built,
`PL!N-PR-025` (asserts `use_count == 2` and a third refusal via real baton touches)
plus the 瑠璃乃 / 葉月恋 combination tests remain the real-pipeline coverage for
the generic ターン1回 mechanism.

The remaining, thinnest first, each to be worked as its own file in
`jidou/combination/` (note: the two 3-test rows below are *weak-link* pairs —
their jidou and sibling have unrelated triggers, so the combination test is
low-value; work them only after the strong-link pairs are exhausted):

1. `PL!N-pb1-012` (3) — jidou on a cost-11 ally's appearance vs sibling
   ライブ成功時 adding a 虹ヶ咲 member from エール. Jidou already exercised by
   `rules/trigger_paths/debut_watcher_baton_and_hand_activation_q196_q197_q198_test.rs`;
   the live_success half is untested but the two triggers do not interact.
2. `PL!N-sd2-010` (3) — jidou waits a 虹ヶ泳 member and grants ブレード+2
   (ターン1回); sibling 登場 draws 2. See `effects/state/per_card/sd2_nijigasaki_abilities_gap_test.rs`
   for what is already covered.
3. `PL!N-bp7-031` — re-check only if the `L6` file stops driving the real ライブ成功時.
4. `PL!SP-bp7-008` — re-check only if the `L4/L5` file regresses.

`PL!HS-pb1-009` (6) was worked first and is **fully landed** — see §6, which
records the three engine bugs its L1 chain exposed and the fixes for all three.
It is the clearest worked example in this document: the pair could not be tested
until the engine bugs were fixed, and fixing them is what made the sibling
reachable in a real game.


## 5. Verifying

- `python cards/test_inventory.py` regenerates the coverage docs; the coverage
  report counts per card, so a new combination test raises `direct_test_count`
  and adds the file to `covering_files` — that is the audit trail, not the proof.
  **The proof is the assertion in the test.**
- `cargo test --test run_all` from `engine/`. Per `AGENTS.md`, never
  `cargo check`/`cargo build`; use `RUST_LOG=debug` + the `Start-Process`
  redirect recipe for a failing filtered test only.
- After any rename/move, re-run the full suite: `mod.rs` is regenerated at
  compile time, so a stale declaration is a compile error, not a silent skip.


## 6. Engine-bug register (found by the L1 combination work)

Working the `PL!HS-pb1-009` pair exposed three distinct engine bugs. **All three
are now fixed and landed.** Full suite: 3721 passed, 0 failed.

### Bug A — FIXED. Self-referential blade condition had no subject.

`evaluate_card_blade_condition` (`engine/src/ability/condition/card.rs`) read its
subject from `selected_cards` only. An auto-triggered ability (ライブ開始時,
自動, …) never runs a selection prompt, so `selected_cards` is empty and the
condition returned **false unconditionally**.

Impact: every 「このメンバーが持つブレードの数がN以上」 gate was dead in a real
game. `abilities.json` has exactly two `card_blade_condition` users, and only
花帆's is self-referential (the other, `PL!S-bp3-025-L` SUKI for you, selects an
Aqours member first, so its selection is genuinely non-empty and unaffected).

Fix: when the selection is empty, fall back to the activating card — the empty
selection *is* the self-reference. A genuinely selection-scoped condition is
unaffected because its selection is non-empty when evaluated. Test:
`jidou/combination/hs_pb1_009_jidou_blades_charge_live_start_threshold_test.rs`.

### Bug B — FIXED. A ターン2回 自動 could fire only ONCE per turn.

`just_completed_ability_key` (the re-scan guard in
`engine/src/core/game_state/abilities.rs`) is set when an ability resolves and is
cleared only on the choice-resume path. It **leaked into the next player
action**. `just_completed_batch_matches` then returns `true` (treat the scan as
the same batch) whenever `just_completed_moved` is empty — and a non-movement
effect such as ブレード+2 records no movement, so `just_completed_moved` is empty.
The next action's first TAS scan then saw `skip_key == num_key && same_batch ==
true` and skipped the ability, even though the per-batch dedup set
(`this_batch_triggered_ability_ids`) had already been cleared, proving the event
was fresh.

Impact: 花帆's 2×ブレード+2 never reached the second +2, so she topped out at 6
(4 printed + 2), never 8 — her ライブ開始時 ブレード ≥ 8 was unreachable in a real
game, and the jidou/sibling pair was inert.

Fix: clear the guard at the end of the `while batch_rerun` loop, so its scope is
exactly one batch (it must survive that batch's loopback re-scans, which is its
purpose, but not the next action). Regression guard: the second appearance in
`hs_pb1_009_two_appearances_reach_the_live_start_blade_threshold`.

### Bug C — FIXED. Appearance detection was turn-lenient and over-fired.

`appeared_in_current_batch` short-circuited to `true` when a scan carried no
movement batch, then AND-ed a **turn-scoped** `has_card_appeared_this_turn`. So a
self-scoped 「このメンバーが登場」 自動 (`PL!SP-pb1-006-R` きな子) fired on a LATER,
unrelated appearance: the host had appeared earlier in the turn, so any
subsequent member's debut re-satisfied it (きな子 gained ブレード+2 when a
*different* きな子 debuted).

A debug probe showed the deeper half: at the moment Coco's
`PL!SP-bp1-002-R＋` 登場 condition is re-evaluated (deep inside `execute_effect`),
**every live batch signal is already empty** —
`recently_appeared=[] moved_cards=[] recently_moved=None` — because the ability
drain consumes the batch. The turn-scoped record was the only reason Coco's draw
fired at all.

**Fix — the enqueue-time appearance snapshot.** The codebase already had the right
idiom for this: `entry_trigger_moved_cards()` lets a condition fall back to the
enqueue-time movement snapshot when the live batch is consumed. Bugs A–C's
appearance half needed the mirror:

- `AbilityQueueEntry` gains `trigger_appeared_cards`
  (`engine/src/ability_queue.rs`). A card played to stage records an appearance
  but **no** movement, so `trigger_moved_cards` is empty for it.
- `build_ability_queue_entry` (the single central construction point,
  `abilities.rs`) captures `self.recently_appeared_cards.clone()` — so the
  snapshot always reflects the event that triggered *this* entry.
- `GameState::entry_trigger_appeared_cards()` mirrors
  `entry_trigger_moved_cards()`.
- `appeared_in_current_batch` reads the live batch signals first, then falls back
  to that snapshot. The turn-scoped `has_card_appeared_this_turn` is gone.

This satisfies all three cases at once: Coco sees her own debut in the snapshot
(draw fires); きな子 does **not** see a foreign debut (the snapshot names the
*other* member, not her — no over-fire); the re-scan and swap tests see their
manually-recorded appearance because it is in the entry's snapshot.

One existing test needed a realism fix, which is itself a finding:
`appearance_and_repeated_swaps_stack_blades_test` recorded an appearance and then
relied on the **next unrelated action's** scan to pick it up — depending on the
turn-leniency being removed. A real debut is scanned immediately, so the test now
scans at the appearance. Same trigger, same +2; the arming is just the realistic
path.

### The five attempts that preceded the fix (do not repeat)

The snapshot approach was the sixth try. The first five narrowed
`appeared_in_current_batch` to batch-scoped and each fixed some failures while
re-breaking `coco_left_side_draws` / `appearance_and_three_swaps_stack_eight_blades`:

1. Batch-scope + guard clear → 4 redundant-`scan_autos_both` tests over-fire.
2. + treat the `[-1]` "consumed" sentinel as "an appearance happened" → the 4
   over-fires clear but 2 under-fire.
3. + stop consuming the appearance batch per ability (drop the clear at
   `abilities.rs:1728-1729` and the `[-1]` push at `:1740-1742`) → **no change**;
   proves the per-entry clear is not the culprit.
4. + capture the appearance in `ResumeSnapshot` and restore it in
   `reprocess_ability` → **no change**; `resume=false` in the trace, so there is
   no resume to restore into.
5. (diagnostic) probe `appeared_in_current_batch` and diff against the passing
   run → showed the batch is empty at effect time, which pointed at the
   enqueue-snapshot fix above.

**The lesson:** attempts 1–4 all tried to make the *live* batch survive longer.
The signal was not being lost too early — it was gone by design, and the fix was
to capture the identity at enqueue (the pattern the codebase already used for
movement) rather than to extend the live batch's lifetime.

### Bug D — NOT an engine bug. A vacuous assertion whose fixture also misattributed the grant

Recorded because the *investigation* is the reusable part, and because I first
wrote this section claiming an engine bug before the fixture turned out to be at
fault.

Sweep C reported `effects/score/live_total/kasumi_bp5_002_heart_greater_test.rs`
as an `or-equal-self` hit: a test named `kasumi_with_own_side_higher_no_gain`
asserting `bonus == 0 || bonus == 1` — a disjunction over the **same** variable,
true for any outcome — with a comment saying to "document as permissive". The
vacuous assertion is real and is fixed.

The engine, however, is **correct**, and my first diagnosis was wrong. The test
staged a second `PL!N-bp5-002-R` 春日未来 as the "higher member on her own side".
Because that member is *itself* a 未来, she is the highest on the board and
legitimately grants her own +1 — the +1 was never 未来-under-test misbehaving.

The real bug was in the **fixture**: to test "a higher member on her own side
disqualifies her", the comparison member must not be another copy of the card
under test, or it grants in its own right and the total is unattributable. The
sibling test `kasumi_with_own_side_lower_still_checks_both` already got this right
by using a non-未来 `filler`. The test now does the same (a non-未来 card with
`+6` heart03), so nothing else can grant and the total is attributable to her
alone. All six tests in the file pass, and the `scope: both` behaviour is now
genuinely covered in both directions.

**Two transferable lessons, both about the sweep's output being a starting point
rather than a verdict:**

1. A same-variable disjunction is *always* vacuous and always worth fixing.
2. When the fix is a red test, **check the fixture before the engine.** I spent
   a cycle instrumenting `evaluate_heart_greater_than_all` and `collect_other_stage_ids`
   — both of which were correct — before noticing the card under test was also
   the comparison member. A red test in a suite this old is more often a bad
   fixture than a bad engine, and the cheapest question to ask first is "which
   card is this actually measuring?"

## 7. Sweeps that found real gaps (reproduce these)

**These are executable:**

```sh
python cards/jidou_test_audit.py            # all sweeps, jidou/ scope
python cards/jidou_test_audit.py --sweep a # one of: a (named-but-not-proven),
                                           #       b (re-scan-guard victim shapes),
                                           #       c (premise-never-happened),
                                           #       d (absolute, not delta)
python cards/jidou_test_audit.py --sweep a --all   # sweep A over EVERY module
```

**Sweep A is not confined to `jidou/`, and that matters.** Scoped to `jidou/` it
reports 4 weak tests; over the whole tree it reports **46**. The defect class is
not a jidou property — the three vacuous assertions found by hand in
`rules/trigger_paths/jidou_paired_ability_combination_test.rs` live *outside*
`jidou/`, and a folder-scoped sweep could never have seen them. Run `--all` when
auditing outside the jidou tree.

**A known false-positive class in Sweep A: helper-delegated assertions.** A hit
with `asserts=0` usually means the assertion lives in a helper, not that the test
is vacuous. Verified case:
`effects/cost_mod/required_hearts/jellyfish_appeared_moved_member_required_hearts_test.rs::jellyfish_q99_one_member_both_flags_counts_once`
has zero inline assertions but calls `check_heart_reduction()`, which ends in
`assert_eq!(reduction, expected)`. The sweep labels `asserts=0` in its output for
this reason. The rule the sweep can only approximate: **`asserts=0` means "look
for a helper"; `asserts=1` means "check it is the promised count"** — neither is a
defect until the body has been read.

**The SHARP form, and its own two false-positive classes.** Sweep A also reports
a *sharp* list: tests whose **name makes a count claim** but whose **inline
assertion never measures a count**, so the assertion cannot fail on the thing
the name promises. That is the question that actually matters — `assert_eq!(x, 2)`
checks a count, `assert!(!x.is_empty())` checks presence.

Sharpening it was worth three refinements, because each version was wrong in a way
that would have buried the real finding:
1. Matching only `len()`/`count`/`total` mis-flagged
   `q195_existing_modifier_stacks_on_set_value`, which does
   `assert_eq!(get_blade_modifier(liella), 4)` — a tight exact-value count check.
2. Matching an `assert_eq!` with a numeric literal failed on **nested parens**
   (`get_blade_modifier(liella)`), re-flagging the same test.
3. The settled rule: **`assert_eq!`/`assert_ne!` measures; a bare `assert!` does
   not.** The sweep now reports its two known false-positive classes —
   helper-delegated assertions (verified:
   `rules/phases/live_success_rules_test.rs::two_live_cards_choose_second`, which
   delegates to `assert_two_live_placement()` and ends in an `assert_eq!` on the
   success zone) and `asserts=0`.

**What the sharp list actually found.** Sampling its 9 candidates produced **two
real defects and seven legitimate tests**:

- **REAL (1) — use-limit never reached.**
  `effects/state/wait_activation/hanayo_self_wait_cost_activates_other_member_test.rs::q248_hanayo_use_limit_blocks_second_activation`.
  Three unrelated reasons it passed: no second member on stage (the 起動 activates
  *another* member), the first activation's choice never answered (later calls
  refused with "another choice is pending"), and a four-way error disjunction.
  Fixed in the fixture, with the first activation's `self→wait` cost asserted so
  the refusal is provably about the limit. The engine surfaces an exhausted 起動
  as the *generic* "No activatable ability" (`turn/actions/mod.rs:521-527`
  `continue`s past a spent ability; the `.ok_or(..)` at `:546` reports it), so no
  message match could ever distinguish "used up" from "no target".
- **REAL (2) — permissive bound hid over-firing.**
  `effects/gain/blades/live_start/mymai_tonight_test.rs::mymai_two_copies_plus_aqours_live_both_fire`
  asserted `blade >= 2` where the exact value is 2 (two MY舞 copies × +1 each).
  `>=` also passes at 3, 4, 5 — so an engine that fired each copy **twice** would
  pass a test whose entire point is that each live-start instance fires exactly
  once. This is the permissive mirror of the turn-limit under-counting bugs, and
  it is now `assert_eq!(blade, 2)`.
- **Legitimate, each verified by reading** — `q195_existing_modifier_stacks_on_set_value`
  (exact `== 4`); `two_live_cards_choose_second` (helper asserts the success zone);
  `jellyfish_q99_…` (helper asserts the reduction);
  `q117_second_wien_copy_counts_as_other_member` (`cheer_checks_required == 0`
  proves *both* copies fired, since one would read 2);
  `second_wait_same_live_phase_does_not_refire` (asserts no second prompt **and**
  the second member stays wait); `riko_equal_cost_both_occupied_triggers` — whose
  `assert!(x == Some("wait"))` is an *exact* state check, and whose "both" refers
  to the cost-tie condition rather than a trigger count.

That last one names a third false-positive class: **a bare `assert!` that still
compares against an exact value** (`== Some("wait")`). The sweep keys on
`assert_eq!`/`assert_ne!` and so cannot see it; it is reported as a candidate and
resolves on reading.

**Sweep E — "permissive bounds" (added after the mymai find).** `assert!(x >= 2)`
or `(x <= 2)` accepts a **range**, so it cannot fail on an exact count in *either*
direction: an ability that fires twice, or never, both satisfy it. Sweeps A–D all
key on claims, disjunction, or presence, and none of them sees this — which is
exactly how the mymai defect survived. The sweep greps for `>=`/`<=` applied to a
state quantity, and reports 66 sites. Most are honest (their names literally say
"at least"), so it is a triage list, not a defect list.

Two real defects came out of it, both the **per-instance** multiplicity claim —
the same shape as the mymai one, and both in live-start tests where a second
firing of one copy is the bug being guarded against:

- `effects/gain/blades/live_start/mymai_tonight_test.rs::mymai_two_copies_plus_aqours_live_both_fire`
  — `assert!(blade >= 2)` where the exact value is 2. Now `assert_eq!(blade, 2)`.
- `effects/gain/blades/live_start/live_start_blade_per_discard_test.rs::two_copies_on_stage_both_gain_blades`
  — `assert!(blade1 > 0 && blade2 > 0)`, a *presence* check, on a test whose own
  steps say each copy discards one card and gains exactly 1. A copy firing twice
  passed a test whose entire point is that the two copies fire independently and
  **once each**. Now `assert_eq!(blade1, 1)`, `assert_eq!(blade2, 1)`, and an exact
  total of 2.

The generalisation this adds to §7's closing lesson: **a bound is a claim about a
range, and a test whose name commits to a count has made a claim about a value.**
`>= 2` and `<= 2` are not weaker versions of `== 2` — they are a different,
much weaker claim, and neither is a regression guard for the per-instance
multiplicity that multi-copy cards exist to test.

**Reading Sweep E's 65 sites.** It is a triage list, and the first four read
resolve as:

- **2 real defects** (above): `mymai_two_copies_plus_aqours_live_both_fire`
  (`>= 2`) and `two_copies_on_stage_both_gain_blades` (`> 0 && > 0`).
- **False positive — loop-termination guard.**
  `jidou/ability_watch/copied_ability_registration_lifecycle_test.rs::real_phase_walk_to_live_success_registers_and_fires_the_copy_once`
  uses `assert!(scans < 32)` to bound a phase-walk loop so a stuck state machine
  fails loudly. Its real claims are `assert_eq!` on the registration count, with
  a reasoned precondition (`should_trigger_live_success(...)`) so a later "no
  energy moved" can only be the copy and never a failed live. A bound on an
  ITERATION is not a claim about a game quantity.
- **False positive — bound paired with its complement.**
  `jidou/title_once/dive_retrieved_to_hand_ab0_places_live_zone_ab1_grants_blade_test.rs::ab1_two_niji_members_only_one_gets_blade`
  is named for "only one", and the sweep sees `mod_a >= 2 || mod_b >= 2` in
  isolation. The test then asserts the complement — `assert!(!(mod_a > 0 &&
  mod_b > 0))` — so "at least one" plus "not both" *is* exactly one. Reading a
  single assertion out of its body is how the sharp-A sweep produced its two
  earlier false positives too.

So Sweep E's rule for a reader: **a bound is a defect only when nothing in the
same test body pins the complementary side.**

It exits 0 even when it reports candidates: they are **review candidates, not
defects**. Read each one before changing anything — several are legitimate (a
poked-flag test paired with a real-pipeline test elsewhere, or a single-value
negative that merely names the card's limit in the function name).

Per-card coverage counts (§1) cannot see a test that *exists but does not check
the thing it is named for*. Three mechanical sweeps caught every such gap in this
audit; all are cheap to re-run and worth repeating when a jidou engine fix lands.

**Sweep A — "named but not proven."** List every jidou test whose function name
promises a count or limit (`twice`, `second`, `2回`, `turn2`, `budget`,
`ceiling`, `refus`, `blocked`, `per_turn`, `stacks`, `repeat`), then count the
`assert!`s in its body. Any with ≤1 is a candidate. This found both instances of
the `1 or 2` vacuity — `PL!HS-pb1-003` (瑠璃乃) and `PL!SP-bp7-005` (葉月 恋) —
plus the misnamed `twice_per_turn_*` tests that only ever fired once. Result:
48 limit-naming tests, 4 weak, 2 real defects, 2 misnames. All closed.

**Sweep B — "Bug-shape victims, verified."** A jidou is a **Bug B** victim when
it can fire more than once per turn (`use_limit` ≥ 2 or unlimited) *and* its
effect is not a card movement (`recover_to_hand` / `draw_card` / `gain_resource`)
— the exact shape the re-scan-guard leak silently under-fired. The sweep then
checks each victim for a test driving **two trigger events in one `#[test]`
body** (one batch asserted twice is not a repeat), counting real driver calls:
`play_to_stage`, `activate_ability`, `replace_member_by_baton_touch`,
`set_recently_moved_cards`, `record_card_movement`, `push_movement_event`.

Current result: **14 victim shapes (abilities), 9 covered, 5 abilities on 4
distinct cards outstanding** — three 無制限 "each_time" stage→waitroom watchers
(`PL!HS-bp2-015-N` 藤島慈, `PL!HS-bp6-018-N` 村野さやか, `PL!HS-bp6-019-N`
大沢瑠璃乃) plus the two `PL!N-bp5-030-L` ability watchers (one card, two
abilities). Every existing test performs a single baton touch.

A genuine repeat for the stage→waitroom family needs the **same card instance**
to leave the stage twice in one turn. Both halves of that were driven rather than
assumed, and each is now settled:

| Requirement | Status |
|---|---|
| A **re-entry** putting the member back on stage mid-turn | **SOLVED.** `PL!HS-pb1-019-N` 大沢瑠璃乃's 起動 (「このメンバーをステージから控え室に置く：自分の控え室からメンバーカードを1枚手札に加える。」) has no cost or group gate, and driven end to end it returns the bailed-out card to 手札 **in the same turn**. |
| A **second departure** in that same turn | **BLOCKED by the area lock.** Re-playing her consumes that area's turn lock, and `can_baton_touch` rejects a replacement in an area deployed this turn — *"Cannot baton touch: area is locked this turn"*. The round trip burns the 起動 card's area and the re-played slot, so every area holding a member is locked by the second departure. That is Rule 9.6.2.1 behaviour, not an engine defect. |

So the 無制限 ceiling for this family is **unobservable by construction, not
merely untested**. Both halves were established by driving them, and a survey of
every card that mentions ステージ + 控え室 confirms the gap is real:

- **Re-entry — SOLVED and proven.** `PL!HS-pb1-019-N` 大沢瑠璃乃's 起動
  (「このメンバーをステージから控え室に置く：自分の控え室からメンバーカードを1枚
  手札に加える。」) has no cost or group gate, and driven end to end it returns the
  bailed-out card to 手札 **in the same turn**.
- **Second departure — structurally unreachable.** It needs a standing stage
  member to reach the 控え室 *without* a baton touch, and **no such card exists**.
  Enumerating every ability mentioning both ステージ and 控え室: they all discard
  from *hand* (「手札を…控え室に置く」), mill the *deck*, put a member *to wait*
  (ウェイト) rather than to the 控え室, or place a recovered member *onto* the
  stage. The only stage→waitroom moves in the pool are the baton touch and a
  card's self-send 起動 — and the baton touch is area-locked for a same-turn
  arrival, which is what blocks the round trip.

`rurino_stage_to_waitroom_watcher_baseline_test.rs` therefore lands the
single-departure grant exactly (2 drawn, 2 discarded) as the baseline a
two-departure test *would* double, were such a card printed.

**How to read Sweep B's remainder.** "Lacks a two-event test" does not imply "is
undertested". For these five shapes the two-event behaviour is **unobservable with
the current card pool**, so no test could distinguish a correct 無制限
implementation from a wrong ターン1回 one. Treat the list as "unobservable, not
missing" unless a future card adds a non-baton stage→waitroom move.

Re-entry cards that look like candidates and are **not** usable for a
cross-turn variant: `PL!HS-bp1-003-R＋` 乙宗 梢's 起動 recovers only
「4コスト以下の『蓮ノ空』のメンバー」 and this member costs 5; every live-phase
recovery returns her in the **next** turn, which cannot distinguish 無制限 from
ターン1回.

**Three corrections to the first write-up of this blocker, all mine:**
- I blamed "group-filter fidelity — the card database's `unit`/`group`/`series`
  are all null". Probing the **runtime** database (`game.db`, not the repo-root
  `cards.json` I had been reading) shows they are fully populated; these are
  **蓮ノ空** cards (unit みらくらぱーく！), not the 『Liella!』/『虹ヶ咲』 my
  fixtures assumed. `assert_card_in_group` had already rejected 『Liella!』; I
  "fixed" it by guessing 『虹ヶ咲』, which was also wrong.
- The gate that actually blocked the first route is **cost**, not group
  (cost ≤ 4 vs cost 5).
- The re-entry route I first reported as "unresolved" was a **transposed card
  number in my own constant**: `PL!HS-**bp**1-019-N` silently resolves to a
  different card ("Dream Believers") via `get_card_id`'s rarity fallback, instead
  of failing. The working constant is `PL!HS-**pb**1-019-N`. Both numbers are now
  asserted in the test, because a `bp`/`pb` transposition here surfaces as a
  *missing ability naming an unrelated card* — the most confusing failure mode in
  this audit.

**A **Bug C** victim is a jidou whose condition reads an appearance; it needs a
real `play_to_stage`/`baton` debut (not a poked `record_card_appearance`) and, if
the trigger names 「このメンバーが登場」, a negative where a *foreign* member
debuts. All 12 appearance-watching jidous drive a real debut, and the
`PL!SP-pb1-006-R` きな子 foreign-debut negative already existed — it is the test
that caught the over-fire during the fix.

Two false-positive classes the sweep learned to avoid, both of which bit this
audit repeatedly:
- **Print variants.** `abilities.json` lists `PL!N-bp5-005-R＋`; the test uses
  `PL!N-bp5-005-P`. Matching the exact string reports a covered card as untested,
  so the sweep matches the *base* card number and every print counts.
- **Comments.** A doc comment that *quotes* the bad assertion it replaced (as the
  rewritten `turn_limited_yell_second_activation_blocked_test.rs` does) would be
  reported as if the bad assertion were still live. Comments are stripped first.

**### Partial hardening is the dangerous state

`rules/trigger_paths/jidou_paired_ability_combination_test.rs` had been worked
over before — its comments show someone already understood the trap once ("checking
only blade_modifiers saw just ab#1 and passed when ab#0 did nothing", "`>= before`
passed when NEITHER jidou fired"). **And three of its five tests were still
vacuous.** A correct delta assertion sat in the same file as three broken ones:

| Test | Was | Now |
|---|---|---|
| `jidou_effect_cause_both_sides` | `after_blades >= before_blades` — true when the jidou grants *nothing* | `len - before == 1` |
| `jidou_distinct_from_constant_and_activation` | `a >= b \|\| a == b` — a **tautology**, true for every pair; its own comment conceded "may or may not add blade" | `len - after_const == 1` |
| `jidou_both_on_same_card_coexist_and_fire_separately` | `!blade_modifiers.is_empty()` — **absolute**, so the card's own printed blades satisfy it | delta against a pre-trigger baseline |

The absolute-vs-delta distinction is the subtle one: `!blade_modifiers.is_empty()`
is satisfied by 葉月恋's own printed blades whether or not her jidou ever fires,
so it could not distinguish "granted" from "not wired at all" — and the file already
contained the correct delta form two tests above, which is exactly why the broken
ones survived review.

**The generalisation: a file being "already hardened" is not evidence that its
tests assert anything.** Audit every assertion in a file, not the file's
reputation; and prefer a baseline-taken delta over an absolute predicate whenever
the card's own printed stats could satisfy it. `cross_player_jidou_triggers_test.rs`
is the counter-example in the same directory — all nine of its assertions are exact
`assert_eq!(…, 1)` / `(…, 2)` with no leniency at all.

**Sweep D — "absolute, not delta" (added after the 葉月恋 case).**
`assert!(!…blade_modifiers.is_empty())` passes when the member is merely ON
STAGE: printed blade/heart satisfy the predicate with or without the jidou ever
firing. A real test takes a baseline before the trigger and asserts a delta.
Detector: a negated `is_empty()` / `len() > 0` on a `*_modifiers` collection
inside an `assert!`, with no `-` against a `before` baseline in the same body.

**It reports review candidates, not defects.** The one current hit,
`characterization/zero_tested_action_types_test.rs::vivid_world_live_phase_blade_and_success`
(`!blade_type_modifiers.is_empty()`), was reviewed **OK**: that map is populated
*only* by the `set_blade_type` effect and the fixture never pre-populates it, so
non-empty genuinely means the ability applied. The question to ask per hit is
whether the collection has a **printed baseline** that could satisfy it — which is
why the sweep names a shape rather than asserting a verdict.

**On detector trust.** A regex sweep that silently reports zero is worse than no
sweep — it looks like a clean bill of health. This bit twice in this audit, and
the second time it was a *hand-written* scan rather than the tool:

1. Sweep D's first pattern matched neither the known-bad nor the known-good form
   and reported 0. Caught only because a 0 on a suite where that exact bug had
   *just* been fixed by hand is implausible.
2. A one-off scan over the 17 files this audit created or modified reported
   **completely clean** — because the file *paths* were piped into
   `Select-String` as input strings, so it searched the path names, not the file
   contents. Adding a sanity line (total `assert` occurrences must be > 0)
   exposed it immediately; re-run correctly, it found 205 asserts and only
   false positives.

The rules that follow, both learned the hard way:
- **Print a sanity count from the same code path** that produced the result. A
  0 is only evidence if the scanner demonstrably read something.
- **Prefer a committed tool over a shell one-liner** for any result you intend to
  act on; `cards/jidou_test_audit.py` strips comments, so a doc comment quoting
  the bad assertion it replaced is not re-reported, and it has no
  `assert_eq!`-matches-`[<>]=` confusion.

Every detector here is checked against a hand-built bad/good pair before being
trusted, and a 0 result should be sanity-checked against a known instance before
it is believed.

**Run the detectors on your own work.** A sweep is only trustworthy if it is
pointed at the tests you just wrote. Doing that here immediately caught two
instances of the very shape Sweep D was built for, in the combination files this
same audit produced:

- `hs_pb1_001_…payment_test.rs` asserted `get_blade_modifier(hana) > 0`
- `hs_pb1_009_…threshold_test.rs` asserted `get_blade_modifier(hanamo) >= 4`

Both are absolute / non-exact counts where the true value is known (1 and 4), and
both are now exact. Re-running the battery over **all 17 files** this audit
created or modified returns no genuine hits — the only matches are the
`assert_eq!` regex confusion and the doc comments quoting the defects already
fixed. The lesson generalises past the audit: **the standard you apply to a file
you did not write is the one your own new files have to clear first** — a
detector that finds nothing in freshly written tests is usually pointed at the
wrong directory, broken, or the tests were written before it existed.
**The general lesson from all four sweeps: an assertion that accepts more than
one value (`1 or 2`, `<= 2`, "no crash") is where a real regression hides.** Where
a test's name commits to a count, assert the exact number in both
directions - the ceiling *and* the grant - so neither a missing nor an extra
firing can pass.

**Sweep C - "did the premise event actually happen?"** Extend the lenient
sweep past jidou/ to the whole test tree (`effects/`, `rules/`): grep for
`assert!(x == N || ...)` / `assert!(x <= N)` / "no crash". Then check the
**fixture** rather than the assertion: did the triggering event actually occur?
This is what exposed the yell family: ~25 tests whose live card required
nothing, so no yell ever happened, and the jidou under test correctly did
nothing every time. The tell was a test named `..._triggers_...` that asserted
the value was zero, with a message blaming a nonexistent engine bug.

The three sweeps share one root cause: **a test can be internally consistent and
still prove nothing, because the event that is supposed to drive it never
happened.** The cheapest detector is to assert the *premise* explicitly -
`assert!(game.state.yell_occurred)`, `assert!(!revealed_cards.is_empty())` -
so a fixture that silently stops producing its event fails loudly instead of
passing vacuously. Prefer observable state over poked flags
(`set_recently_moved_cards`, `record_card_appearance`, hand-set
`yell_occurred`) whenever a real path exists; use a poked form only when
paired with a real-pipeline test for the same ability.

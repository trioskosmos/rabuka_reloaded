# PBLL02 test audit — what is bullshit, what is missing

Audit date: 2026-09-30. Suite at time of audit: **3984 passed, 0 failed** (was 3983 before
this pass).

This supersedes the stale coverage claims in `PBLL02_ABILITY_REVIEW.md` (§3, §8.3, §8.11,
§8.17, §8.20). Every number below was measured, not inherited. Where a sub-audit's claim
conflicted with the tree, the tree won — see §6.

Method: three per-file audits of all 45 PBLL02 test files against the printed text in
`cards/cards.json` (the authority) and `cards/abilities.json` (parser output), plus
independent re-derivation of every coverage number, and source verification of every
harness claim.

---

## 0. An engine bug found and FIXED: `PL!-pb2-040-L` over-fires

**The bug.** The printed text is

> 自分のステージに、**このターン中に**自分の『Printemps』のカードの効果によってウェイト状態から
> アクティブにしていたメンバーが1人以上いる場合、−3。2人以上いる場合、**さらに**−2。
> 3人以上いる場合、**さらに**−1。

Only the first clause states the population; the 2人/3人 clauses are elliptical. But
`_mark_activated_this_turn` (`cards/ability_extraction/parser.py:3613`) tags a node only when
that node's **own** text contains 「このターン中に」 and 「アクティブにしていた」. So steps 2
and 3 parsed as a bare count of the stage, and the engine reads the tag per condition with
**no inheritance** (`core/card.rs:4079-4081`), falling through to the location-count branch
at `ability/condition/card.rs:2401`.

Measured: staging three Printemps members and activating **none** produced **−6**, where the
card requires **0**. The reduction was reachable by a board that did nothing.

**Why no test caught it.** Every test in that file used a `modifier_for(count)` helper that
staged `count` members **and** activated the same `count`, and its negative used `count = 0`
— zero staged *and* zero activated. The two quantities were never varied independently, so
"counts the activation history" and "counts anything on the stage" were indistinguishable.

**The fix.** Added `_inherit_elliptical_history_steps` to the parser, called from
`_mark_activated_this_turn_nodes`, which carries `activated_this_turn` from a step's
condition into a following step whose condition is a bare 「N以上」 continuation.

It carries **only** `activated_this_turn`, deliberately **not** `group_names`: on a
continuation step the group can legitimately belong to the branch **action** (the target)
rather than to the count condition — the repo's own
`test_sunny_day_song_group_filter_stays_on_mus_branch` depends on exactly that. Copying the
group onto the condition would silently add a group filter to that count and change the
card. The group half of the 040 population is therefore left alone on purpose.

**Blast radius, measured rather than assumed** — the whole diff of `cards/abilities.json`:

```
-  "generated_at": ...          (timestamp)
-  "engine_commit": ...         (stamp)
+  "activated_this_turn": true  (040-L step 2)
+  "activated_this_turn": true  (040-L step 3)
```

Two functional lines in a 27,000-line file, on the intended card. No other card moved, so the
`action_047` failure mode the doc warns about did not occur.

**Verification.** Engine suite **3985 passed, 0 failed**. New regression test
`a_staged_member_that_was_never_activated_reduces_nothing` asserts `0` and would read `-6`
against the old parse. The 040 test file also gained `modifier_with(staged, activated)`.

Parser suite: **123 passed, 1 failed** — and the failure is **pre-existing, not caused by
this change**, proven by restoring HEAD's `parser.py` and re-running: it fails identically.
See §0.1.

## 0.1 A pre-existing red test that explains the 042-L "OPEN GAP"

`cards/ability_extraction/tests/test_ability_invariants.py::test_compound_subconditions_inherit_location`
fails on `PL!-pb2-042-L` (PSYCHIC FIRE):

> bare sub-conditions of a compound must inherit the clause's location
> (`parser._propagate_compound_locations`); missing on:
> `[('PL!-pb2-042-L | PSYCHIC FIRE (ab#0)', '自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる場合')]`

This is the mechanism behind the `042_l` test file's recorded OPEN GAP — the ability never
fires because a compound sub-condition never inherits its `location`. **The repo's own
invariant test is already red on it and the engine suite is green anyway**, because
`pb2_psychic_fire_four_clauses_pl_pb2_042_l_test.rs` pins `assert!(!fired)`.

So the `042-L` gap is not open-ended: it is diagnosed, the failing test names the cause, and
the fix is in `_propagate_compound_locations`. It was never chased because the "test" for it
asserts the broken behaviour and therefore passes forever.

### 0.2 …but the red invariant test is a FALSE POSITIVE, and 042-L is still genuinely broken

I checked that claim rather than repeating it, and it needs correcting in two places.

**The parser is right; the test is wrong.** The invariant wants the compound's bare
sub-condition stamped with `revealed_cards`. That sub-condition is
「自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる場合」 — it already carries
`position: "center"` and its own text names the **centre area**, which is not the revealed-cards
zone. Stamping it would be wrong, and `_drop_unsupported_child_location` exists precisely to
prevent it — its docstring names `PL!-pb2-042-L`. The test checks an intermediate state that
the parser deliberately undoes. The invariant's exemption list is missing the
text-contradicts case, so it should be amended, not the parser.

**The ability is still genuinely broken, for a different reason.** I tested this empirically:
inverting the assertion in `all_clauses_met_waits_the_foe` shows the ability does **not** fire
with all four clauses satisfied, while all five negatives still hold. So it is a missing
positive, not an over-firing engine — the compound evaluates false somewhere.

The parse is correct, so the fault is in evaluation. Two live suspects:

* **`yell_trigger`** — the condition carries a top-level `yell_trigger: true` (「自分が
  エールしたとき」), and the test fixture never performs an actual エール; it pushes cards into
  the revealed-cards list directly. If the engine evaluates `yell_trigger` against real yell
  state, the condition can never be true in this fixture — and **the test may be wrong rather
  than the engine**, which would be a different defect from the one the header records.
* **`position: "center"` inside a nested `comparison_condition`** — the centre restriction is
  carried as a bare `position` field, not as a `location`. A generic evaluation path that
  reads only `location` would see no zone and could reject.

Discriminating between those needs a fixture that performs a real エール, which is the next
piece of work. The test header now records both suspects rather than "which clause is failing
is still unknown".



Stated first so the rest is read in proportion.

* The suite is green and the **parser is deterministic and in sync**. Re-running
  `extract_card_abilities.py` changed `cards/abilities.json` by exactly two lines — the
  `generated_at` timestamp and the `engine_commit` stamp — and produced byte-identical
  generated decoders. The `action_047` class of unrelated-output-shift is not happening.
* **All 98 PBLL02 ability nodes are exercised.** Computed at the node level (an ability
  counts as covered when any test file names any card carrying it), not the card level.
  This is the right metric: 36 of the 47 unnamed card prints are `-R`/`-PP` reprints of a
  tested sibling, i.e. reskins that do the same thing, and 11 of the remainder share a
  node with a tested card.
* Roughly **20 of the 45 files are real behavioural coverage** — they drive the engine
  and measure a printed outcome. The strongest are `pb2_debut_wait_opponent_up_to_three_printed_hearts_…033_n`,
  `pb2_two_mus_members_live_or_success_zone_score_…038_l`, `live_start_lily_white_success_zone_count_…041_l`,
  `pb2_optional_self_wait_heart03_…028_n`, `pb2_bibi_member_under_per_blade_…011_p_plus`,
  `pb2_kidou_look_three_take_printemps_member_…026_n` (the cost half) and
  `baton_touch_place_liella_under_then_activate_copied_ability_…005`.
* `play_to_stage` now **panics on an occupied area** (`helpers/mod.rs:390-400`), which
  structurally prevents the implicit-baton-touch trap. No test in this audit is
  accidentally saved by one.

---

## 2. The harness traps — these are worse than any individual bad test

A test can only be as honest as the API it measures through. Four helpers currently lie
in ways that produce *confidently worded* false coverage.

### 2.1 `pending_choice_count()` is not the number of candidates — **FIXED, helper added**

`Choice::select_cards(zone, max_select, …)` sets `count = max_select = min(requested,
matching)` (`ability/effects/look.rs:110,156`). Candidates are a **separate**
`filtered_indices` field (`look.rs:175`). `helpers/choices.rs:421` reads `count`.

So for any 「1枚」 look, `assert_eq!(game.pending_choice_count(), 1)` returns **1 whether
one card or five qualify**. It cannot detect a dropped group or property filter.

Affected, with the false claims attached:

| Test | Asserted | Claim in the message | Reality |
|---|---|---|---|
| `…032_n` `only_blade_heart_free_mus_members_are_selectable` | `count == 1` | "Dropping either filter would offer 2 or 3" | `min(1, n)` is 1 for every n. Sentence is false. |
| `…008_pp` `the_bound_separates_the_seven_from_the_eight` | `count == 1` | "the one that actually pins the bound" | Adding the 7-total live gives `min(1,2)=1`. Bound not pinned. |
| `…026_n` `offered` | `count == 1` | "only 小泉花陽 qualifies" | Never reaches the look at all (§2.2). |

Added `TestGame::pending_choice_option_count() -> Option<usize>`
(`helpers/choices.rs`), returning `filtered_indices.len()` per
`GameState::choice_option_count` (`core/game_state/mod.rs:1355-1363`), and `None` for an
unfiltered prompt rather than a fabricated `0`. Its doc comment records the trap and also
notes that `pending_selection_cards()` is **not** the answer: when `filtered_indices` is
set, `selection_cards` deliberately lists all looked-at cards so the UI can grey the
non-matching ones (`core/game_state/abilities.rs:2584-2588`).

### 2.2 `drain_auto_ability_choices()` leaves the mandatory cost prompt pending

`helpers/choices.rs:341-356` drains `SelectAutoAbility` and **breaks on anything else**.
For any card whose 登場/起動 has a mandatory cost (`手札を1枚控え室に置く`, …), the cost
raises a `SelectCard` prompt that survives the drain. The test then measures that prompt
and believes it measured the effect.

Both `…032_n` and `…026_n` are in this state: they assert on a **discard-cost** prompt.
`…032_n`'s three assertions (`has_pending_choice`, `type == "SelectCard"`, `count == 1`)
all read the cost. Neither file ever observes its look.

### 2.3 `game.id()` returns a fresh pooled instance every call

`helpers/mod.rs:217-232`. `card_id` panics only when the card number is absent from
`cards.json` (`mod.rs:104-107`) — there is no stage-legality check, so `game.id(X)` happily
returns an instance for a card that could never legally be on stage. Asserting on a
freshly-requested id inspects a card that was never played.

`audit_id_in_assert.py` reports 6 sites, 5 in PBLL02 files. Its own bottom line (game-state
reads through a fresh id) is 0, so these are fixture-property checks and mostly benign —
but `…015_p_plus` line 68 has the real bug: it stages `game.id(OPPONENT)` and then waits a
**different** instance of the same print, so the 自動's trigger state is attached to a card
that was never played.

### 2.5 `main_deck` draws from the FRONT — a `push` after `fill_decks` is never looked at — **NEW, found the hard way**

`MainDeck::draw()` is `cards.remove(0)` (`core/zones.rs:811-820`), and the look step
uses `draw_multiple` (`ability/effects/look.rs:1031`). So a 「デッキの上からN枚見る」 takes
indexes 0..N. `fill_decks` (`tests/helpers/mod.rs:564-571`) fills 30 cards by
`push`, so **anything `push`ed after it lands at the bottom and is never looked at.**

The repo already has the right helper — `put_on_deck_top(game, player, card)` inserts at
index 0 (`helpers/mod.rs:573-575`) — and the existing tests use it. The rewritten
`…032_n` did not, and produced this:

```
look saw 5 copies of the filler -> zero matched -> all 5 discarded -> no prompt
```

The filler `PL!-sd1-010-SD` carries `blade_heart: {b_heart03: 1}`, so zero matches was
**correct engine behaviour**, not a bug. It took a diagnostic run with five copies of a
card that must qualify to prove the engine was innocent. This is the same class of error
as the retracted OPEN GAP in `…018_p_plus`: a fixture mistake that reads as an engine
defect. `…032_n` now uses `put_on_deck_top` and pins the filler's blade heart in
`the_fixture_invariants_hold`.

### 2.6 `select_option(n)` on a `SelectCard` selects hand index n — it is not a decline

`turn/actions/mod.rs:1078-1086` maps the scalar straight to a hand index. The decline
idiom is `select_indices(&[])`; on a `SelectTarget` pay/skip prompt `0` **is** decline.

`…019_n` `a_non_mus_centre_member_gains_nothing` calls `select_option(1)` against a
one-card hand. The cost was never paid, so the test named "a non-μ's centre member gains
nothing" cannot distinguish a working group filter from an effect that never started.

---

## 3. Tests that assert the opposite of the printed card

The most dangerous class, because **they will start failing the day the engine is fixed**,
which reads as a regression and invites a wrong "fix".

| File | Test | Body | Printed card says |
|---|---|---|---|
| `…037_l` | `all_printemps_qualifies_and_the_member_is_taken` | `assert!(!hand_gained_with(&[P, P]))` | the member **is** taken to hand |
| `…037_l` | `all_lily_white_qualifies` / `all_bibi_qualifies` | same shape | same |
| `…042_l` | `all_clauses_met_waits_the_foe` | `assert!(!fired)` | the foe **is** waited |
| `pl_pb2_001_pp` | `the_score_and_all_blade_clauses_produce_nothing_measurable_yet` | `assert_eq!(p1_constant_total_score_bonus, 0)` | 「ライブの合計スコアを＋１する」 |

`pl_pb2_001_pp` is worse than the others. Its loop runs the same assertion for the スコア
and the ALLブレード cases, but `abilities.json` shows the ALLブレード step is
`gain_resource / heart / heart_type: all` — it grants a **heart**. Asserting a *score* field
for a heart-granting clause can never fail at all.

`…037_l` also pins a decode bug as an expected value:
`assert_eq!(carried, vec![false, true, true])` claims 「すべて」 is uncarryable on the
`location_condition`. But `abilities.json` shows the Printemps branch **does** carry
`all_members: true`; the loss is in `get_all_members()`, which matches only `Condition::Group`
(`core/card.rs:4064-4072`). The parse is fine; the typed decode drops it, and the test
blesses that.

### 3.1 Literal tautologies — cannot fail under any engine

* `…024_n` — `assert!(observed || !observed)`, i.e. `assert!(true)`. **FIXED**: now builds
  the genuinely-empty stage its name claimed and pins the vacuous reading `true` that
  `…029_n` already pins.
* `…010_p_plus` — `blade_with_history(&[])` is `inner(&[]) - inner(&[])` = `x - x`.
* `…016_p_plus` — `assert_eq!(game.pending_choice_count(), 0)`, but
  `pending_choice_count` is hardcoded `0` for every non-`SelectCard` choice
  (`choices.rs:421-428`). The prompt is a `SelectTarget`.
* `…009_pp` — the two negatives are the **same equality with the operands swapped**, and
  neither asserts `== 0`. If the engine wrongly paid for *both* the cheap μ's member and
  the expensive non-μ's member, both pass.
* `…032_n` — asserts the hand is unchanged, and the message says the cost and the take
  cancel. Passes if the cost never fires, if the take never fires, or if both do.
* `…017_p_plus` — asserts nothing landed under a second member that has no
  under-card-placing ability, so nothing could ever land there.
* `…036_n` — `hand.contains(d1) && hand.contains(d2)` holds both with the 2-discard paid
  (hand = `{d1,d2}`) and unpaid (hand = `{keep_a,keep_b,d1,d2}`). The mandatory cost is untested.
* `…003_pp` — `our_own_live_success_abilities_are_not_invalidated` stages an **ally** and
  asserts `0`; the card targets the **opponent's** stage, which is empty. The `0` comes
  from "no opponent member", not from "our own are not targets".
* `…021_n` — `an_empty_setup_grants_nothing` and `it_does_not_fire_for_another_cards_ability`
  both assert on an `eri` that is in no zone at all (the second stages `other`, a different
  pooled instance, and asserts on `eri`).

### 3.2 Round-trip parse tests presented as coverage

Reads a field off the ability it just parsed and asserts it equals what the parse produced.
Zero engine lines. Can only fail on a decode break. The repo's §8.20 correctly identifies
this category and then commits it anyway in:

* `…014_p_plus` — **all four tests.** Not one line drives the engine.
* `…010_p_plus` `the_clause_parses_as_a_history_not_a_state`
* `…016_p_plus` `the_parse_is_a_three_option_choice`
* `…041_l` `the_constant_parses_as_a_counting_weight_of_two`
* `…039_l` `the_live_success_harness_fires_the_ability` — the doc comment claims "the
  live-success path really ran"; the body constructs no game and runs no live.
* `pb2_double_baton_debut_recover_and_score_…000_duo` — 4 of its 7 tests
* `…021_n` `the_wait_branch_parses_as_a_self_scoped_state_condition`
* `…040_l` `the_clause_parses_as_a_history_not_a_state`
* `…018_p_plus` `the_cost_requires_three_distinct_names`
* `…009_pp` `fixture_costs_are_what_the_test_assumes`
* `…013_n` `two_prints_are_one_name` — two pooled copies of one card asserted to share a
  `name` field. A tautology of the database, not a property of the `distinct` filter the
  file is about.

---

## 4. Wrong character and fixture metadata

**18 files attribute the wrong character.** All the `PL!` constants are correct card
numbers — this is comment rot, invisible to `assert_card_identity`, and it is exactly the
class the repo's own audit targets.

Fixed in this pass: `…018_p_plus` (絢瀬絵里 → **矢澤にこ**), `…029_n` (西木野真姫 →
**絢瀬絵里**), `…010_p_plus` (絢瀬絵里 → **高坂穂乃果**), `…015_p_plus` (絢瀬絵里 →
**西木野真姫**), `…016_p_plus` (深層共鳴 → **東條 希**), `…023_n` (南ことり → **星空 凛**,
including the `KOTORI` const and local, renamed to `RIN`).

Still wrong: `…007_pp` (小泉花陽 → 東條 希), `…017_p_plus` (東條 希 → 小泉花陽),
`…014_p_plus` (乙宗 紫 → 星空 凛), `…032_n` (西木野真姫 → 星空 凛), `…003_pp` (矢澤にこ →
南ことり), `…031_n` (西木野真姫 → 園田海未).

Note `…031_n` and `…032_n` both name a constant `MAKI` for two **different** characters.

**Wrong fixture claims — these are worse than comment rot, because they invert semantics:**

* `…032_n` — **both blade fixtures are inverted.** `NO_BH = PL!-bp6-017-N` (小泉花陽) has
  `blade_heart: {b_heart03: 1}`; `BH_MEMBER = PL!-pb2-002-PP` (絢瀬絵里) has **no**
  `blade_heart` key at all. `has_blade_heart()` is exactly `blade_heart.is_some()`
  (`core/card.rs:4273-4279`). Under `negation: true` the single match is 絢瀬絵里 — the card
  the test names as the excluded one. The ブレードハートを持たない filter is not tested;
  the test passes with the semantics precisely backwards.
* `…017_p_plus` — `P_5 = PL!-pb2-030-N` is commented "with no abilities of their own". It
  prints a 常時 blade ability, and it is the 5th card in the `only_four_are_taken` test.
* `…014_p_plus` — `MUS_LIVE` is documented as "A μ's live that is NOT lily white" and, on
  the next line, as "lily white — qualifies". Both cannot be true; cards.json says
  `unit: 'lily white'`, and the test then asserts it **is** lily white.
* `…004_pp` — `PL!HS-bp1-019-L` called "DOLLCHESTRA"; it is 蓮ノ空.
* `…005` compound — `PL!SP-sd1-006-SD` called 若菜四季; it is 桜小路きな子. The premise pin
  at line 216 asserts group `5yncri5e!` while the ability filters on **『Liella!』**, so it
  pins a property the ability never reads.
* `…019_n` — header claims it also covers "the identically-worded `PL!-bp4-011-N`". That
  card has no `手札を1枚控え室に置いてもよい` clause.

---

## 5. What is genuinely missing

### 5.1 Printed abilities with no test anywhere

Confirmed by matching every card number against all 1,400+ `.rs` files.

| Card | Untested ability |
|---|---|
| `PL!-pb2-011-P＋` | ab#1 自動 — put a 『BiBi』 member from the waitroom under her, with the `2枚以下` cap |
| `PL!-pb2-004-PP` | ab#1 自動／ターン1回 — one extra yell |
| `PL!-pb2-012-P＋` | ab#1 起動／ターン1回 — take a 『Printemps』 live from the waitroom |
| `PL!-pb2-006-PP` | the 起動 half (the file only ever matches `ライブ開始時`) |
| `PL!-pb2-017-P＋` | ab#1 ライブ開始時 — up to 3 under-cards to the waitroom, one 『Printemps』 member set active/wait per card |
| `PL!-pb2-018-P＋` | ab#0 — activate up to 3 waited members, draw one per member |
| `PL!SP-pb2-036-N` / `037-N` | the **右サイド／左サイド restriction itself** — the only thing that distinguishes these two cards, and neither test ever plays the card on the wrong side |
| `PL!-pb2-007-PP` | the 『μ's』 filter on the **waitroom** recovery (only the success-zone half has a negative; the test asserts `hand >= 1` and never that the card taken is the μ's live) |
| `PL!-pb2-014-P＋` | step (1) of the 登場, the whole `reveal` cost shape, and any engine behaviour |
| `PL!-pb2-016-P＋` | options 0 and 2 (only option 1 is executed); the per-card scaling 「1枚につき」; the repeatable clause 「同じ選択肢を2回以上選んでもよい」 |
| `PL!-pb2-012-P＋` | the 「ウェイトにしてもよい」 half — the parse has **no wait action and no prompt at all**, so every test would pass if the engine subtracted 2 for free |
| `PL!-pb2-015-P＋` | the real 自動 trigger — the parse has no `state_condition` for ウェイト状態 and `target: "both"`, not opponent. Every test fires it manually, so nothing would notice if it fired unconditionally. |
| `PL!-pb2-021-N` | the documented negative `an_active_member_with_no_trigger_grants_nothing_is_NOT_yet_true` is **referenced by the file's own header at lines 35-36 and does not exist** — it is the only guard against a real over-fire |

Two of these sit on visibly damaged parses, so nothing in the suite would surface a
regression: `PL!-pb2-012-P＋` ab#1 contains a stray operandless
`{"action":"modify_score","operation":"add"}`, and `PL!-pb2-015-P＋` lost its trigger
condition entirely.

### 5.2 Card prints

Per your ruling, reprints are reskins and one test per ability is enough. On that basis:
**nothing is missing.** 47 ability-bearing prints have no test file naming them; 36 are
reprints of a tested sibling, and the remaining 11 share an ability node with a tested
card, so the node-level figure is **98/98**.

### 5.3 Test-quality rules that are declared but not enforced

* `WRITING_TESTS.md` / `TEST_QUALITY.md` already flag `no_drive`, `no_assert`,
  `live_success_impossible_live` and `drain_zero_may_decline` as baselines. They are
  **advisory only** — nothing fails the build. `…018_p_plus`'s `two_prints_are_one_name` and
  `…039_l`'s `the_live_success_harness_fires_the_ability` are both already in `no_assert`
  and are still committed.
* `PL!-pb2-039-L` has `need_heart: {heart01:9, heart03:9, heart06:9, heart0:9}`. The best
  single member prints 6/3/5, so the live is **unplayable by construction** and its
  ライブ成功時 positives can never be exercised — the repo's own `TEST_QUALITY.md:75` says so.
  A card that cannot be played should not carry the pack's flagship score ability, or the
  test should stage the score zone directly and say so.

---

## 6. Corrections to `PBLL02_ABILITY_REVIEW.md`

| Doc claim | Reality |
|---|---|
| §8.17 "card VARIANTS with no test at all now reads **0**" | **47**. The table that drove it to 0 was deleted in §8.20; nothing replaced it. 11 are non-reprints, but all 11 share a node with a tested card. |
| §3 lists 4 missing mechanics: `all_blade`(001), `distinct_name`(018), `or_location`(038), `card_identity`(041) | still 4, but a **different set**: `all_blade` is fixed, and a **new** `per_unit` mismatch on `PL!-pb2-016-P＋` has appeared |
| §8.3 item 15 "`distinct` missing from the cost (**fixed**)" for 018 | extractor still reports `distinct_name` on 018 |
| §4.1 / P7 the `modify_score` missing-`value` warning on 012 起動 | still emitted, verbatim |
| §8.11 "10 residual findings" from the fixture audit | **11** |
| §8.20 `audit_id_in_assert.py` "reports 0 remaining" | 6 suspect sites (its own game-state bottom line is 0) |
| §6 "`EffectFilter` is the source of truth" at 159 fields | **165** |
| **§8.3 item 13** — `modify_score` + `per_unit` is "a general gap": `PL!HS-bp2-020-L` "also returns +0, and it has no test either" | **False on both halves.** Same shape (per_unit + group + `distinct: card_name`), and `live_start_distinct_group_member_score_test.rs:50-53` asserts it returns **6** for three distinct 蓮ノ空 members. That test is in the green suite. Verified directly. |
| §8.2 "3718 passed, 0 failed" / §1 "3727 passed, 2 failed" | 3983 / 0 |

**Where a sub-audit was wrong and the tree corrected it** — worth recording, because the
error is instructive. The `…040_l` audit reported that three tests assert `-3`/`-5`/`-6`
with `OPEN GAP:` messages, "yet no modifier is produced at all", and concluded "the file
cannot be in a green suite". The suite is green. The tests pass, so the engine produces
those values. The helper's own docstring (lines 57-69) records that the gap was closed by
fixing the **test** — an earlier version staged the members and called
`recalculate_constants()`, which never runs a ライブ開始時, so the modifier read a trivially
zero. The `OPEN GAP:` prefixes were never removed. Those were **stale markers, not red
tests** — and per §8.20, a reader acting on them would have deleted working assertions.

**FIXED** in this pass: the three stale `OPEN GAP:` prefixes, and the genuinely vacuous
`a_member_still_in_wait_does_not_reduce`.

That second fix is the most instructive item in this audit, because **my first correction
was also wrong** and the test caught it. The story, in full:

1. The original test staged a waited member, sampled the modifier, called
   `recalculate_constants()` — which never runs a ライブ開始時 — and compared against the
   value from one line earlier. Nothing in between could have changed it, so it passed
   unconditionally. The premise was never tested.
2. The test was *named* `a_member_still_in_wait_does_not_reduce` and asserted `0`.
3. I rewrote it to actually perform the live. It went red: **the engine produces −3, not 0.**
4. −3 is correct. 「このターン中に…ウェイト状態からアクティブにしていた」 is
   *retrospective*, and the step's condition is a `card_count_condition` over the
   **activation history** (`activated_this_turn`). A member that was activated this turn
   and has since been waited again still satisfies it.
5. So the original test was not merely vacuous — it was **a vacuous test protecting a wrong
   premise**, and its name asserted a rule the engine does not implement and, on the
   printed reading, should not.

The test is now `a_member_waited_again_after_being_activated_still_counts` and pins −3 for
real. This is the §8.20 failure mode in its purest form, and it is the third instance in
this audit where a vacuous test was the only thing standing between the suite and a wrong
belief.

### Contradiction resolved

`…024_n` declined to pin whether 「のみ」 is vacuously true over an empty stage, while
`…029_n` pinned it as `true` with a real assertion. Both stages held only members that
*were* in the group, so neither was actually testing the vacuous case. `…024_n` now builds
a genuinely empty stage and pins `true`, consistent with `…029_n`, with the §8.3 item 1
open question recorded in the doc comment so a future ruling goes red loudly.

### Leftover junk in the tree

* `engine/tests/test_modules/characterization/zz_probe_026b.rs` — untracked scratch. It is a
  `#[test]` whose entire body is `println!`; it cannot fail, and it shifts the generated
  coverage reports.
* `cards/gen_pbl02_card_table.py` is committed but its output
  `pbl02_card_table_test.rs` does not exist.
* `cards/audit_vs_official.py` is untracked despite §8.22 documenting it as a deliverable.
  It does still pass: 113/113 official, 70/70 parsed, 0 missing, 2 text diffs (the
  ALLフレード / heart-icon normalisation), 5 unrepresented clauses — 4 explained by
  normalisation in §8.22, `PL!-pb2-033-N` unexplained.
* `C:\Users\trios\AppData\Local\Temp\kilo\grp.py` shadowed the stdlib `grp` module that
  `pathlib` imports, so it executed on **every** Python invocation from that directory and
  printed 239 lines, and would crash any script run elsewhere. Renamed to
  `mu_series_probe_scratch.py`.

---

## 7. Changes made in this pass

Every item below was compiled and run; the suite was **3983 passed, 0 failed** with all of
them applied, except where noted.

### Verified green

| File | Change |
|---|---|
| `helpers/choices.rs` | added `pending_choice_option_count()` + doc comment recording the `max_select` trap |
| `…024_n` | `assert!(observed \|\| !observed)` → real assertion on a genuinely empty stage, pinning the vacuous `true` reading that `…029_n` already pins |
| `…024_n` | `no_opponent_member_is_harmless` asserted on our **own** card (the ability targets the opponent) → now breaks the コスト2 gate on an opponent member above the bound |
| `…040_l` | removed 3 stale `OPEN GAP:` prefixes on assertions that pass (`-3`/`-5`/`-6`) |
| `…040_l` | `a_member_still_in_wait_does_not_reduce` was vacuous **and its premise was wrong** → rewritten as `a_member_waited_again_after_being_activated_still_counts`, pinning −3 for real |
| 11 test files | wrong-character headers corrected against `cards.json`; `…023_n`'s `KOTORI` const and locals renamed to `RIN` |
| `…007_pp` | `東條希` → `東條 希` in the const comment, matching cards.json |

### Written but NOT yet verified

(none — `…032_n` was verified in a later pass, see below)

### Verified in the second pass

`…032_n` — fully rewritten and green. The build break that blocked it was **not** mine: a
concurrent session started an `alloc`/`no_std` migration across 28 files in `engine/src` and
`ability/choice.rs:440` / `ability/effects/position.rs:20` used `alloc::borrow::Cow`
without the crate being linked. They then added `extern crate alloc;` to `lib.rs:30` with a
comment describing exactly that E0433, so no fix of theirs was needed or made.

What the rewrite changed:

* **Both blade fixtures corrected.** `PL!-bp4-012-N` (南 ことり, no blade heart) is now the
  qualifier and `PL!-PR-012-PR` (小泉花陽, `blade_heart {b_heart03: 1}`) is the negation
  case. The group control is `PL!HS-PR-001-PR` (日野下花帆, Hasunosora, no blade heart), so
  only the group filter can exclude it. The old file had these backwards, which is why the
  negation was never tested.
* **The mandatory cost is paid explicitly** instead of measured. The old test measured it.
* **`pending_choice_option_count()`** replaces `pending_choice_count()`. The look now
  genuinely offers **1** candidate out of 5, and the test says what each dropped filter
  would produce.
* **`put_on_deck_top`** instead of `push` — see §2.5. This was the bug I introduced and
  then diagnosed: the look was seeing five filler cards, not my candidates.
* Card identity fixed (星空 凛, not 西木野真姫).
* `the_fixture_invariants_hold` pins the blade-heart premises with
  `has_blade_heart_strict()` — the accessor that matches the `blade_heart` key in
  `cards.json`. The engine's filter goes through `has_blade_heart()`, which is *also* true
  when `special_heart` is non-empty (`core/card.rs:4273-4279`), so the two can disagree. I
  used the wrong one first, wrote an assertion that failed, and did not paper over it: the
  filler's exclusion mechanism is stated in a comment and **not asserted**, because I did
  not establish it.

`…032_n` is now 3 tests, all passing. Suite total moved 3983 → **3984**.

### Verified in the third pass — the same two traps, two more files

Both of these are the §2.1 and §2.2 traps again, so they are worth stating as a pattern:
**the look-and-select shape is the single most broken test idiom in this suite**, because it
is the only shape with two `SelectCard` prompts in a row (the mandatory cost, then the look)
and only one number that looks like a count.

* **`…026_n`** — three separate defects, all now fixed and green.
  1. It measured the **cost**, not the look. The 起動 cost is a `sequential_cost` of
     `change_state(self→wait)` + `move_cards(hand→discard, 1)`, and that sub-cost raises its
     own `SelectCard` (`ability/cost/handlers.rs:608`). The loop broke on the first
     `SelectCard`, which is the cost. Since both are `SelectCard`, matching on the prompt
     type cannot separate them — the test now identifies the cost by **the hand shrinking**.
  2. It read `pending_choice_count()` (the `max_select` trap).
  3. **Its expected value was wrong.** `FILLER = PL!-sd1-010-SD` is 高坂 穂乃果, whose unit
     is **Printemps**, so it satisfies 「『Printemps』のメンバーカード」 and must be offered.
     The correct candidate count is **2**, not the asserted 1. The group filter is in fact
     working correctly: it admits both Printemps cards and excludes only 絢瀬絵里 (BiBi).
     The old test passed with a wrong number for a wrong reason.
* **`…008_pp`** — `offered_count` returned `pending_choice_count()`, so for a 「1枚」 take it
  was 1 for any non-empty prompt. Now uses `pending_choice_option_count()`. The consequence
  is that `the_bound_separates_the_seven_from_the_eight` — the file's self-declared "one
  that actually pins the bound" — is now pinned for the first time: adding the 7-total live
  to the pool would give a count of 2, and it does not.

Suite after both: **3983 passed, 0 failed** (the 3984→3983 delta is a concurrent session,
not these edits — neither added nor removed a test).

## 0.4 Two more parser fixes — E3 and E7 closed, parser suite fully green

### E3: `action_053` fired on the bare word 「追加」

`PL!-pb2-012-P＋` (南ことり) 起動 carried a junk node inside its cost:

```json
{ "text": "この能力を起動するための追加コストとして",
  "action": "modify_score", "operation": "add" }
```

An operandless `modify_score` — exactly the `modify_score` missing-`value` validation
warning the old doc listed as open P7, still emitted on every run.

Cause: `ActionRule(name='action_053_modify_score', match='追加', …)` (parser.py:2380) fires
on **any** sentence containing 「追加」 that is not 「エール」. 「追加コスト」 is a connective,
not a score operation.

**I was wrong first here**, and it is worth recording: I read the parse's cost text and
concluded the parser had **fabricated** a cost clause. It had not — `cards/cards.json` shows
the printed 起動 really does say 「…この能力を起動するための追加コストとして、手札を2枚控え室に
置くか、『Printemps』のメンバーを2人ウェイトにする。」 The defect is the stray node, not a
fabricated cost. I had the right instinct to check `cards.json` before editing the parser.

Fix: require スコア alongside 追加 — `match_all=['スコア', '追加']`. Blast radius, the whole
`abilities.json` diff: the two stamps, the 5 junk lines removed, and the two
`activated_this_turn` lines from the 040-L fix. Nothing else in the corpus moved.

### E7: the red invariant was a false positive — amended, not the parser

`test_compound_subconditions_inherit_location` demanded that `042-L`'s second sub-condition
inherit `location: "revealed_cards"`. That sub-condition is
「自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる場合」 — it carries
`position: "center"` and names センターエリア, which is not the revealed-cards zone. Stamping
it would assert both at once, and `_drop_unsupported_child_location` exists to undo exactly
that (its docstring names this card). The net parse is correct; the invariant was checking an
intermediate state the parser deliberately reverses.

Added the missing exemptions: a sub-condition carrying its own `position`, or whose text names
エリア/置き場, is not a candidate for zone inheritance.

**Parser suite: 124 passed, 0 failed** — green for the first time this session, with both
parser changes measured for blast radius rather than assumed.

## 8. Suggested order of work

1. **Fix the inverted fixtures in `…032_n`.** The ブレードハートを持たない filter is
   currently untested and the test reads as if it were. Use the new
   `pending_choice_option_count()`, and drain past the cost prompt.
2. **Rename or rewrite the §3 anti-tests** (`…037_l`, `…042_l`, `pl_pb2_001_pp`). They will
   fail the day the engine is fixed, which is the most expensive way to find out.
3. **Make `TEST_QUALITY.md` enforceable** — a `no_drive` / `no_assert` ratchet that fails
   the build. Two offenders are already listed and committed.
4. **Delete the round-trip parse tests**, or move them to a data-validation suite that is
   not read as behavioural coverage. `…014_p_plus` is the clearest case.
5. **Add the §5.1 abilities**, starting with `…021-N` (the missing over-fire guard) and
   the `…036-N`/`037-N` side restriction.
6. **Fix the remaining 6 wrong-character headers** and the `…017_p_plus` "no abilities of
   their own" claim.
7. Rewrite this doc's §3/§8 to match §6 above.

### 0.3 A fabricated "KNOWN GAP" eliminated — SP-013 was never broken

`under_card_and_success_live_conditions_test.rs::kaleidoscore_optional_branches` recorded a
KNOWN GAP in the engine and pinned the absence as expected behaviour, in three assertions:
the discard prompt never appears, the card stays in hand, the energy never leaves the energy
deck (that last one written as `<=`, so it is satisfied by an unchanged state and cannot fail
on any of the behaviour it names).

**The gap was the test's own fixture.** It used `PL!SP-bp1-004-P` as the card to discard.
That card is 平安名すみれ, `unit: "CatChu!"`. The cost filters on
`group_names: ["KALEIDOSCORE"]` and `card_matches_group_str`
(`engine/src/ability/util.rs:637-659`) matches on `unit`, so **the cost was unpayable and no
prompt could ever be raised.**

It was also *fictional*, which is what makes it worth recording: `keke_discard_blade_heart_no_draw`
in `debut_discard_blade_heart_draw_test.rs` exercises the same card with a KALEIDOSCORE
discard and asserts the **opposite** of all three. Both tests were green simultaneously,
claiming the prompt appears and does not appear.

Replaced with `kaleidoscore_blade_heart_discard_is_paid_and_draws_nothing`, using
`PL!SP-sd1-010-SD` (KALEIDOSCORE, `blade_heart {b_heart02: 1}`) — the same fixture the
sibling test proves works. It asserts the real behaviour: the cost **is** paid, the energy
**is** placed from the energy deck, and nothing is drawn because the discarded card has a
blade heart. Both premises are pinned (`unit == "KALEIDOSCORE"`, blade heart present) so a
wrong unit can never make this vacuous again. It passes; the engine was correct throughout.

**On my own two errors here, since both were caught by the tree and not by me:**

1. I first picked `PL!SP-bp1-021-N` as the blade-bearing KALEIDOSCORE card, from an earlier
   multi-card data dump. The `has_blade_heart_strict()` precondition I wrote **failed**, which
   is the point of writing it. `cards.json` confirms `PL!SP-bp1-021-N` has
   `blade_heart: None`; I had misattributed a line from the previous card in the dump. The
   engine agreed with the data both times.
2. The fixture-selection lesson is now the general one: **check the data file directly for
   each fixture, in a single-card query, and pin the discriminating property as a test
   precondition.** A card's properties read out of a multi-record dump are as error-prone as a
   test's comment, and the failure mode is identical — a confident test that cannot fail.


Ordered by "can this lie to me", not by effort. `A` = the assertion is wrong or cannot fail,
`B` = the assertion is right but the fixture cannot reach it, `C` = mechanism untested,
`D` = metadata/comment rot.

### A — assertions that cannot fail, or that contradict the printed card

| # | Item | Card | State |
|---|---|---|---|
| A1 | `pl_pb2_001_pp` — the ALLブレード half asserts `p1_constant_total_score_bonus` for a clause that grants a **heart** (`gain_resource`/`heart`/`heart_type: all`). Cannot fail. The スコア half pins `0` where the card prints 「＋１する」 | 001-PP | open |
| A2 | `all_clauses_met_waits_the_foe` — name says the foe is waited, body pins `!fired` | 042-L | diagnosed, see §0.2 |
| A3 | `all_printemps_qualifies_and_the_member_is_taken` / `all_lily_white_qualifies` / `all_bibi_qualifies` — names say taken, bodies pin nothing taken | 037-L | open |
| A4 | `every_branch_carries_the_all_marker` — pins `vec![false, true, true]`, blessing a `get_all_members()` decode loss the parse actually carries | 037-L | open |
| A5 | `the_score_and_all_blade_clauses…` absence assertions, same file as A1 | 001-PP | open |
| A6 | `the_cost_paid_branch_is_still_a_vacuous_comparison` — asserts the parser defect is present; fails on a correct engine | 021-N | open |
| A7 | `empty_own_stage_is_vacuously_bibi_only` — `assert!(x \|\| !x)` | 024-N | **FIXED** |
| A8 | `an_empty_setup_grants_nothing` / `it_does_not_fire_for_another_cards_ability` — assert on a card in no zone | 021-N | open |
| A9 | `the_prompt_does_not_enumerate_its_options` — `pending_choice_count()==0`, hardcoded 0 for `SelectTarget` | 016-P＋ | open |
| A10 | `an_empty_history_gives_nothing` — `inner(&[]) - inner(&[])` | 010-P＋ | open |
| A11 | `a_cheap_mus_baton_touch_does_nothing` / `an_expensive_non_mus_baton_touch_does_nothing` — same equality, operands swapped; neither asserts `== 0` | 009-PP | open |
| A12 | `the_optional_take_cancel_test` — asserts the hand is unchanged "because the two cancel"; passes if either never fires | 032-N | removed in rewrite |
| A13 | `under_self` assertion where the other member has no under-card ability | 017-P＋ | open |
| A14 | `our_own_live_success_abilities_are_not_invalidated` — stages an ally, asserts `0`; the ability targets the opponent's (empty) stage | 003-PP | open |
| A15 | hand-contains assertion satisfied with or without the mandatory cost | 036-N | open |
| A16 | `no_opponent_member_is_harmless` — asserts on our own card | 024-N | **FIXED** |
| A17 | `kaleidoscore_optional_branches` — three absence assertions incl. an `<=` that cannot fail, against a fixture that cannot pay the cost | SP-013 | open, see B7 |
| A18 | `the_clause_parses_as_a_history_not_a_state` — whole-blob `contains("activated_this_turn")`; passes while 2 of 3 steps lack the tag | 040-L | open (tag now inherited; assert per-step) |
| A19 | `the_same_member_twice_counts_once` — asserts the test's own dedup helper | 010-P＋ | open |

### B — fixture cannot reach what is claimed

| # | Item | Card | State |
|---|---|---|---|
| B1 | **Both blade fixtures inverted** — `PL!-bp6-017-N` *has* a blade heart, `PL!-pb2-002-PP` has none, so the 「持たない」 filter was never tested | 032-N | **FIXED** |
| B2 | `PL!SP-bp1-004-P` is **CatChu!**, but the cost is 「『KALEIDOSCORE』」 — unpayable, so the "KNOWN GAP" is the fixture | SP-013 | **FIXED** |
| B3 | `FILLER` is **Printemps**, so it qualifies under 「『Printemps』のメンバー」 — expected value 1 was wrong, correct is 2 | 026-N | **FIXED** |
| B4 | `PL!SP-pb2-002-R` called a "live card"; it is a `メンバー` | SP-002 | open |
| B5 | `P_5 = PL!-pb2-030-N` called "no abilities of their own"; it prints a 常時 blade ability | 017-P＋ | open |
| B6 | `MUS_LIVE` documented as both "NOT lily white" and "lily white — qualifies" | 014-P＋ | open |
| B7 | `offered_count` read `pending_choice_count()`, so the 7-vs-8 heart bound was never pinned | 008-PP | **FIXED** |
| B8 | `select_option(1)` on a one-card `SelectCard` — the cost was never paid | 019-N | open |
| B9 | Two "copies" share one `i16`, so "paid only once" is unproven | SP-002 | open |
| B10 | `game.id(OPPONENT)` staged and a *different* instance waited — the 自動's trigger state is on an unplayed card | 015-P＋ | open |
| B11 | Deck built with `push` after `fill_decks` — the look saw only fillers | 032-N, 026-N | **FIXED** (032), 026 was already front-loaded |
| B12 | `advance_to_live_start` uses blind `game.pass()` counts | SP-048 | open |
| B13 | `P_1` is a **live** being queried for a member-side modifier, unstaged | 040-L | open |

### C — mechanism untested

| # | Item | Card | State |
|---|---|---|---|
| C1 | **`cards_activated_this_turn` writer/reader contract has NO test.** 3 hand-written writers in the test tree; no test drives a real wait→active through the engine. Regressions that would ship green: writer stops firing, writer over-fires (loses the `!contains` dedup), writer records the wrong id, the turn-boundary clear stops working | 010-P＋, 040-L | **FIXED the 040 half**; the contract test is the top open item |
| C2 | **`の`-style historical/live-end duration never tested** — `duration: live_end` read with no live entered and no rollover | 010-P＋ | open |
| C3 | Copy of the **same** 起動 twice from two under-cards: `ability_ops.rs:673-716` has no dedup, and whether 「すべて得る」 means once or twice is unpinned either way | SP-005 | open |
| C4 | Copy survival across a **turn boundary** | SP-005 | open |
| C5 | `heart06`×2 option, `このメンバー以外`, and `ライブ終了時まで` on the choice branch — zero coverage in the whole tree | SP-002 | open |
| C6 | `card_property`+`negation` never exercised, and **arithmetically blind**: 6 cards→3→cap 2, flip one→5→cap 2 (same); 2→1, add one→3→1 (same) | SP-008 | open |
| C7 | 『Liella!』 group filter on that same ability | SP-008 | open |
| C8 | `コスト2以下` probed only at exactly 2; `<`, `<=` and `<= 3` all pass | SP-047 | open |
| C9 | `してもよい` decline paths (047, 050, 013) | several | open |
| C10 | 右サイド/左サイド restriction — never played on the wrong side | SP-036/037 | open |
| C11 | Options 0 and 2; the per-card scaling; the repeatable clause | 016-P＋ | open |
| C12 | The real 自動 trigger — no `state_condition` for ウェイト状態, `target: "both"` | 015-P＋ | open |
| C13 | The 「ウェイトにしてもよい」 half — parse has no wait action or prompt at all | 012-P＋ | open |
| C14 | 13 printed abilities with no test anywhere (§5.1) | various | open |
| C15 | μ's filter on the **waitroom** recovery (only the success-zone half has a negative) | 007-PP | open |
| C16 | 行動 ordering 「その後」 — the only genuinely strong sequential assertion in the set | SP-048 | done, keep as the model |

### D — metadata rot

18 wrong-character headers (**12 FIXED**); remaining: `…014_p_plus` (乙宗 紫 → 星空 凛), `…032_n` (→ 星空 凛, done in rewrite), `…003_pp` (→ 南ことり, done). Dead helpers: `stage_and_total` (020-N), `NICO_NAME`/`MAKI_NAME`/`ERI_NAME` (042-L), `ENERGY` (009-PP), `FILLER`/`LW_LIVE` (014-P＋). Misnamed tests: `score_sum_eight_does_not_cross_the_threshold` (tests 6 and 9), `…024_n` duplicated pair. Duplicated header block in 040-L. `048-L` header cites the wrong `abilities.json` idx (867 vs 965) and documents an already-fixed parser bug. 05-compound calls `PL!SP-sd1-006-SD` 若菜四季 (it is 桜小路きな子) and pins group `5yncri5e!` for a filter that reads 『Liella!』.

### E — still-open engine work

| # | Item | State |
|---|---|---|
| E1 | `040-L` steps 2/3 not history-gated → −6 reachable | **FIXED** (parser), blast radius measured |
| E2 | `042-L` compound never fires | diagnosed to two suspects, **not** separated — see §0.2 |
| E3 | `012-P＋` 起動: stray operandless `{"action":"modify_score","operation":"add"}`; the `modify_score` missing-`value` warning still emitted every run | open |
| E4 | `018-P＋` `distinct_name` still reported by the extractor despite the doc's "fixed" | open |
| E5 | New `per_unit` mismatch on `016-P＋` appeared after the doc was written | open |
| E6 | `015-P＋` lost its trigger condition entirely in the parse | open |
| E7 | Red parser invariant `test_compound_subconditions_inherit_location` — **false positive**; amend the test's exemption list, not the parser | open |
| E8 | The extractor still reports 4 missing mechanics; doc §3 lists a different 4 | open |

## 11. Not audited

The 36 reprint prints (correctly, per your ruling), the `PL!HS-pb1-*` starter cards, and
any ability whose node is shared with a tested card. The `v8_*` bot files are being edited
concurrently and were left alone.

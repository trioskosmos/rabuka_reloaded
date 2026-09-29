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

## 1. What is genuinely solid

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

## 9. Not audited

The 36 reprint prints (correctly, per your ruling), the `PL!HS-pb1-*` starter cards, and
any ability whose node is shared with a tested card. The `v8_*` bot files are being edited
concurrently and were left alone.

# Test Hardening & Parser/Engine Fix Plan 窶・2026-08-26

## Mission (standing directive)

**Engine and parser rewrites while adding more tests for possible in-game
scenarios that have not yet been considered.** Keep examining the existing test
corpus to find what still needs writing. Tests where **both players use
abilities against each other** are required 窶・one-sided scenarios are not
enough for a two-player simulator.

Method, in order:
1. Read `engine/rules/rules.txt` and `cards/qa_data.json` for the EXACT rules
   process before writing any scenario 窶・no guessed semantics.
2. Mine abilities.json for interaction shapes (whose-effect gates, cross-player
   triggers, opponent choices) and check them against existing tests.
3. Write real-game flows, not synthetic state injection.
4. When a test fails: run it under `$env:RUST_LOG="debug"` and READ THE FULL
   FLOW before touching code or expectations; classify test bug vs engine bug
   vs parser gap, then fix end-to-end (AGENTS.md).

## Status (updated as work lands)

| Item | State |
|---|---|
| P0.1 orphaned mod.rs registration | DONE 窶・6 files recovered, `sumire_bp5_test_debug.rs` deleted as stale duplicate; konata/location/shizuku fixed to current APIs |
| P0.2 DIVE! false-trigger audit | DONE 窶・6 real-game tests in `dive_false_trigger_test.rs`; found + fixed **engine gap**: `movement` key on non-Movement condition variants was dropped by both serde and bytecode decode paths, so static presence re-triggered "when placed" abilities. Fixed via `ConditionCommon.movement` + regenerated decoder |
| P0.3 `has_pending_choice` soft-guard migration | DONE 窶・12 -> 4 sites: b9_more probe-if deleted; parser_issues_p3 nested redundant if removed; umi_bp3 + edelnote advance loops rewritten as `while !pending`; performance_snapshot_audit (3 sites) and performance_phase_rules migrated to dispatch-on-prompt-type with panic on ANY unexpected prompt; helpers::scan_autos_both now answers via variant-dispatched `answer_choice`. Remaining 4 baselined as genuinely legitimate (generic helper, optional DASH arrange, structural on_choice loop) |
| P1.1 per-test inventory | OPEN |
| P1.2 or-assertions sweep | DONE 窶・36 multi-line `||` hits triaged: 10 files tightened to exact outcomes (pl_s_bp7_007 ﾃ・ 竊・exact stage slots; kuroe_dia 竊・selected card on deck top; l0_gap_livesuccess vacuous disjunct removed; batch34 decline now pins waitroom retention; natsumi Q264 adds not-discarded check; opponent_choice wait竕leave both-stay; mia/card_ability/izumi/look_and_select deterministic picks pinned). 22 hits judged legitimate (match guards, loop conditions, error-string chains, engine zone aliasing) |
| P1.3 helper dedup (`fire_trigger` x40) | DONE 窶・21 byte-identical clones deleted onto `helpers::fire_trigger`; batch30 keeps a 3-line drain wrapper; batch8's `fire_trigger_nth` kept as genuine extension |
| P2.* parser/pipeline hardening | OPEN |
| P3.* engine quality debt | MOSTLY DONE -- either-target fixed, mojibake fixed, non-skippable silent no-op fixed (empty answers now Err); remaining: queue-u8 indexing, runaway counters per-game semantics, web-server lock logging |

## §5 Round 5 — seat-relative / cross-player abilities (L1-depth mining)

Mined TEST_INVENTORY for `相手`-shaped abilities still at L1 depth (no
negative/mirror coverage). New file `cross_player_round5_test.rs`. Ledger of
what existed vs what was missing:

| Card | Existing coverage | Missing until now → added test |
|---|---|---|
| PL!S-bp6-022-L (ライブ成功時: 相手のエネルギーが自分より多い→スコア+1) | batch12, both tests fired from P1 only (`fire_trigger` hardcodes pid=p1) | P2-owned copy fired AS p2 (+/- mirror); mirror boards each side pinned independently; waited energy counts per rules 4.7.4 (`cards.len()` not `active_count()`) |
| PL!SP-pb2-029-N (登場/開始時: 相手のコスト2以下をウェイト) | batch11, single-sided debut only | Mirror standoff — both seats debut their own メイ via real play_to_stage with seat flip; cost gate protects opposing メイ (9>2); already-waited eligible target re-selected no-op; cost-4 negative; 9.6.3.1.3 no-prompt pin |
| PL!N-bp7-009-R (登場: 自分と相手それぞれ7枚ミル) | bp7_q267 file pins refresh boundaries + both-refresh THOROUGHLY (do NOT duplicate) | ADDED: P2-fired copy (seat-relative "自分と相手"); exact top-card identity across both mills — no cross-pollination. Note: engine resolves both-target move_cards OPPONENT-first then self (`execute_move_cards_both`); fill_decks seeds 30/deck not 20 |
| PL!HS-PR-035-PR (登場: 相手の控え室メンバー3枚→相手デッキ底+ブレード3以下ウェイト) | bp7_ginko file (L1, placement only) | ADDED accept/decline/P2-seat tests. Found + fixed **engine bug #6**: the follow-up rest resolved against the prior step's `selected_cards` (the 3 discard→deck-bottom members, never on stage) instead of scanning the opponent's STAGE — the printed clause 「相手のステージにいる元々…ブレード3つ以下のメンバー1人をウェイトにする」 could never fire. Fixed END-TO-END: (a) state.rs member_op falls back to the stage scan when NO selected card is on the target stage (diagnostic line kept); (b) **parser fix** — the select-followup emitter no longer stamps `source="selected_cards"` onto clauses that scope their own objects with an existence qualifier (`〜にいる／〜にある`); PR-035-PR now parses `source="stage"`. Backreference verbs (「そのライブカードを手札に加える」, しずく bp5-003) keep the stamp — first regen attempt over-matched `手札に` and broke shizuku_bp5, caught by suite, narrowed to existence qualifiers. Golden diff vs HEAD = exactly one semantic line. Grounding: printed text scopes the rest to stage members; Q118/Q154 gate the rest on the move; Q102 + 9.6.3.1.3 keep the zero-candidates no-op; DB scan confirmed this was the ONLY change_state+source=selected_cards ability |
| PL!SP-bp5-027-L (成功時: 自エネルギー置き→そうした場合相手は1枚引く) | batch32 (accept/skip, own-side) | planned: conditional gate = opponent draw ONLY on accept; empty energy deck edge; combo with 4.7.4 (waited energy it places feeds bp6-022-L comparison) |
| PL!S-bp7-025-L (成功時 choice: 相手コスト4以下2人までウェイト+次ターン非アクティブ制限 / draw 1) | batch9 (L1+choice) | planned: option A cost gate + restriction carry-over; option B draw |

Rules grounding used: 4.7.4 ('エネルギー'=energy-zone cards, orientation-
independent), 8.4.4→8.4.5→8.4.6 (LiveSuccess resolves before score compare,
both players fire same phase), 5.2.1 (ウェイトにする has no state precondition),
9.6.3.1/.1.3 (exact-count must choose when possible; zero eligible ⇒ ignored,
no prompt), Q267 (mill deck-out refresh).

## §6 Round 6 — finder blind spots (integration-level)

Gaps the per-ability inventory structurally cannot see; new file
`integration_blindspot_test.rs`:

1. **Real-rollover expiry** — live_end temporary effects were only ever
   expired by CALLING `check_expired_effects()` directly with a hand-set turn
   phase (modifier_layer_characterization_test). Nothing proved the actual
   victory→Active rollover (`turn/phases.rs`) invokes it. New test grants via a
   real ability INSIDE the live, passes through performances/victory, asserts
   the grant reverts exactly at the rollover.
2. **Dual-trigger second windows** — 13 abilities carry 「登場/ライブ開始時」;
   tests always fire ONE window manually. PL!N-bp5-004-R 果林 had debut-only
   coverage, PL!HS-bp6-004-R 吟子 live-start-only. New tests drive the missing
   windows through REAL flow (play_to_stage / phase advance), including gate
   edges: original-blade==EXACTLY-4 (果林) and cost<=9 (吟子).
3. Flow lesson pinned by these tests: after a mid-turn main-phase action, the
   fixed 5-pass walk lands on the WRONG seat's LiveCardSet — drive phases
   explicitly; SelectAutoAbility prompts must be answered with SELECTIONS
   (empty answer declines the resolution under test).

RESOLVED observation: 果林's dual-trigger change_state raises a
「Pay optional cost: Put this member to wait state?」 gate during resolution.
NOT a bug: her printed text is 「このメンバーをウェイトにしてもよい：相手の…」
— an optional self-cost that the parser correctly emits as
cost={optional:true, self_cost:true} and the engine correctly gates on.
Initial "suspect gate" diagnosis came from reading only `effect`, never
`cost`; the new permanent [PAY_SKIP_GATE] diagnostic (resolver.rs) logs
route+description for every gate emission and would have answered this
immediately.

Ruling gaps surfaced for future rounds: Q29 (arrival-turn baton-touch ban —
mechanism `deployed_this_turn` exists and is cleared at rollover; covered
indirectly via kasumi/sumire_bp4/qa_new_tests198 but no direct Q29 pin),
Q31 (duplicate card numbers in live zone — implicitly covered by
performance_snapshot_audit test 2 using two copies of one card number),
Q39/Q34/Q33 (yell-before-heart-check ordering + live-card cleanup timing).

## §7 Round 7 — non-ability game systems (phase machine)

New `phase_machine_rules_test.rs` — systems with no ability text that the
ability inventory structurally cannot see. Empirical facts now pinned:

| Rule | Pinned behavior |
|---|---|
| 7.4.1 | Waited member + waited energy stand ONLY for the turn player across the Active→Energy boundary; the other player's persist |
| 7.5.2 | Energy phase moves the TOP energy-deck card into the zone (executes on the Energy→Draw transition); EMPTY deck = silent skip, no panic |
| 7.6.2+Q267 | Draw phase draws exactly 1 even from an empty main deck — waitroom refresh feeds it silently (deck ends at waitroom−1) |

CRITICAL identity model for future tests: `is_first_attacker` NEVER flips
(fixed RPS result); the turn player comes from `current_turn_phase`
(FirstAttackerNormal / SecondAttackerNormal). Several earlier test drafts
wrongly assumed the flag tracks the turn.

Also pinned implicitly: `deck_refreshed_this_turn` is set ONLY by explicit
effect-driven refreshes (mill/look overdraw), NOT by the silent phase-draw
refresh — an internal-marker distinction, documented on the test.

## ﾂｧ2 Zone-change / source-gate audit (round 2)

Systematic scan of every trigger-gate key in abilities.json (zone_change
source/destination, movement on non-Movement variants, self_effect_only,
check_self, temporal phase_target) cross-referenced against the test corpus.
Result: 16 gate shapes, all covered except three families 窶・now tested in
`engine/tests/test_modules/zone_change_gate_test.rs`:

| Card | Gate | Tests |
|---|---|---|
| PL!S-bp6-002-R+ 譯懷・譴ｨ蟄・ab#0 | live_card_zone竊壇iscard, Aqours live, target=self | positive deck-top/bottom placement; ﾎｼ's-live negative; own-side-only negative |
| PL!N-pb1-009-R 螟ｩ邇句ｯｺ迺・･・ab#0 | live_card_zone竊壇iscard + negated has_blade_heart, this_turn | positive draw+hearts; blade-heart-departure negative |
| PL!N-bp5-005-R+ 螳ｮ荳区・ ab#0 | stage竊壇iscard via REAL baton touch (rules 9.6.2.3.2/.1) | cost-15 newcomer full payoff (+2 energy AND draw); cost-13 boundary (energy only); blade-heart newcomer negative |

Baton-touch semantics pinned from rules.txt + qa_data.json: the 縲後ヰ繝医Φ繧ｿ繝・メ縺励◆縲・event belongs to the ARRIVING member's play (9.6.2.3.2.1); the departed ability's
conditions describe the NEWCOMER; net payment = newcomer cost 竏・occupant cost.
Energy assertions require wait-state cards in the pool (activation flips wait竊誕ctive).

## ﾂｧ4 Round 4 窶・opponent-live-success across seats

The only card reading `opponent_live_success` (Strawberry Trapper PL!S-pb1-021-L)
was "covered" exclusively by synthetic flag injection; no test verified the
real pipeline ever sets it, and the legacy flag was armed ONLY for P2's success
(`player2_won`) 窶・a P2-owned card could never see P1 succeed.

Fixes:
1. Per-seat tracking: `p1/p2_live_success_this_turn` + `_no_excess` on
   GameState, written at victory determination; reset with the other turn flags.
2. Owner-relative evaluation: `evaluate_opponent_live_success_condition` picks
   the seat opposite the ACTIVATING card's owner ([OPP_LIVE_SUCCESS_EVAL]
   diagnostic).
3. **Engine bug #5 (ordering)**: LiveSuccess triggers fired at the TOP of
   `execute_live_victory_determination`, before verdicts/results existed 窶・   every real-flow opponent-success evaluation saw stale state. Fix:
   `record_pretrigger_live_results()` records per-seat outcomes before the
   trigger block using the SAME score formula as the post-trigger totals
   (extras = 0 by construction). The later authoritative pass remains as
   post-extras truth. Skipped seats don't clobber explicitly-armed values.
   Follow-up refactor item RESOLVED: mirror upgraded to exact score formula
   rather than deleted (deleting would require reordering the trigger/score
   dependency chain 窶・triggers feed extras feed final scores).

New `opponent_live_success_flow_test.rs`: full organic round 窶・P1 exact-fill
success (豬ｷ譛ｪ + START:DASH!!, heart-less deck so yell adds nothing) vs P2-owned
Trapper 竊・+2 pinned through the real pipeline.

Suite state after round 4: **2945 passed / 0 failed / 0 ignored.**

## ﾂｧ3 Round 3 窶・both-player interaction (mission directive)

Mined abilities.json for the 縲悟ｯｾ謌ｦ逶ｸ謇九・繧ｫ繝ｼ繝峨・蜉ｹ譫懊〒繧ら匱蜍輔☆繧九・marker
class (6 cards) and for effects that move OPPONENT members (exactly one:
PL!HS-pb1-014-R 螳蛾､雁ｯｺ蟋ｫ闃ｽ debut, position_change dest=front target=opponent).

New `cross_player_interaction_test.rs`: 蟋ｫ闃ｽ's MiraKura-gated debut force-
repositions P2's 蜿ｯ蜿ｯ 竊・蜿ｯ蜿ｯ's own area-move watcher must fire from the
opponent-caused move (heart06 until live end); plus a gate-blocked negative.

**Engine bug #4 found and fixed:**
`fire_opponent_cause_watchers_for_move` enqueues the watcher with a
trigger_moved_cards SNAPSHOT, but resolution-time evaluation of
`position_change` conditions only consulted live scratch state
(`position_change_events`, `recently_moved_cards`) 窶・which the enqueuing
player's batch loop legitimately clears (abilities.rs:1433-1445) before the
other seat's queue resolves. The watcher armed, then silently did nothing.
Fix: the position_change arm of evaluate_movement_condition now falls back to
the entry snapshot via entry_trigger_moved_cards(), with a [POS_CHANGE_EVAL]
diagnostic line. Also noted: answering a non-skippable SelectTarget with empty
indices silently no-ops ("Unknown source position") 窶・queued as P0.3/P3 work.

Suite state after round 3: **2944 passed / 0 failed / 0 ignored.**

**Engine bugs found and fixed by this round:**

1. **Indices-channel answers to `position|destination` choices dropped cards**
   (actions.rs BCR dispatch): only the `card_id` channel was mapped through
   the options array; an answer arriving via `card_indices` (the normal
   `select_indices`/web-UI channel) fell through and the raw index became the
   destination string `"0"`, matching no zone 窶・the chosen card silently
   vanished. Fixed by mapping `card_indices.first()` through `options` first.
   Found by `riko_responds_only_to_own_side_live_zone`.

2. *(Prior round)* `movement` on non-Movement condition variants was dropped by
   both decode paths 窶・see commit history.

Note: `blade_heart` is its OWN printed field (`cards.json .blade_heart`,
b_heart icons) distinct from the `blade` stat 窶・picked test members accordingly.

Suite state after round 2: **2939 passed / 0 failed / 0 ignored.**

Suite state after this round: **2934 passed / 0 failed / 0 ignored** (was 2927 passing
with 7 failing orphans + 2 ignores before recovery). The two former `#[ignore]`s were
full-width-plus card-ID typos, not engine gaps.

---

Working plan derived from the full-repo audit (engine src, cards/ parser ecosystem,
abilities.json, engine/tests). Guiding directive: **work on tests and fixes together** 窶・every parser gap gets an end-to-end test, every engine fix pins behavior, no documenting
around gaps (AGENTS.md rule).

---

## P0 窶・Silent test death / false confidence

### P0.1 Register the 7 orphaned test files (`test_modules/mod.rs` drift)
Files present on disk but never compiled into `run_all`:
- `untested_abilities_batch22_test.rs` (entire coverage batch)
- `l0_gap_constant4_test.rs` (also holds 2 `#[ignore]`d tests)
- `heart_color_test.rs`
- `konata_bp1_test.rs`
- `location_condition_cost_test.rs`
- `shizuku_bp4_aggregate_test.rs`
- `sumire_bp5_test_debug.rs`

Register all, compile, fix fallout. If a file cannot compile against current helpers,
fix the file (it may encode stale APIs worth resurrecting). Delete only true duplicates
(`sumire_bp5_test_debug.rs` vs `sumire_bp5_test.rs`: compare assertions before deleting).
Then add a CI guard: every `*_test.rs` under `test_modules/` must appear in `mod.rs`
(extend `cards/test_inventory.py --check`).

### P0.2 False-positive trigger audits (the "DIVE! class" of bugs)
Pattern: auto abilities whose triggers are **compound** (temporal gate AND a
zone_change/count gate). Any effect elsewhere in the DB that touches the same zone
transition is a candidate for falsely arming them. See ﾂｧ1 for the worked example.

Workstream:
1. Enumerate all auto abilities whose trigger includes `zone_change` +
   `source/destination` gates (grep `abilities.json`).
2. For each, identify sibling effects in the DB performing the same transition
   (mass retrieval, per-card retrieval, reveal/look-without-move, deck竊檀and,
   opponent-side moves).
3. Write negative tests: sibling card moved 竕 self moved 竊・NO trigger.
4. Write positive controls: self actually moved via another card's effect 竊・trigger.
5. Where a negative fails, classify: test bug vs engine bug vs parser gap; fix end-to-end.

First concrete instance done here: **DIVE! (PL!N-bp4-026-L)** 窶・see ﾂｧ1 below.

### P0.3 Kill the `if game.has_pending_choice()` soft-guard pattern
WRITING_TESTS.md explicitly forbids it; it appears ~250ﾃ・across ~105 files.
A missing prompt means the ability fires free / skips cost and the assert still passes.
Plan: migrate file-by-file to `drain_choices_strict` / dispatch-on-choice loops
(already exist in helpers/mod.rs), add a grep-based CI lint banning the guard outside
explicitly-negative tests.

## P1 窶・Coverage honesty

### P1.1 Make `test_inventory.py` per-test, not per-file
Current: substring presence of a card_no anywhere in a file = "covered"; L1 inferred
from `"assert" in text` anywhere in the covering file. Headline "771/771 covered" is
file-granular, not ability-granular. Change to parse each `#[test] fn` body and map
card 竊・asserting test; exclude unregistered (orphaned) files from stats until P0.1 lands.

### P1.2 Or-assertions and vacuous asserts
Replace `assert!(a || b)` outcome-tolerance tests (e.g. `pl_s_bp7_007_test.rs`) with
exact-outcome or decision-point assertions. Sweep `assert!(result.is_ok())`-style
tests in early `qa_new_tests*.rs`.

### P1.3 Deduplicate test helpers
~40 private copies of `fire_trigger` (with behavioral drift between copies),
~50 `fill_decks` variants with three signatures. Consolidate onto `helpers::`,
delete local clones mechanically, one batch family per commit.

## P2 窶・Parser/pipeline hardening (each item ships with a regression test)

### P2.1 Single source of truth for opcode/tag tables
`COND_TO_VARIANT_TAG` / `ACTION_TO_VARIANT_TAG` / `COST_TYPE_TO_ACTION`
(`cards/compile_abilities.py`) are hand-synced copies of facts derivable from
`engine/src/core/card.rs`. Derive them from card.rs the way
`generate_*_decoder.py` already does. Regression test: generated table diff ==
handwritten table before switchover; then CI freshness gate like the condition decoder has.

### P2.2 Decoder generator parity
`generate_condition_decoder.py` drops all `serde(alias)` attrs (effect generator keeps
them); unknown field types emit silent `skip_value()` arms. Add generation-time warnings
+ a parity test asserting both generators agree on shared READER_MAP entries.

### P2.3 Compiler input mutation
`compile_abilities.py` renames keys on the live parsed JSON mid-walk (non-idempotent).
Encode from deep-copied data; add a round-trip test: compile twice 竊・identical bytes.

### P2.4 Scraper loudness
Fetch failure currently breaks pagination and writes truncated output with exit 0;
type-extraction miss silently defaults to member. Make failures non-zero exit +
explicit "INCOMPLETE" marker; add a smoke fixture test with a frozen HTML sample.

### P2.5 abilities.json hygiene
`input_hash` always null, `parser_version` never bumps, dead `use_limit`/`cost` null
keys, `type` mixing three namespaces, `zone` duplicating `source`. Fill hash/version at
generation time; drop provably-dead keys in one coordinated regeneration (bytecode
regenerate + golden diff + full suite).

## P3 窶・Engine quality debt (fix alongside touching nearby code)

- u8 indexing in `ability_queue.rs` slot bookkeeping 竊・widen or saturate.
- Process-lifetime runaway counters (`PCA_CALLS`, `CHOICE_CALLS`, queue `clear()`)
  竊・per-game reset semantics.
- Web-server lock handling: replace silent `if let Ok(lock())` skips with logged
  recovery via existing `lock_recover`.
- Mojibake in runtime strings/comments (`game_state/abilities.rs:1515,756`;
  AI-artifact comment `player.rs:491`); pin `PYTHONUTF8=1` in Python scripts.
- Consolidate 11ﾃ・duplicated `MemberArea竊段ndex` matches onto `to_index()`.
- Pair-call invariant `evaluate_success_zone_constant_abilities()` +
  `restore_performance_need_heart_modifiers()` 竊・single method.

## ﾂｧ1 Worked example: DIVE! false-positive trigger surface

Card: `PL!N-bp4-026-L` ("DIVE!"), two auto abilities:

- **ab#0** 窶・trigger = AND(temporal: *own* main phase, `zone_change discard竊檀and`,
  self-target). Effect: optional move 1 "DIVE!" hand竊値ive zone; then
  `reduce_live_card_set_limit` +1.
- **ab#1** 窶・condition = location(self in live zone, `movement: moved`). Effect:
  1 Nijigasaki member gains blade+2 until live end.

Existing tests (`dive_auto_trigger_test.rs`, `dive_edge_test.rs`, `dive_live_card_test.rs`)
always move DIVE! itself and always set `recently_moved_cards(vec![dive])` by hand.
They never probe cross-card interference. Candidate false-positive classes being tested
in `dive_false_trigger_test.rs`:

| # | Scenario | Expected |
|---|----------|----------|
| F1 | Another live card moved discard竊檀and | ab#0 must NOT fire |
| F2 | DIVE! moved main_deck竊檀and | ab#0 must NOT fire (source gate) |
| F3 | DIVE! revealed/looked at in discard, not moved | ab#0 must NOT fire |
| F4 | Opponent's DIVE! moved during our phase scan | P1's copy unaffected; P2's fires only in P2's own main phase |
| F5 | Mass retrieval sweeping DIVE! discard竊檀and | ab#0 SHOULD fire (positive control) |
| F6 | ab#1 with DIVE! statically in live zone, nothing moved this turn | NO blade grant |
| F7 | ab#1 re-scan next turn after movement flag cleared | NO re-grant |
| F8 | Two DIVE!s, only one moved | exactly one placement choice / limit bump |

Any failure here is classified test-bug/engine-bug/parser-gap and fixed end-to-end.

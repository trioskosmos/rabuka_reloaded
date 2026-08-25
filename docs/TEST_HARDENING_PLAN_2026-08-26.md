# Test Hardening & Parser/Engine Fix Plan — 2026-08-26

## Status (updated as work lands)

| Item | State |
|---|---|
| P0.1 orphaned mod.rs registration | DONE — 6 files recovered, `sumire_bp5_test_debug.rs` deleted as stale duplicate; konata/location/shizuku fixed to current APIs |
| P0.2 DIVE! false-trigger audit | DONE — 6 real-game tests in `dive_false_trigger_test.rs`; found + fixed **engine gap**: `movement` key on non-Movement condition variants was dropped by both serde and bytecode decode paths, so static presence re-triggered "when placed" abilities. Fixed via `ConditionCommon.movement` + regenerated decoder |
| P0.3 `has_pending_choice` soft-guard migration | OPEN (~250 sites) |
| P1.1 per-test inventory | OPEN |
| P1.2 or-assertions sweep | DONE — 36 multi-line `||` hits triaged: 10 files tightened to exact outcomes (pl_s_bp7_007 ×3 → exact stage slots; kuroe_dia → selected card on deck top; l0_gap_livesuccess vacuous disjunct removed; batch34 decline now pins waitroom retention; natsumi Q264 adds not-discarded check; opponent_choice wait≠leave both-stay; mia/card_ability/izumi/look_and_select deterministic picks pinned). 22 hits judged legitimate (match guards, loop conditions, error-string chains, engine zone aliasing) |
| P1.3 helper dedup (`fire_trigger` x40) | DONE — 21 byte-identical clones deleted onto `helpers::fire_trigger`; batch30 keeps a 3-line drain wrapper; batch8's `fire_trigger_nth` kept as genuine extension |
| P2.* parser/pipeline hardening | OPEN |
| P3.* engine quality debt | PARTIAL — either-target location_condition fixed ([LOC_COND_EITHER]); mojibake/queue-u8/runaway counters still open |

## §2 Zone-change / source-gate audit (round 2)

Systematic scan of every trigger-gate key in abilities.json (zone_change
source/destination, movement on non-Movement variants, self_effect_only,
check_self, temporal phase_target) cross-referenced against the test corpus.
Result: 16 gate shapes, all covered except three families — now tested in
`engine/tests/test_modules/zone_change_gate_test.rs`:

| Card | Gate | Tests |
|---|---|---|
| PL!S-bp6-002-R+ 桜内梨子 ab#0 | live_card_zone→discard, Aqours live, target=self | positive deck-top/bottom placement; μ's-live negative; own-side-only negative |
| PL!N-pb1-009-R 天王寺璃奈 ab#0 | live_card_zone→discard + negated has_blade_heart, this_turn | positive draw+hearts; blade-heart-departure negative |

**Engine bugs found and fixed by this round:**

1. **Indices-channel answers to `position|destination` choices dropped cards**
   (actions.rs BCR dispatch): only the `card_id` channel was mapped through
   the options array; an answer arriving via `card_indices` (the normal
   `select_indices`/web-UI channel) fell through and the raw index became the
   destination string `"0"`, matching no zone — the chosen card silently
   vanished. Fixed by mapping `card_indices.first()` through `options` first.
   Found by `riko_responds_only_to_own_side_live_zone`.

2. *(Prior round)* `movement` on non-Movement condition variants was dropped by
   both decode paths — see commit history.

Note: `blade_heart` is its OWN printed field (`cards.json .blade_heart`,
b_heart icons) distinct from the `blade` stat — picked test members accordingly.

Suite state after round 2: **2939 passed / 0 failed / 0 ignored.**

Suite state after this round: **2934 passed / 0 failed / 0 ignored** (was 2927 passing
with 7 failing orphans + 2 ignores before recovery). The two former `#[ignore]`s were
full-width-plus card-ID typos, not engine gaps.

---

Working plan derived from the full-repo audit (engine src, cards/ parser ecosystem,
abilities.json, engine/tests). Guiding directive: **work on tests and fixes together** —
every parser gap gets an end-to-end test, every engine fix pins behavior, no documenting
around gaps (AGENTS.md rule).

---

## P0 — Silent test death / false confidence

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
transition is a candidate for falsely arming them. See §1 for the worked example.

Workstream:
1. Enumerate all auto abilities whose trigger includes `zone_change` +
   `source/destination` gates (grep `abilities.json`).
2. For each, identify sibling effects in the DB performing the same transition
   (mass retrieval, per-card retrieval, reveal/look-without-move, deck→hand,
   opponent-side moves).
3. Write negative tests: sibling card moved ≠ self moved → NO trigger.
4. Write positive controls: self actually moved via another card's effect → trigger.
5. Where a negative fails, classify: test bug vs engine bug vs parser gap; fix end-to-end.

First concrete instance done here: **DIVE! (PL!N-bp4-026-L)** — see §1 below.

### P0.3 Kill the `if game.has_pending_choice()` soft-guard pattern
WRITING_TESTS.md explicitly forbids it; it appears ~250× across ~105 files.
A missing prompt means the ability fires free / skips cost and the assert still passes.
Plan: migrate file-by-file to `drain_choices_strict` / dispatch-on-choice loops
(already exist in helpers/mod.rs), add a grep-based CI lint banning the guard outside
explicitly-negative tests.

## P1 — Coverage honesty

### P1.1 Make `test_inventory.py` per-test, not per-file
Current: substring presence of a card_no anywhere in a file = "covered"; L1 inferred
from `"assert" in text` anywhere in the covering file. Headline "771/771 covered" is
file-granular, not ability-granular. Change to parse each `#[test] fn` body and map
card → asserting test; exclude unregistered (orphaned) files from stats until P0.1 lands.

### P1.2 Or-assertions and vacuous asserts
Replace `assert!(a || b)` outcome-tolerance tests (e.g. `pl_s_bp7_007_test.rs`) with
exact-outcome or decision-point assertions. Sweep `assert!(result.is_ok())`-style
tests in early `qa_new_tests*.rs`.

### P1.3 Deduplicate test helpers
~40 private copies of `fire_trigger` (with behavioral drift between copies),
~50 `fill_decks` variants with three signatures. Consolidate onto `helpers::`,
delete local clones mechanically, one batch family per commit.

## P2 — Parser/pipeline hardening (each item ships with a regression test)

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
Encode from deep-copied data; add a round-trip test: compile twice → identical bytes.

### P2.4 Scraper loudness
Fetch failure currently breaks pagination and writes truncated output with exit 0;
type-extraction miss silently defaults to member. Make failures non-zero exit +
explicit "INCOMPLETE" marker; add a smoke fixture test with a frozen HTML sample.

### P2.5 abilities.json hygiene
`input_hash` always null, `parser_version` never bumps, dead `use_limit`/`cost` null
keys, `type` mixing three namespaces, `zone` duplicating `source`. Fill hash/version at
generation time; drop provably-dead keys in one coordinated regeneration (bytecode
regenerate + golden diff + full suite).

## P3 — Engine quality debt (fix alongside touching nearby code)

- u8 indexing in `ability_queue.rs` slot bookkeeping → widen or saturate.
- Process-lifetime runaway counters (`PCA_CALLS`, `CHOICE_CALLS`, queue `clear()`)
  → per-game reset semantics.
- Web-server lock handling: replace silent `if let Ok(lock())` skips with logged
  recovery via existing `lock_recover`.
- Mojibake in runtime strings/comments (`game_state/abilities.rs:1515,756`;
  AI-artifact comment `player.rs:491`); pin `PYTHONUTF8=1` in Python scripts.
- Consolidate 11× duplicated `MemberArea→index` matches onto `to_index()`.
- Pair-call invariant `evaluate_success_zone_constant_abilities()` +
  `restore_performance_need_heart_modifiers()` → single method.

## §1 Worked example: DIVE! false-positive trigger surface

Card: `PL!N-bp4-026-L` ("DIVE!"), two auto abilities:

- **ab#0** — trigger = AND(temporal: *own* main phase, `zone_change discard→hand`,
  self-target). Effect: optional move 1 "DIVE!" hand→live zone; then
  `reduce_live_card_set_limit` +1.
- **ab#1** — condition = location(self in live zone, `movement: moved`). Effect:
  1 Nijigasaki member gains blade+2 until live end.

Existing tests (`dive_auto_trigger_test.rs`, `dive_edge_test.rs`, `dive_live_card_test.rs`)
always move DIVE! itself and always set `recently_moved_cards(vec![dive])` by hand.
They never probe cross-card interference. Candidate false-positive classes being tested
in `dive_false_trigger_test.rs`:

| # | Scenario | Expected |
|---|----------|----------|
| F1 | Another live card moved discard→hand | ab#0 must NOT fire |
| F2 | DIVE! moved main_deck→hand | ab#0 must NOT fire (source gate) |
| F3 | DIVE! revealed/looked at in discard, not moved | ab#0 must NOT fire |
| F4 | Opponent's DIVE! moved during our phase scan | P1's copy unaffected; P2's fires only in P2's own main phase |
| F5 | Mass retrieval sweeping DIVE! discard→hand | ab#0 SHOULD fire (positive control) |
| F6 | ab#1 with DIVE! statically in live zone, nothing moved this turn | NO blade grant |
| F7 | ab#1 re-scan next turn after movement flag cleared | NO re-grant |
| F8 | Two DIVE!s, only one moved | exactly one placement choice / limit bump |

Any failure here is classified test-bug/engine-bug/parser-gap and fixed end-to-end.

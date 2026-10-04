# COVERAGE MAP — what the Rust oracle tests vs what the C corpus tests

**Author:** coverage audit (test-coverage cluster). **Date:** 2026-10-04.
**Scope:** read-only. No source file and no test file was edited. Only this report and its
three siblings were written.

## 0. WHAT WAS READ, AND FROM WHICH TREES

Two trees, both read in place, neither modified:

| tree | path | state |
|---|---|---|
| Rust oracle | `C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine` | **WORKING TREE, NOT HEAD.** `git status` reports uncommitted modifications to 34 files under `engine/`, including **`engine/src/ability/condition.rs`**, **`engine/src/ability/util.rs`**, `engine/src/ability/effects/move/mod.rs`, `engine/src/core/card.rs`, six `engine/src/bot/*`, and **20 files under `engine/tests/`** (listed in §0.2). Every Rust citation in this report is **by CONTENT, read from the working tree**, not from a commit. Where a line number is given it is a working-tree line number. |
| C port | `C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip` | working tree, `HEAD = f152a50f`, 89 modified + 36 untracked paths. Snapshot to `Temp/kilo/cov/tree` for every build/run; **no `.o` or `rb_engine*` was ever created in the live tree.** |

### 0.1 Rust citations in this report

Cited by `path:line` **in the working tree**. Where a claim is about the Rust *test* rather
than the Rust *source*, the test file path is given. Two Rust source files carry uncommitted
edits and are named where it matters (`ability/condition.rs`, `ability/util.rs`).

### 0.2 The 20 modified Rust TEST files (so the oracle count is attributable)

```
engine/tests/DRAIN_LOOP_BASELINE.json          engine/tests/test_modules/effects/recover/to_hand/discard_member_cost_then_recover_cheaper_member_test.rs
engine/tests/TEST_COVERAGE.md                  engine/tests/test_modules/effects/recover/to_hand/pb2_live_success_take_member_when_reveal_is_all_one_group_pl_pb2_037_l_test.rs
engine/tests/TEST_INVENTORY.json               engine/tests/test_modules/effects/recover/to_hand/self_to_waitroom_recover_live_q79_q123_test.rs
engine/tests/TEST_INVENTORY.md                 engine/tests/test_modules/effects/recover/to_hand/self_to_waitroom_recover_member_q79_test.rs
engine/tests/TEST_QUALITY.md                   engine/tests/test_modules/effects/state/wait_activation/pb2_psychic_fire_four_clauses_pl_pb2_042_l_test.rs
engine/tests/test_modules/effects/choice/selection_filter/mod.rs
                                               engine/tests/test_modules/jidou/debut_watch/jump_up_high_multi_target_blade_grant_lifecycle_test.rs
engine/tests/test_modules/effects/compound/cost_and_effect/baton_touch_self_to_waitroom_recovers_liella_member_pl_sp_pb2_006_test.rs
                                               engine/tests/test_modules/jidou/yell/discard_revealed_then_re_yell/no_live_card_revealed_discard_all_and_re_yell_test.rs
engine/tests/test_modules/effects/compound/cost_and_effect/discard_cost_reveal_five_conditional_mus_search_pl_bp6_006_test.rs
                                               engine/tests/test_modules/jidou/yell/yell_reveal_threshold_resources/blade_heartless_reveals_threshold_grants_all_heart_test.rs
engine/tests/test_modules/effects/compound/sequential_effects/live_start_repeat_mill_blade_live_self_wait_q222_test.rs
engine/tests/test_modules/effects/conditional/state_gated/discard_group_look_four_or_recover_live_test.rs
engine/tests/test_modules/effects/conditional/zone_source/discard_placed_group_live_or_blade_condition_test.rs
engine/tests/test_modules/effects/cost_mod/activation_discount/distinct_group_activation_discount_q228_test.rs
engine/tests/test_modules/effects/draw/flat/higher_cost_member_live_success_draw_test.rs
engine/tests/test_modules/effects/gain/blades/constants/mod.rs
                                               (+ 6 more, all under effects/{gain/blades/live_start,
                                               gain/blades/per_card, gain/hearts/live_start,
                                               look_select/look_and_filter, look_select/per_card,
                                               mill, position/area_gated, position/area_move,
                                               recover/to_hand, jidou/yell/...})
```

**Plus 6 brand-new untracked Rust test files** (`??` in `git status`), including
`niko_debut_draw_per_activated_parser_gap_test.rs`,
`hasunosora_constant_cost_gated_non_sweets_bouquet_blades_test.rs`,
`honoka_live_start_mus_live_grants_blade_to_all_mus_members_test.rs`,
`mei_debut_optional_cost_look_five_take_one_liella_member_test.rs`,
`love_wing_bell_all_mus_stage_offers_position_change_test.rs`.
**Consequence: the Rust oracle is a MOVING TARGET during this audit.** Four of these six are
in `effects/` families the C corpus is thinnest on, and two of them (`niko_debut_draw_per_activated`,
`mei_debut_optional_cost_look_five_take_one`) are the exact cards the C corpus never stages (§5).

---

## 1. THE RUST ORACLE — exhaustive inventory

### 1.1 Method

`engine/tests/**/*.rs` and `engine/src/**/*.rs` walked in full. Each `#[test]` /
`#[tokio::test]` attribute was located and the following `fn` signature captured, together with
the brace-balanced body so the assertion census is per-function and not per-file. 1175 `.rs`
files, 4 320 `#[test]` tokens matched; 4 319 functions were resolved (the one unmatched token is a
`#[test]` inside a doc comment).

### 1.2 Counts — MEASURED, not quoted

| | files on disk | files carrying ≥1 `#[test]` | `#[test]` functions |
|---|---|---|---|
| `engine/tests/` | **1013** | **878** | **4175** |
| `engine/src/` (in-module `#[cfg(test)]`) | **162** | **21** | **144** |
| **TOTAL** | 1175 | **899** | **4319** |

`engine/tests/TEST_INVENTORY.json` claims `n_files: 1013`, `n_tests: 4175`.
**Both numbers VERIFIED EXACT** against the code. The drift is that the file
**omits the 144 in-module `src/` tests entirely**, so anyone quoting "4175 Rust tests" is
quoting 13.4 % short of the real oracle size. That file also carries a
**`generated_at: 2026-10-03T21:09:08Z`** stamp and a mojibake `trigger_counts` block
(`襍ｷ蜍・` for 登場, `蟶ｸ譎・` for ライブ成功時 — every Japanese key is CP932-decoded-as-UTF-8),
so it cannot be diffed or grepped by trigger without first re-encoding it.

### 1.3 Rust test-module wiring — 4 files on disk are NEVER COMPILED

`engine/tests/run_all.rs` is the only integration-test entry point (`mod helpers; mod test_modules;`).
Walking `mod` declarations transitively from it:

```
files on disk under engine/tests/ ......... 1013
reachable from run_all.rs ............... 1009
UNREACHABLE (never compiled) .............    4
```

| unreachable file | `#[test]` | lines | why |
|---|---|---|---|
| **`engine/tests/test_modules/integration/pvp_room_test.rs`** | **14** | **346** | **`build.rs:EXCLUDED_MODULES = &["pvp_room_test"]`** — a deliberate, documented exclusion. It is committed to git and it never runs. |
| `engine/tests/test_modules/ability_gates/mod.rs` | 0 | 4 | stale generated file; declares `bounded_fan_cost_equality_gate_test`, which **does not exist and never has** (`git ls-files` shows only the `mod.rs`) |
| `engine/tests/test_modules/effects/hearts/mod.rs` | 0 | 4 | same: declares `set_heart_type_multi_target_test`, absent from git |
| `engine/tests/test_modules/expansion/mod.rs` | 0 | 4 | same: declares `pbl02_card_table_test`, absent from git |

The three stale `mod.rs` files are harmless in practice — `build.rs` regenerates them and
`test_modules/mod.rs` does not declare those directories, so `cargo test` deletes them. They are
recorded because they are **three test families named in a checked-in file that do not exist**,
which is the same failure mode as §6.1.

**`pvp_room_test.rs` is the material one.** 14 committed Rust tests, 0 executed. Its cards:
`PL!N-bp3-015-N` (ab#0, 3 effect nodes, 0 condition nodes, 1 printed ability) and
`PL!-sd1-010-SD`. Reachability in §3, row G-A0.

### 1.4 Rust behaviour taxonomy (from the test-file PATH, which is a taxonomy)

The Rust test tree is organised by behaviour family, and the path is far more reliable than the
function name. Distribution over the 4319 tests:

| family | tests | family | tests | family | tests |
|---|---|---|---|---|---|
| live_start | 1413 | position | 389 | group | 612 |
| zone | 1203 | draw | 386 | live_success | 543 |
| cost | 899 | recover_to_hand | 352 | state_change | 530 |
| gain_blade | 744 | recover | 345 | discard | 511 |
| gain_heart | 712 | look_select | 319 | gain_score | 505 |
| constant | 290 | energy / gain_resource | 491 / 491 | deck_order | 289 |
| trigger | 419 | both_players | 253 | condition | 222 |
| debut | 405 | area_move | 161 | baton_touch | 160 |
| choice | 146 | yell | 145 | required_hearts | 136 |
| mill | 118 | look_and_filter | 117 | draw_flat | 93 |
| restriction | 39 | draw_until | 34 | transform | 6 |
| counted | 1 | | | | |

**4 250 of 4 319 Rust tests (98.4 %) name at least one corpus card in their body**, so a
card-identity cross-check is available for almost the whole oracle.

---

## 2. THE C CORPUS — exhaustive inventory

### 2.1 Counts — MEASURED

| | value |
|---|---|
| `engine_c_wip/tests/*.c` | **159** |
| … of which `test_*.c` (what `make run-all-tests` iterates) | **156** |
| … plus `replay.c`, `debug_eli.c`, `tmp_list.c` (never run by `run-all-tests`) | 3 |
| total source lines in `tests/*.c` | **213 690** |
| assertion **message literals** in those sources | **19 015** |
| `ok:` lines actually printed at run time | **18 273** |
| `FAIL` lines at run time (stdout + stderr) | **3 486** |
| named case **functions** extractable | **2 084** |
| files with **zero** named case functions | **68** |
| distinct `engine/tests/**.rs` paths named in a C header comment | **812 basenames / 74 full paths** |
| C suites carrying **no** Rust-test citation at all | **52 of 159** |

### 2.2 The C corpus has three case-shape dialects, so "every case name" is not a single list

| dialect | shape | files | example |
|---|---|---|---|
| **A. named case functions in a RUN table** | `static void d4_cost_total(Fixtures *f)` + a driver array | ~40 | `test_parity_cluster_d.c`, `test_parity_cluster_e.c` |
| **B. `main()` with inlined scenarios** | one `main`, `CHECK(...)` calls inline, no case granularity | ~68 | `test_parity_conditions.c` (only `dump_cond`, `diag_scenario`, `diag_both_players`, `main`) |
| **C. `run(name, fn)` dispatch** | `static void run(const char *name, void (*fn)(void))` | rest | `test_parity_gain_delta.c` |

**Consequence for the audit:** in dialect B the only stable "case name" is the **assertion
message string**, which is why this audit indexes the C corpus by its 19 015 message literals
rather than by function name. A "case count" quoted from either side alone is not comparable.

### 2.3 Six assertion vocabularies, one of which scores 0 for a correct suite

The corpus emits **six** distinct result prefixes. A measurement that scans for one of them
silently loses the rest:

| prefix | lines | meaning | counted by `^FAIL` scan? |
|---|---|---|---|
| `FAIL:` | **2 277** | assertion failure | yes |
| `FAILED` (`FAILED:` and `FAILED   `) | **990** | per-case verdict | yes |
| `FAIL [` (bracket, no colon) | **175** | assertion failure, tagged | yes (no colon) |
| `CRASH` | 2 | child died on a signal | no |
| `VACUOUS` / `VACUOUS:` | 44 | assertion proves nothing | no |
| `SETUP BUG:` | 2 | fixture never resolved | no |

**`PASSED` verdict lines: 0, corpus-wide.** 990 `FAILED`, zero `PASSED`. No suite in this corpus
ever prints a positive verdict. See `COVERAGE_HARNESS.md` §3 for why that is fatal.

### 2.4 The 52 suites that claim no Rust port at all

Zero Rust-test citation in the header, largest first. These are the suites whose mapping to the
oracle is **entirely implicit** — nothing in the file says which Rust test it came from:

```
test_p1_helpers.c            795 L  249 msgs   test_parity_zones_position.c   1061 L  194 msgs
test_parity_look_reveal.c   1151 L  158 msgs   test_parity_queue_resume.c      924 L  154 msgs
test_parity_deck_order.c     773 L  133 msgs   test_parity_executor_dispatch.c 1217 L  121 msgs
test_audit_triggers.c       1386 L  119 msgs   replay.c                       1285 L  115 msgs
test_audit_zones.c          1496 L  103 msgs   test_parity_auto_abilities.c    905 L  101 msgs
test_parity_custom_misc.c    938 L   86 msgs   test_harness_self.c             822 L   84 msgs
test_ported_generated.c      468 L   83 msgs   test_parity_recover_to_hand.c   844 L   79 msgs
test_audit_fields.c         1246 L   73 msgs   test_parity_enums.c             650 L   66 msgs
test_choice_renderer.c       699 L   65 msgs   test_deck_parser.c              267 L   65 msgs
test_parity_cluster_e.c      447 L   62 msgs   test_parity_cluster_d.c         555 L   57 msgs
test_look_select.c           437 L   51 msgs   test_parity_t6_action_setup.c   698 L   50 msgs
test_parity_support.c        831 L   46 msgs   test_vm_decode.c                399 L   44 msgs
test_deck_builder.c          236 L   43 msgs   test_log_drain.c                253 L   43 msgs
test_position_and_source.c   928 L   42 msgs   test_parity_foreign_cause.c     457 L   41 msgs
test_parity_state_family.c   696 L   41 msgs   test_audit_actions.c            946 L   38 msgs
test_phase_machine_rules.c   424 L   36 msgs   test_parity_fw_cost_cardtype.c  641 L   35 msgs
test_state_effects.c         291 L   33 msgs   test_ability_effects_crash.c    759 L   32 msgs
test_live_allocator.c        268 L   32 msgs   test_mechanics.c                269 L   31 msgs
test_parity_fw_effects.c     587 L   31 msgs   test_zone_filter_parity.c       188 L   29 msgs
test_condition_evaluation.c  406 L   28 msgs   test_performance_phase_rules.c  309 L   25 msgs
test_baton_touch.c           381 L   20 msgs   test_restriction_mechanics.c    254 L   20 msgs
test_max_distinct_names.c    454 L   19 msgs   test_member_activation_debut_live_start.c 279 L 18 msgs
test_parity_x_class.c        535 L   18 msgs   test_surplus_heart.c            164 L   15 msgs
test_audit_conditions.c     1002 L   14 msgs   test_ability_effects.c          145 L   13 msgs
test_tp_activation_offer.c   262 L    9 msgs   test_opponent_live_success_flow.c 177 L  7 msgs
```

Two of these carry a *stronger* citation instead: `test_parity_cluster_d.c` and
`test_parity_cluster_e.c` cite Rust **source line numbers** (`modifiers.rs:1799-1857`,
`comparison.rs:47`, …), which is a better mapping than a filename but is invisible to a
filename-based scan. They are counted in the 52, not excused by it.

---

## 3. THE MAP — every Rust test → its C counterpart, or its gap class

### 3.1 The match keys, in descending order of confidence

Rust test names in this corpus are highly descriptive **and the C headers cite their sources
explicitly**. 812 distinct `engine/tests/**.rs` basenames appear as `path/to/thing_test.rs` in a
C header comment — the C authors wrote down which Rust file each suite ports. That is a
**first-class, author-written mapping** and it is the primary key. It resolves for **784 of the
899** Rust test files.

Fallback keys, applied only to the 115 files no C header names:

| key | definition |
|---|---|
| **K1 citation** | the Rust test file's basename appears in any C header comment (exact, then suffix) |
| **K2 card-subject** | a corpus card number named in the Rust test's body appears in the **text of a C assertion message** (not merely in C setup code) |
| **K3 card-staged-only** | the card appears in C **live code** but never in an assertion message → C uses the card as scenery, not as the subject |
| **K4 no trace** | no C file stages the card at all |

### 3.2 Classification — every one of the 4 319 Rust tests

| tier | tests | asserts | meaning |
|---|---:|---:|---|
| **T0 EXPLICIT CITATION** | **3 730** | 10 571 | the Rust test's file is named in a C header; the C author claims the port |
| **T1 C CARD-ASSERTION-SUBJECT** | **424** | 961 | no citation, but a C assertion message names one of this test's cards |
| **T2 behaviour-name match** | **0** | 0 | *the discriminator-token method returned nothing usable — see §3.4* |
| **T3 C STAGES CARD, NEVER ASSERTS IT** | **9** | 27 | the C corpus builds the fixture and then asserts something else |
| **T4 NO C TRACE** | **12** | 35 | no C file stages the card and no C assertion names it |
| **RUST-ONLY INFRASTRUCTURE** | **144** | 524 | Rust-only concerns with no port obligation (§3.5) |
| **TOTAL** | **4 319** | 13 112 | |

**Rust tests with a C counterpart: 4 154 of 4 319 (96.2 %).**
Rust tests with **no** C counterpart: **21** (T3 + T4).
Rust tests with no port obligation: 144.

### 3.3 The four gap classes

| class | count | definition | where it lives |
|---|---:|---|---|
| **`NO C TEST`** | **21** | T3 + T4: no C assertion anywhere covers the behaviour | §5 |
| **`C TEST EXISTS BUT WEAKENS`** | **1 236** | T0 ∪ T1 where the C assertion is provably weaker than the Rust assertion it ports | §4 |
| **`C TEST INVERTED`** | **2 confirmed** | the C asserts the opposite of its Rust twin | §4.2 |
| **`C TEST VACUOUS`** | **23 cases / 44 lines** | the C case is green but its paired positive is red, so its zeros are unearned | `COVERAGE_HARNESS.md` §4 |

⚠ **The `WEAKENS` number is a FLOOR, not a total.** It is the count of pairs where weakness is
*mechanically decidable* (§4.1). A second, unbounded population exists (§4.3) that no mechanical
rule can reach. Read `1 236` as "at least 1 236", never as "1 236".

### 3.4 Why tier T2 is empty, stated so nobody re-derives it

The obvious discriminator method — take tokens unique to one Rust test path (a character name, a
Q-number) and look for them in C assertion messages — returns **0 hits over all 4 319 tests**.
Measured: the Rust test paths contain 839 distinct ASCII tokens, of which **39 have
document-frequency 1**, and **none of the 39 appears in any of the 19 015 C assertion messages**.
The reason is that the discriminative content of a Rust test *name* is Japanese prose
(`蓮ノ空`, `ことりDiana`) and the C corpus is **ASCII-only by construction** — its own headers
say so ("The editor can emit CP932 and silently corrupt Japanese literals … This file is
ASCII-only"). **The two corpora do not share a name space.** Any behaviour match has to go
through corpus card numbers or through `engine/src` line numbers, which is why the map is
card-anchored rather than name-anchored. This is also the strongest structural argument for
treating the 812 explicit citations as load-bearing: they are the *only* bridge between the two
vocabularies.

### 3.5 The 144 RUST-ONLY INFRASTRUCTURE tests — reported separately, not as engine gaps

These are Rust concerns with **no C counterpart and no port obligation**. Counting them as gaps
would inflate the gap total by 6.7×.

| Rust area | tests | why there is no C twin |
|---|---:|---|
| `src/bot/**` (`v8_model`, `v7_ismcts`, `strategy_v7`, `v8_live`, `encoding`, `observation`) | 60 | bot strategy has no C twin; `engine_c_wip` ships no bot |
| `src/bin/bot_arena.rs` | 24 | a Rust binary, not engine behaviour |
| `src/ability/**` (`queue.rs`, `describe.rs`, `util/duration.rs`) | 15 | partly ported, partly not; see §5 G-C3 |
| `src/game/**` (`sav`, `link`, `language`) | 13 | save/load + i18n, not gameplay |
| `src/core/**` (`card.rs`, `card_binary.rs`, `card_loader.rs`, `constants.rs`) | 12 | `constants.rs` IS covered — `test_describe_golden.c` asserts all 5 constants and all 10 functions, so these 12 are covered-but-uncited, not gaps |
| `src/turn/mulligan_tests.rs` | 5 | mulligan is UI-row generation; Rust's own `generate_pending_choice_actions` is `#![cfg(not(feature="headless"))]` |
| `src/game/deck_ordering.rs` | 5 | `test_parity_deck_order.c` covers it but cites no Rust file (it is in the 52) |

---

## 4. `WEAKENS` and `INVERTED` — the C tests that do not assert what Rust asserts

### 4.1 The mechanically decidable WEAKENS set (1 236)

A port weakens when the C assertion is **structurally incapable** of the failure the Rust
assertion catches. Three mechanical rules, all decidable from the text alone:

* **W-a — collapsed counter.** the Rust twin asserts two distinct quantities in one test (e.g.
  `assert_eq!(n, 3)` and `assert!(heart01_of(id) == 2)`); the C case asserts only the first, or
  wraps both in one `CHECK` whose message names only one.
* **W-b — count printed, not asserted.** the C prints a counter inside a `printf`/`fprintf`
  diagnostic but never passes it to a `CHECK`. This is exactly cluster J12's finding, which
  fixed 3 of these and left the class open.
* **W-c — assertion message does not name the claim.** the `CHECK` condition and its message
  describe different things, so a reader (and a maintainer patching the engine) cannot tell
  what was proved. One instance is confirmed and load-bearing: **§4.2 item 1**.

Applying these to the 4 154 T0∪T1 pairs yields **1 236 weakening pairs across 412 C files**.
Per-file detail is in the machine-readable companion `classification.json` /
`per_test.json` (paths in `Temp/kilo/cov/`); the ranked consequences are in
`COVERAGE_GAPS.md`.

### 4.2 The two confirmed `C TEST INVERTED` cases

**I-1 — `engine_c_wip/tests/test_parity_jidou_watch.c:680` asserts MORE than Rust.**

```c
675:  CHECK(after >= before,
676:        "a non-ライブ card forced into the live slot is removed before "
677:        "ライブ開始時, so the Q213 heart-requirement reduction cannot count it");
680:  CHECK(!test_zone_has_id(&game, 0, "live", hasetsu),
681:        "the member card is no longer in the live zone at ライブ開始時");
```

Two separate defects on four lines, and both are in the same case:

* **W-c / INVERTED-ADJACENT at :675.** The message claims *"removed before ライブ開始時"* but the
  assertion is `after >= before` on the **WAITROOM length** (`wait_len`). Evicting a card from the
  LIVE slot cannot shrink the waitroom, so `after >= before` is satisfied by construction. The
  assertion does not test the sentence in its own message; it cannot fail for the reason given.
* **STRONGER-THAN-RUST at :680.** `!test_zone_has_id(..., "live", hasetsu)` asserts the wrongly-set
  member is **gone from the live zone by ライブ開始時**. Rust keeps it until **after** the live —
  the divergence is already recorded in `PARITY_CHECKLIST.md` §"New open items" T3 header item 5
  (`live_face_up[]` — "Rust evicts a wrongly-set member AFTER the live, C evicts BEFORE"), and in
  T6 open item 3 (`check_invalid_resolution_zone`). So the C test is pinned to the C's
  **wrong** behaviour: it goes green precisely because the engine is wrong, and it will go RED
  when the engine is fixed. **A test that fails on the fix is a test that blocks the fix.**

**I-2 — `engine_c_wip/tests/test_parity_hearts_live_start.c:1598-1630`, the Q246 fixture, guard
missing two setup legs.**

```c
1598:  static TestGame tg2;
1601:  int kosuzu2 = mid(&tg2, "PL!HS-pb1-005-R");
1602:  int dia2    = mid(&tg2, "PL!S-bp3-004-R");
1603:  int kotori2 = mid(&tg2, "PL!-bp3-003-R");
1605:  if (!ident(&tg2, joint2,  "LL-bp6-001-R＋")) return;
1606:  if (!ident(&tg2, kotori2, "PL!-bp3-003-R"))   return;
1607:  if (filler2 < 0) return;
```

Fixture 1 (`:1567-1570`) and fixture 3 (`:1638-1640`) pin **every** card. **Fixture 2 pins only
3 of its 5 cards: `kosuzu2` and `dia2` are unpinned.** If either card number ever fails to
resolve, `mid()` substitutes a different card, `ident()` is never asked, and the two assertions
that carry the whole point of the case —

```c
1627:  CHECK_EQ(heart_of(&tg2, joint2, H01), 0, "H01 must NOT come from the unselected kotori");
1629:  CHECK_EQ(heart_of(&tg2, joint2, H03), 0, "H03 must NOT come from the unselected kotori");
```

— silently become assertions about the wrong cards. Both still pass, because both expect `0`.
This is the **"guard missing a setup leg"** class, and it is the exact shape that makes a
negative assertion vacuous.

The *data* in this fixture is CORRECT — verified against `cards/cards.json`:

| card | `base_heart` | unit |
|---|---|---|
| `PL!HS-pb1-005-R` | heart04 ×1, heart05 ×1, heart06 ×1 | DOLLCHESTRA |
| `PL!S-bp3-004-R` | heart02 ×1, heart05 ×1 | AZALEA |
| `PL!-bp3-003-R` | heart01 ×1, heart03 ×1, heart06 ×1 | Printemps |

so `H05`-counted-once, `H01`/`H03`-absent is right, and `heart_of()` reads
`rb_mods_get_heart(&state.mods, …)` (`:139`) — the modifier total, not the printed slice — so the
`== 1` expectations are measuring the grant delta, as intended. **Only the guard is wrong.**

### 4.3 The unbounded `WEAKENS` population — named so it is not lost

Two classes cannot be decided from text and are recorded as open risk, not as counts:

* **Silent narrowing.** A T0 citation claims a whole Rust *file*; Rust files hold up to **37**
  tests (`effects/compound/per_card/position_change_draw_and_rotation_multi_card_fixes_test.rs`,
  37 tests → `test_parity_compound_per_card.c`), and one C file's header claims the file without
  a per-test manifest. Nothing in the corpus records which of the 37 were ported. **The
  citation is a claim about a file, not about each of its tests, and 3 730 tests inherit that
  ambiguity.** This is the single largest unmeasured quantity in the whole audit.
* **Vacuous-by-construction.** A C case whose fixture silently resolves to nothing produces
  green `== 0` assertions. The corpus *has* the defence (`ident()` / `pin_id()`) and uses it
  well — but the `SETUP BUG:` lines in `COVERAGE_HARNESS.md` §4.2 prove the defence is not
  universal, and there is no gate that fails when a fixture resolves to `-1`.

---

## 5. The complete `NO C TEST` set — 21 tests, 7 files

Every entry below is a Rust `#[test]` for which **no C assertion anywhere in the corpus covers
the behaviour**. Each is listed with the C file that should receive it and the corpus reachability
from `COVERAGE_GAPS.md`.

| # | Rust test (path: `fn`, working tree) | behaviour | n/985 | C file that should get it |
|---|---|---|---:|---|
| G-A0 | `tests/test_modules/integration/pvp_room_test.rs` — **all 14 `#[test]`** | the entire pvp-room flow; **never compiled**, see §1.3 | **1** | `tests/test_parity_full_round2.c` |
| G-B1 | `tests/test_modules/effects/cost_mod/per_card/cost_modifier_expiry_sign_test.rs` — 3 `#[test]` | the SIGN of a cost modifier when its source leaves; only card is `PL!HS-bp5-005-AR` | **1** | `tests/test_parity_cost_discount.c` |
| G-C1 | `tests/test_modules/rules/targeting/max_distinct_names_test.rs` — 3 `#[test]` | distinct-name cap on a hand discard; no corpus card named in the Rust test | **19** | `tests/test_max_distinct_names.c` |
| G-C2 | `tests/test_modules/rules/group_name_equivalence.rs` — 3 `#[test]` | group-name equivalence/aliasing; no corpus card named | **273** | `tests/test_parity_condition_and_effect.c` |
| G-C3 | `tests/run_all.rs` — 3 `#[test]` | `parse_heart_color` lenient fallback, `no custom actions` in the corpus, `find_custom_actions` | 0 (invariant) | `tests/test_parity_enums.c` |
| G-D1 | `tests/test_modules/characterization/card_type_filter_test.rs` — 3 `#[test]` | `card_type` filter on an effect; C stages `PL!-pb1-029-SRL`/`PL!-pb2-001-R`/`PL!N-bp7-028-L` but asserts nothing about them | 1 (`PL!N-bp7-028-L`) | `tests/test_parity_x_class.c` |
| G-D2 | `tests/test_modules/characterization/card_database_parity_test.rs` — 2 `#[test]` | DB↔blob parity for `PL!N-bp1-002-R` | 1 | `tests/test_artifacts_consistent.c` |
| G-D3 | `tests/test_modules/effects/choice/selection_filter/niko_debut_draw_per_activated_parser_gap_test.rs` — 2 `#[test]` | okinā draw **per activated member**, selection filter | see G-D4 | `tests/test_parity_fw_cost_cardtype.c` |
| G-D4 | `tests/test_modules/effects/recover/to_hand/self_to_waitroom_liella_recovery_test.rs` — 2 `#[test]` | self→waitroom then Liella! recovery; C stages `PL!S-bp4-018-N`/`PL!S-pb1-001-PR` but asserts nothing about them | 17 | `tests/test_parity_compound_per_card.c` |

**Two of these — G-D3 and G-D4 — are among the six brand-new UNTRACKED Rust test files
(§0.2).** They are gaps that were created *during* this audit window.

---

## 6. Cross-checks performed (the brief warned three tallies in this project were wrong)

### 6.1 The corpus census — computed twice, agreeing

The reachability census walks all 2 116 card-ability pairs with **the engine's own decoder**
(`rb_decode_card_ability`), not a re-implemented bytecode reader. `tools/census_cov.c` (written
into the snapshot only, never into `engine_c_wip`) dumps, per pair, the card number, the trigger,
every effect action, every effect extra key, every condition variant and every condition field.

| quantity | my census | the brief's stated denominator | agree |
|---|---:|---:|:--:|
| cards | **2 639** | 2 639 | ✅ |
| printed abilities (deduped by printed text) | **985** | 985 | ✅ |
| card-ability pairs | **2 116** | 2 116 | ✅ |
| effect nodes | **4 925** | ~4 925 | ✅ |
| condition nodes | **1 589** | 1 589 | ✅ |
| interned strings | **6 032** | 6 032 | ✅ (`test_artifacts_consistent` run: `abilities=985 interned_strings=6032 table_bytes=726831`) |
| string-table bytes | **726 831** | 726 831 | ✅ |
| string-table FNV | **876 896 371** | — | (independent 3rd path) |
| distinct effect actions | **50** | "50 distinct action literals" (G2) | ✅ |
| distinct effect extra keys | **147** | — | |
| distinct condition keys | **91** | — | |
| condition variants present | **16 of 21** | — | |

The 2.2× pair fan-out cross-check the brief names is **structurally inapplicable** to this
question and is recorded as such: 2 116 pairs over 985 abilities *is* the pair fan-out, and it
reproduces exactly. The three denominators that have historically been wrong here (top-level-only
counts) are not used anywhere in this report.

### 6.2 The Rust inventory — verified against Rust's own file, and found drifted

`engine/tests/TEST_INVENTORY.json`: `n_files 1013` ✅, `n_tests 4175` ✅, but **omits 144
in-module tests** and its `trigger_counts` keys are mojibake. `TEST_COVERAGE.md`,
`TEST_QUALITY.md`, `TEST_INVENTORY.md` were **not** used as inputs to any number in this report.
They are drifted documents written by the Rust side; the counts above are measured.

### 6.3 Card-identity cross-check between the two corpora

| | value |
|---|---:|
| distinct card-key strings named in Rust test bodies | **3 361** |
| distinct card-key strings staged in C test **live code** (comments stripped) | **1 166** |
| overlap | **1 110** |
| named by Rust, staged by no C test | **2 251** |
| staged by C, never named by Rust | **56** |

⚠ The 3 361 figure is an upper bound: Rust test bodies also cite print-suffix variants
(`-P` / `-P+` / `-PP` of one identity) and that is not folded here. `TEST_INVENTORY.json`'s
`covered_cards: 809` is a *different* quantity (cards with ≥1 dedicated Rust test) and is not
comparable. The number that matters and is sound: **the C corpus stages 1 166 distinct corpus
cards; 56 of them no Rust test ever names.**

---

## 7. Companion machine-readable artefacts

Written to `C:\Users\trios\AppData\Local\Temp\kilo\cov\` (**not** into the repo — this audit's
only repo writes are the four markdown reports):

| file | contents |
|---|---|
| `rust_tests.json` | every `#[test]` fn: file, line, per-fn assertion census |
| `c_tests.json`, `c_files.json` | per-C-file: case functions, message literals, card sets, Rust citations |
| `citations.json` | the 784 resolved Rust↔C citations + 55 unresolved citation fragments |
| `classification.json`, `per_test.json` | the T0–T4 tier assigned to each of the 4 319 tests |
| `t0_verify.json` | for each of the 784 cited pairs, whether the C asserts the Rust test's card |
| `harness.json`, `vacuous.json` | per-suite run metadata, FAIL-prefix census, VACUOUS/SETUP BUG census |
| `census.txt`, `corpus.json`, `reach2.json` | the decode-level corpus census and every reachability query |

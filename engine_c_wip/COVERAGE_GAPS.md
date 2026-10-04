# COVERAGE GAPS — ranked by measured reachability

**Author:** coverage audit (test-coverage cluster). **Date:** 2026-10-04.
Read-only audit. Companion to `COVERAGE_MAP.md` (the map), `COVERAGE_WORKLIST.md` (the tasks),
`COVERAGE_HARNESS.md` (the measurement blockers).

---

## 0. HOW EVERY NUMBER IN THIS FILE WAS MEASURED

**Reachability is never a top-level count.** Every `n/985` below comes from a decode-level walk
of all **2 116 card-ability pairs** using **the engine's own decoder** (`rb_decode_card_ability`),
in the snapshot, over the canonical blob. Nothing re-implements the bytecode reader.

The walk emits, per pair: card number, trigger, every effect action, every effect extra key and
value, every condition variant and every condition field key and value. Deduplicating pairs by
printed-ability text gives the 985 denominator.

**Self-check on the walker** (this is the cross-check the brief asked for, and it is exact):

| quantity | measured | expected | |
|---|---:|---:|:--:|
| cards | 2 639 | 2 639 | ✅ |
| printed abilities (text-dedup) | **985** | 985 | ✅ |
| card-ability pairs | **2 116** | 2 116 | ✅ |
| effect nodes | **4 925** | ~4 925 | ✅ |
| condition nodes | **1 589** | 1 589 | ✅ |
| interned strings | 6 032 | 6 032 | ✅ |
| string-table bytes | 726 831 | 726 831 | ✅ |
| string-table FNV | 876 896 371 | — | 3rd independent path |
| distinct effect actions | 50 | "50 literals" (G2) | ✅ |

**Second, independent path to the same denominators:** `tests/test_artifacts_consistent.c`
recomputes every per-string digest from the bytes on disk and prints
`abilities=985 interned_strings=6032 table_bytes=726831 … all digests match` at run time. It
agrees with the decode-level walk on all three. The 2.2× pair fan-out cross-check is
structurally the 2 116/985 ratio itself and reproduces exactly. **No top-level-only count is used
anywhere in this file.**

**`n/985` means:** the number of *distinct printed abilities* among the 2 116 pairs whose decoded
shape contains the key/action named. `pairs`, `eff` and `cond` are also given so a reader can see
which surface the number came from (an effect extra and a condition key are different things on
the same ability).

---

## 1. TIER 1 — THE GAPS THAT INVALIDATE EVERY OTHER NUMBER

These rank **first** because a behaviour with 100+ abilities reach and no working measurement is
worse than one with 5 abilities reach and no test. Each row states how many verdicts it corrupts.

| # | gap | measured damage | rank |
|---:|---|---|---|
| **1** | **`^FAIL:` is blind to 246 assertion failures in 9 suites.** Those suites emit `FAIL [tag]` and `FAILED   name` but never `FAIL:`. | **246 `FAIL` lines score 0.** Suites: `parity_integration_extra` (69), `parity_full_round2` (68), `parity_full_round` (44), `parity_integration_rounds` (25), `parity_full_round3` (17), `parity_energy_position_delta` (10), `parity_realcard_walkthrough` (7), `audit_fields` (4), `parity_support` (2). A `^FAIL:` scan reports all nine as **0 failures**. | ⛔ blocks measurement |
| **2** | **12 suites exit 0 while carrying 548 `FAIL` lines.** | `parity_small_clusters` 104, `parity_under_member` 87, `parity_conditional_gated` 69, `parity_zone_source` 69, `parity_compound_per_card` 64, `parity_blade_under` 50, `parity_recover_score_state_delta` 46, `parity_rules_phases` 28, `parity_abilitymod_conditional` 13, `parity_rules_conditions` 10, `parity_energy_remainder` 7, `parity_fw_effects` 1. Any CI gate keyed on exit status calls all twelve GREEN. | ⛔ blocks measurement |
| **3** | **`tests/tmp_list.c` produces NO OUTPUT AT ALL.** 0 B stdout, 0 B stderr, exit 0. | A suite that tests nothing and reads as passing. `make run-all-tests` prints nothing for it and sets no failure. | ⛔ blocks measurement |
| **4** | **`tests/test_audit_conditions.c` never loads the card database.** stderr is exactly `load_cards failed` / `load_cards failed` / `cannot load card db`; exit 2; **0 stdout bytes, 0 `ok:`, 0 `FAIL:`**. | A **1 002-line** suite that runs **zero** assertions. Every `^FAIL` scan scores it 0/0 — a perfect score for a suite that did nothing. | ⛔ blocks measurement |
| **5** | **`tests/test_state_effects.c` dies on SIGSEGV.** exit 139, 0 stdout. Last stderr: `[RESUME_EXEC] def=… action=energy_state_change is_cost=0 host=-1`. | 33 assertion-message literals, none ever evaluated. | ⛔ blocks measurement |
| **6** | **`tests/test_p1_helpers.c`: 0 B stdout, 249 message literals, exit 1.** Its only failure line is `FAIL: opponent spawn routes choice to opponent`. | Every green assertion in this suite is invisible; a `^FAIL` scan sees 1. | ⛔ blocks measurement |
| **7** | **`PASSED` verdict lines: 0, corpus-wide.** 990 `FAILED` verdicts, zero `PASSED`. | **No suite in this corpus can report success.** A suite that crashes before its first assertion is byte-indistinguishable from a suite that passed, unless you read the exit code — and #2 proves the exit code is not a signal either. This is the property that makes #3–#6 undetectable. | ⛔ blocks measurement |
| **8** | **17 suites print zero `ok:` lines** (full list in `COVERAGE_HARNESS.md` §5). | Their assertion vocabulary is private. 3 of them exit 0. | high |
| **9** | **23 genuinely vacuous cases / 44 `VACUOUS` lines** — the checklist's "43 + 5" tally is a substring scan over the word `vacuous` and is **wrong in both count and location**. | §2. | high |
| **10** | **2 `SETUP BUG:` lines, both in one suite, both wrong-card fixtures.** | `test_parity_blades_live_start.c`: `PL!HS-pb6-002-R` resolves to **id = −1 (no such card)**; `PL!HS-bp1-002-CL` resolves to **id = 2793 = a different card**. Every assertion in those cases tests the wrong card and still passes. | high |
| **11** | **30 suite names in circulation have no `tests/*.c`.** Full list in `COVERAGE_HARNESS.md` §6. | Each one reads as passing, because `make` emits no `FAIL` for a suite that does not exist. | high |
| **12** | **`tests/test_parity_look_per_card.c`: 2 FORK children die with exit 139.** | `CRASH    n_tomari_accept_6  <-- unexpected child exit 139` and `n_tomari_empty_hand`. The checklist records this as a *non-reproducing* flake; it reproduced on the first run of a fresh from-scratch build. | medium |

---

## 2. THE 23 VACUOUS CASES — with reachability

**Where they actually are** (the checklist names ten suites; **all ten of those emit zero `VACUOUS`
lines today**. The live ones are three suites the checklist does not mention):

| suite | `VACUOUS` lines | distinct vacuous cases |
|---|---:|---:|
| `test_parity_required_hearts` | 26 | **13** (12 named on stderr, 1 stdout-only) |
| `test_parity_cost_discount` | 10 | **10** |
| `test_parity_state_per_card` | 8 | 8 (these are the suite's own **"PROMPT CONTROL"** markers — a non-vacuity guard firing loudly, not a vacuous assertion) |

**Why the earlier tally was wrong:** 314 lines in the corpus **contain** the substring `vacuous`;
only **44 begin with it**. The 314 are dominated by per-case summary lines that read
`… 0 failure(s), 0 setup bug(s), 0 vacuous` — a line that *reports zero* vacuous assertions is
itself counted as one by any substring scan. Suite distribution of the 314:
`parity_cost_discount` 100, `parity_required_hearts` 97, `parity_full_round` 22,
`parity_look_per_card` 16, `parity_state_per_card` 9, `parity_target_player` 8,
`parity_recover_q79` 6, and 25 more. **A vacuous audit must anchor on `^VACUOUS`, never on a
substring.**

### 2.1 The 23 vacuous cases, ranked by the reachability of the behaviour they fail to prove

| # | vacuous case (suite) | what its zero was supposed to disprove | paired positive | behaviour reachability |
|---:|---|---|---|---|
| 1 | `c_five_plus_duplicate` (`required_hearts`) | 5 distinct members do **not** trigger the reduction | `c_six_distinct_reduce_two` (**RED**) | `modify_required_hearts` **28/985** · 43 pairs · 87 eff · 315 cond |
| 2 | `c_five_plus_renosora_live` | a 蓮ノ空 already live does not add a 6th | `c_six_distinct_reduce_two` (RED) | 28/985 |
| 3 | `c_five_plus_wrong_group` | a wrong-group member does not add a 6th | `c_six_distinct_reduce_two` (RED) | 28/985 |
| 4 | `d_mirage_one_baton` | one baton-touch does not reach the threshold | `d_mirage_two_baton_heart05` (**RED**) | 28/985 + `K=baton_touch_trigger` **30/985** · 63 pairs |
| 5 | `d_mirage_non_renosora` | a non-蓮ノ空 mirror member does not count | `d_mirage_two_baton_heart05` (RED) | 28/985 |
| 6 | `d_mirage_renosora_no_baton` | a 蓮ノ空 with no baton does not count | `d_mirage_two_baton_heart05` (RED) | 28/985 |
| 7 | `f_center_zero_heart03` ⚠ *name mismatch, see below* | a centre member with no heart03 grants nothing | `f_center_two_heart03_reduces_1` (**RED**) | 28/985 |
| 8 | `f_no_center_member` | an empty centre grants nothing | `f_center_two_heart03_reduces_1` (RED) | 28/985 |
| 9 | `f_center_non_mus` | a non-Musume centre member grants nothing | `f_center_two_heart03_reduces_1` (RED) | 28/985 |
| 10 | `i_no_qualifying` | neither appeared nor moved → no reduction | `i_two_appeared_reduce_2` (**RED**) | 28/985; `K=appearance` 69 nodes, `K=movement` 117 nodes |
| 11 | `i_neither_appeared_nor_moved` | neither leg satisfied | `i_two_appeared_reduce_2` (RED) | 28/985 |
| 12 | `i_appeared_then_left` | appearance then departure cancels | `i_two_appeared_reduce_2` (RED) | 28/985 |
| 13 | `l_heart02_lt5_no_effect` | below the threshold → no increase | `l_heart02_ge5_increase` (**RED**) | 28/985 |
| 14 | `f_no_reduction_for_member_with_ability` (`cost_discount`) | a member WITH the ability gets no reduction | `f_reduction_applies_to_no_ability_member` (**RED**) | `modify_cost` **22/985** · 48 pairs · 97 eff · 235 cond; `X=cost_limit` **76/985** · 171 pairs |
| 15 | `n_does_not_reduce_low_cost_group_member` | a low-cost group member is unaffected | `n_reduces_cost_seventeen_group_member_by_two` (**RED**) | 22/985 |
| 16 | `n_does_not_reduce_wrong_group_member` | a wrong-group member is unaffected | `n_reduces_…` (RED) | 22/985 |
| 17 | `n_absent_after_source_removed_before_play` | the modifier is gone once the source leaves | `n_reduces_…` (RED) | 22/985 |
| 18 | `k_gameplay_decline_pays15_keeps_hand` | declining the optional cost keeps the hand | `k_gameplay_accept_cost10_discards_hand` (**RED**) | 22/985 |
| 19 | `k_accept_with_ten_energy_pays_ten` | 10 energy pays the 10 cost | `k_gameplay_accept_…` (RED) | 22/985 |
| 20 | `k_passive_cost_not_set_by_discard` | the discard did not set the passive cost | `k_gameplay_accept_…` (RED) | 22/985 |
| 21 | `k_passive_cost_not_set_with_hand_cards` | holding cards did not set it | `k_gameplay_accept_…` (RED) | 22/985 |
| 22 | `g_four_unique_groups_cost_zero` | 4 unique groups ⇒ cost 0 | `g_reduction_charges_effective_cost` (**RED**) | 22/985; `X=cost_limit` 76/985 |
| 23 | `g_zero_cost_all_rarities` | cost 0 for every rarity | `g_reduction_charges_effective_cost` (RED) | 22/985 |

⚠ **A fourth harness defect, found while tabulating this:** case **`f_center_zero_heart03`**
(stdout) and **`f_center_rin_zero_heart03`** (stderr) are **the same case under two different
names.** Any measurement that joins stdout and stderr per case name loses this case entirely.
This is the same cross-stream vocabulary split as `FAIL:` vs `FAILED` vs `FAIL [`.

---

## 3. THE TOP-15 BEHAVIOURAL GAPS, RANKED BY REACHABILITY

Ranked by `n/985` (distinct printed abilities). Every row is a behaviour with **no C test that
proves it** — either no test at all, or a test whose assertion is mechanically weaker, inverted,
or vacuous.

| rank | behaviour | n/985 | pairs | eff | cond | why it is a gap | reachability source |
|---:|---|---:|---:|---:|---:|---|---|
| **1** | `group_names` on an effect (group/unit filtering of every targeted effect) | **273** | 543 | 1 574 | 1 708 | `tests/test_modules/rules/group_name_equivalence.rs` has 3 `#[test]` and **no card key at all**; `parity_card_filters` is a **phantom suite**. A17's six `CardFilter` carriers (`groups[]`, `exclude_group_names`, `cost_limit_min/max`, `original_heart_total`) have **0 producers** — `card_filter_from_effect` sets none — so **every corpus number ever quoted about those carriers came from dead code**, exactly as the checklist says. The matcher exists; nothing populates it. | `X=group_names` |
| **2** | `X=cost_limit` (the cost-limit filter, 181 `cost_limit_operator` nodes) | **76** | 171 | 465 | 260 | `parity_card_filters` phantom; `A17`'s `cost_limit_min`/`cost_limit_max` carriers are 0-producer, and `cost_limit_min`/`max` reach **1/985** so the producer patch is a 1-ability change. | `X=cost_limit` |
| **3** | `K=trigger_event` (which event a condition listens for) | **112** | 235 | 494 | 3 044 | Largest single condition key after `text`/`location`. D8 assigns it to T7/`vm.c` and marks it "NOT MINE". **No C suite is named after it**; `parity_trigger_paths` and `test_trigger_paths` cover the *scanning*, not the key's decode. | `K=trigger_event` |
| **4** | `src = deck_top` (1 094 effect nodes) | **119** | 274 | 1 094 | 503 | `parity_draw_top`, `parity_nazuna_at_deck_top`, `parity_lookup`, `parity_watchers`, `parity_vm`, `parity_resolver`, `parity_dynamic_count` are **all phantom**. The one real suite, `test_parity_deck_order.c` (773 L), carries **no Rust citation** and reports **0 FAIL with 159 `ok:` lines** — it cannot be mapped to an oracle test, so its coverage is unverifiable. F6/F7's `deck_top`/`deck_bottom`/`energy_deck` arms are live on 165/985 abilities. | `src=deck_top` |
| **5** | `dst = deck_top`/`deck_bottom`/`deck_top_or_bottom` | **57** | 130 | 415 | 304 | same phantom-suite cluster; `deck_top_or_bottom` is a **distinct value (6 nodes)** with no named test anywhere. | `dst` census |
| **6** | `K=group_names` (group filter on a **condition**, a different surface from row 1) | **155** | 270 | 543 | 3 035 | `test_parity_condition_and_effect.c` is the intended home and has **no Rust citation** (one of the 52). `tests/test_modules/rules/group_name_equivalence.rs` is the Rust oracle and is **uncited**. | `K=group_names` |
| **7** | `K=distinct` (the distinct-name cap **on a condition**) | **23** | 52 | 111 | 741 | `tests/test_modules/rules/targeting/max_distinct_names_test.rs` has 3 `#[test]` and **no card key**; `parity_max_distinct_names` is a **phantom** (the real file is `test_max_distinct_names.c`). That file emits **4 043 `ok:` lines and 6 FAIL** — an enormous loop-driven suite whose oracle mapping is unwritten. | `K=distinct` |
| **8** | `K=baton_touch_trigger` | **30** | 63 | 129 | 817 | I5 records that the baton-touch latch (`baton_touch_used`) was cleared by the VICTORY rollover and that "from the first touch onward triggers.c:243 suppressed every 「バトンタッチされたとき」". The suite `test_parity_jidou.c` / `test_baton_touch.c` exist; the former is one of the 52 uncited. 30/985 abilities, 817 condition nodes. | `K=baton_touch_trigger` |
| **9** | `modify_required_hearts` — **13 vacuous cases, zero proven** | **28** | 43 | 87 | 315 | §2 rows 1–13. **Every paired positive is RED**, so not one of the 13 negatives proves anything. This is the single largest "we have tests and they prove nothing" block in the corpus. | `A=modify_required_hearts` |
| **10** | `modify_cost` — **10 vacuous cases, zero proven** | **22** | 48 | 97 | 235 | §2 rows 14–23. Same shape: every paired positive RED. | `A=modify_cost` |
| **11** | `K=negation` (the condition negation flag) | **30** | 70 | 225 | 731 | D4 landed the **double**-negation numeric tail for 3 nodes; the remaining 70 negation nodes (3 044→731 condition nodes) have no C test that pins single- vs double-negation. D3's four post-dispatch negation exemptions are implemented but **untested for 3 of 4**. | `K=negation` |
| **12** | `src = preceding_moved` / `recently_moved` / `those_cards` / `looked_at*` | **18** | 43 | 139 | 226 | D6's BLOCKED-ON sub-item `count_for_preceding_moved_score` (card.rs:3902, 2 nodes). The C has **two** readers of "the cards just moved" (`g->recently_moved` at `condition.c:3290` vs the queue entry's `moved_cards`) and **neither** satisfies both clauses of `PL!S-bp3-008-R`. 18/985 abilities, and **no test can distinguish the two readers** because no C test stages a `preceding_moved` node. | `src` census |
| **13** | `K=watches_area_move` | **17** | 29 | 45 | 289 | The checklist records 17→29 nodes; **measured 29 nodes / 17 abilities — exact**. `parity_watchers` and `parity_area_move`-adjacent coverage is partly phantom. E16's *reader* (`condition.c:1184-1209`) is still unwired, so all 17 abilities are on a path that matches the **moved card** where Rust matches `cause_card_id`. | `K=watches_area_move` |
| **14** | `K=any_location` | **18** | 36 | 90 | 544 | D8 assigns the consumer to `card.rs:1355` and marks it "NOT MINE / BLOCKED". 36 nodes confirmed. `card.rs:1355` has no C reader, so all 36 nodes evaluate to the wrong answer today, and **no test asserts the right answer.** | `K=any_location` |
| **15** | `X=exclude_group_names` / `exclude_cards` / `exclude_names` | **4** | 12 | 24 | 0 | A17: matcher landed (H1), **producer 0**. 12 effect nodes across 4 abilities read a filter that can never carry the field. `parity_card_filters` (phantom) was the suite that would have caught it. | `X=exclude_*` |

### 3.1 Just below the cut, with their counts (so the ranking is auditable)

| behaviour | n/985 | pairs | note |
|---|---:|---:|---|
| `src=deck_top`+`deck_bottom`+`energy_deck` (F6/F7 arms) | 165 | 365 | reachable; one uncited suite covers it |
| `A=conditional_on_result` | 23 | 53 | `parity_effect_conditions` is phantom |
| `K=self_effect_only` | 4 | 11 | checklist "11 nodes" ✅ |
| `K=energy_placed` | 11 | 4 | checklist "11 nodes" ✅ |
| `A=position_change` | 27 | 64 | |
| `A=set_heart_type` | 9 | 13 | `effects/hearts/set_heart_type_multi_target_test.rs` **does not exist** |
| `A=perform_yell` | 5 | 8 | |
| `A=play_baton_touch` | 3 | 10 | |
| `A=choose_target_player` | 6 | 10 | J9-N: measured `n_child=1`, still open |
| `A=activate_ability` | 4 | 11 | |
| `A=invalidate_ability` | 3 | 7 | |
| `X=heart_total_limit` (E12) | 5 | 8 | 2 abilities are `replacement_appear_wait` |
| `K=blade_limit`/`blade_limit_operator`/`original_value` (D8) | 6 | 13 | |
| `K=cost_total` (D4) | 7 | 13 | checklist "22 nodes" ✅ (22 = cond nodes) |
| `K=no_excess_heart` (D7) | 2 | 3 | **checklist says 2 nodes; measured 3** |
| `X=count_as`/`counted_by` (E10) | 1 | 1 | `PL!-pb2-041-L` |
| `X=non_stackable` (E1 dedupe) | 3 | 3 | checklist "3 of 985" ✅ |
| `K=same_name` | 1 | 1 | |
| `K=activated_this_turn` (condition) | 1 | 1 | the "3" in D8 is the **CardFilter** surface, not the condition |

---

## 4. `NO C TEST` — the 21 tests with no C counterpart at all

| # | Rust file (`#[test]` count) | behaviour | n/985 | pairs | eff |
|---:|---|---|---:|---:|---:|
| G-A0 | `tests/test_modules/integration/pvp_room_test.rs` (**14**, **never compiled** — `build.rs:EXCLUDED_MODULES`) | the whole pvp-room flow | **1** | 1 | 3 |
| G-B1 | `tests/test_modules/effects/cost_mod/per_card/cost_modifier_expiry_sign_test.rs` (3) | the **sign** of a cost modifier when its source leaves; only card `PL!HS-bp5-005-AR`, which **no C test stages** | **1** | 1 | 6 |
| G-C1 | `tests/test_modules/rules/targeting/max_distinct_names_test.rs` (3) | distinct-name cap on a hand discard | **19** (`K=distinct`) / 23 | 52 | 111 |
| G-C2 | `tests/test_modules/rules/group_name_equivalence.rs` (3) | group-name equivalence | **273** (`X=group_names`) / 155 (`K=group_names`) | 543 | 1 574 |
| G-C3 | `tests/run_all.rs` (3) | `parse_heart_color` lenient fallback; corpus contains no `action:"custom"` | 0 (invariant) | — | — |
| G-D1 | `tests/test_modules/characterization/card_type_filter_test.rs` (3) | `card_type` on an effect; C stages `PL!-pb1-029-SRL` but asserts nothing | 1 | 1 | 1 |
| G-D2 | `tests/test_modules/characterization/card_database_parity_test.rs` (2) | DB↔blob parity for `PL!N-bp1-002-R` | 2 | 2 | 7 |
| G-D3 | `tests/test_modules/effects/choice/selection_filter/niko_debut_draw_per_activated_parser_gap_test.rs` (2) — **untracked, new** | okinā draw **per activated member**; `PL!-pb2-018-P＋` | 2 | 2 | 3 |
| G-D4 | `tests/test_modules/effects/recover/to_hand/self_to_waitroom_liella_recovery_test.rs` (2) | self→waitroom then Liella! recovery. `PL!S-bp4-018-N` and `PL!S-pb1-001-PR` carry **0 abilities in the corpus** — the C stages cards that do nothing | 0 | 0 | 0 |

**Two of the nine (G-D3, G-D4) are among the six brand-new UNTRACKED Rust test files.** They are
gaps created *during* the audit window. **The audit window is not closed.**

---

## 5. CORRECTIONS TO TALLIES THIS PROJECT HAS ALREADY PUBLISHED

Every one of these is a measurement, not an opinion. `PARITY_CHECKLIST.md` is wrong or
un-reproducible on the following, as of the working tree read on 2026-10-04.

| published claim | measured | direction |
|---|---|---|
| "**43 `VACUOUS` + 5 `SETUP BUG`** assertions" (H5), located in `parity_look_per_card` 14, `parity_target_player` 8, `parity_recover_q79` 5, `parity_recover_hand_a` 4, `parity_wave1c` 3, `parity_cce_gaps` 2, `parity_look_and_filter` 2, `parity_parser_e2e_extra` 2, `parity_wave1a` 2, `parity_recover_live` 1 | **44 `^VACUOUS` lines / 23 distinct cases, ALL in `parity_required_hearts` (13), `parity_cost_discount` (10), `parity_state_per_card` (8 controls).** All ten named suites emit **zero** `VACUOUS` lines. `SETUP BUG`: **2**, both in `parity_blades_live_start`. | **wrong count, wrong location** — a substring scan counted `0 vacuous` summary lines |
| "`no_excess_heart` … **2** nodes" (D7) | **3** condition nodes across **2** abilities | wrong node count |
| "`energy_relative` … added: **2** nodes" (D4) | **`energy_relative` = 0 condition nodes** in the shipped bytecode | **wrong — the key is not on the wire** |
| "J9-F: `PL!-bp6-020-L` ab#0 satisfies both halves (`trigger_type=each_time` + `watches=true`), so the missing guard **is reachable**" | **`watches` = 0 occurrences**, as an effect extra key AND as a condition key. There is no `watches` key in the shipped bytecode at all. | **wrong — reachability claim does not hold** |
| "1 236 `WEAKENS` pairs" is this file's own number, and its floor-not-total status is stated in `COVERAGE_MAP.md` §3.3 | — | — |
| F6/F7: "`deck_top` and `deck_bottom` are separate arms … `rb_zone_of_str` no longer folds it" | **reachable and large:** `src=deck_top` **1 094 nodes / 119 abilities**, `src=deck_bottom` 42 nodes / 9 abilities, `src=energy_deck` 78 nodes | ✅ confirmed, larger than implied |
| C4: "all 124 `change_state` cost nodes carry `state_change` = `wait`" | `A=change_state` = 320 pairs / 146 abilities / 910 effect nodes (this counts *effect* nodes; the *cost* subset is the 124 the claim is about — not re-measured here, flagged as unverified) | not verified |
| "F4: `tools/census_fw.c` finds **0** corpus nodes that reach the `selected_card_score` shape" | `X=dynamic_count` = **0 nodes** ✅ consistent | ✅ |
| "T5-2: **0** of 2 206 effect nodes carry an `id` key" | `X=id` = **0** nodes ✅ | ✅ |
| "G2: `look`/`reveal_effect`/`set_card_identity_all_regions` = 0 each, against 50 action literals" | **50** distinct effect actions ✅, and none of those three is among them | ✅ |
| G1/G2 reachability method | `dynamic_count`, `id`, `original_heart_total`, `groups[]`, `baton_touch_source`, `any_location`-as-effect-key, `action_reference`-as-effect-key, `reference_card`-as-effect-key, `require_position_cards`-as-effect-key are all **0 effect-extra keys** but several are live **condition** keys. **Effect-extra and condition-key surfaces must be counted separately** — several published tallies conflate them. | **method correction** |

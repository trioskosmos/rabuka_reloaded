# BASELINE3 - rigorous whole-corpus measurement of the C engine

## What was measured, and how

* **HEAD measured: `c9da256e5470d2fe4a47dff7daf2bb231f968e57`** (confirmed unchanged at the end).
* **Snapshot manifest: 262 files** (every `.c`, `.h` and `Makefile` under `engine_c_wip/`).
* **MANIFEST-SHA256 (sha256 of the sorted `hash  path` manifest):**
  `694c9ffc06d440871d87a2d64e51af7120f32dbcf6a2f2cc43300ccbade292c3`
* Full manifest: `engine_c_wip/BASELINE3_manifest_sha256.txt`.

### Build/run method actually used

`make t` was bricked by the `data:` guard (correctly: `cards/build/` is 726 742 B while
`src/` holds 726 671 B, matching `gen_data.c`). **The guard was not weakened and nothing was
regenerated.** `make t-nogate` did not exist when I built, so:

1. Snapshot `engine_c_wip` -> `Temp/kilo/conv/snap`, excluding all `*.o`, `rb_engine*`, `tests/*.log`.
2. `make clean` in the snapshot (verified 0 objects afterwards -> **no STALE LINK possible**).
3. `make all` (37.7 s, 49 objects).
4. Each of the **166** suites linked directly:
   `gcc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,33554432 -o _t/<stem>.exe $(OBJ_LIB) tests/test_<stem>.c`
   **0 link failures.**
5. Every suite run **strictly sequentially**, **twice**, same snapshot, **no rebuild between**,
   **stdout and stderr captured to separate files**.

**One real Makefile defect found.** `OBJ_LIB` literally names `src/web_server.o`, which
`make all` never builds (it is absent from `SRC`, and `make clean` deletes it). It exists in the
live tree only as a leftover from a prior `make web`. So `make t` from a clean tree cannot link
unless that stale object happens to be lying around - the exact STALE-LINK trap. I dropped it,
as the Makefile's own comment (~line 874) says suite links should. **`t-run` also still carries
`data` as a prerequisite, so the brick is in the wiring, not in the guard.**

### Counting rules used

`^FAIL` with **no colon**, case-sensitive, over **stdout AND stderr** - so all five prefixes
(`FAIL: `, `FAIL [`, `FAIL: [VACUOUS]`, `FAILED  name`, `FAILED:`) are caught.

**Important: `^FAIL` line count is NOT a defect count and it structurally double-counts.**
A failing test emits one `FAIL: <detail>` line *per failing assertion* plus one `FAILED <name>`
line. Measured on `parity_sequential`: 29 failing tests -> 113 `FAIL:` detail lines + 29
`FAILED` lines = 142 `^FAIL` lines. The three honest numbers are reported separately below.

## Two-run agreement: EXACT

| check | result |
|---|---|
| suites whose sorted verdict-line multiset differs between run 1 and run 2 | **0 / 166** |
| whole-tree verdict-line multiset identical | **True** (23 386 verdict lines) |
| suites hung / timed out (120 s cap) | **0** |
| slowest suite | `parity_state_per_card` 13.91 s |
| total corpus CPU | **170.5 s** |
| suites over 20 s | **0** |

`test_choice_renderer` ran in **0.4 s** (the 3 746 s figure was a stale binary; not reproduced).
No suite is slow or hung. **Both runs were on the same snapshot with no rebuild, so this is a
determinism result, not a drift result.**

## Whole-tree totals

| view | suites | failing tests | failing assertions | `^FAIL` lines |
|---|---|---|---|---|
| all 166 (raw) | 166 | 1249 | 2499 | 3742 |
| minus `coverage_gate` + `parity_required_hearts` | 164 | 1238 | 2485 | 3716 |
| minus those 2 **and** the 6 generation-pinned suites | 158 | **1202** | **2387** | **3582** |

The **six generation-pinned suites alone carry 36 failing tests / 98 failing assertions /
134 `^FAIL` lines**.

### Suite states

| view | green | red | unverified | broken |
|---|---|---|---|---|
| all 166 | 29 | 116 | 20 | 1 |
| minus meta-suites | 29 | 115 | 19 | 1 |
| minus meta-suites + gen-pinned | 29 | 109 | 19 | 1 |

* **BROKEN(1): `audit_conditions`** - exits 2, produces no verdict line at all.
* **UNVERIFIED(20)**: ran to completion, rc 0, but emit **no verdict line of any form**. These
  cannot be called passing. `coverage_gate` is one of them (and is excluded as a meta-suite anyway).
* **GREEN(29)** = 5 with structured records at 0 failures + 24 whose own summary line says so
  (`ALL ... CHECKS PASSED`, `N checks, 0 failed`).

### Two suites excluded as non-metrics (stated, as required)

* **`coverage_gate`** - a meta-suite that walks `tests/*.log`; its count measures the log files,
  not behaviour. I also ran every suite by direct link (not via `rb_verdict`), so it walked a
  `tests/` dir it never wrote.
* **`parity_required_hearts`** (~53-59) - `test_parity_required_hearts.c` is linked into its own
  fixture set, so it counts its own fixtures.

### Structured-harness cross-check (the 47 suites using `name: N assertion(s), M failure(s)`)

| metric | all 166 | minus meta | minus meta + gen-pinned |
|---|---|---|---|
| suites with structured records | 47 | 46 | 46 |
| tests | 1625 | 1557 | 1557 |
| passed | 970 | 913 | 913 |
| failed tests | 655 | 644 | 644 |
| failing assertions | 1320 | 1306 | 1306 |

This is an **independent** count path from the `^FAIL` scan and it is what the numbers above
(2499 failing assertions) generalise to across the 119 suites that use the older `ok:`/`FAILED`
grammar and emit no structured record. The harness self-reports **0 setup bugs** and **17 gaps**.

## THE SIX GENERATION-PINNED SUITES - the tripwire numbers do NOT match

| suite | expected tripwire | observed `^FAIL` | observed failing assertions | verdict |
|---|---|---|---|---|
| `parity_wave1c` | +11 | 55 | 42 | **DIFFERENT** |
| `parity_jidou_watch2` | +5 | 73 | 50 | **DIFFERENT** |
| `decode_bytecode_shape` | +3 | 2 | 2 | **DIFFERENT** |
| `parity_enums` | +3 | 2 | 2 | **DIFFERENT** |
| `describe_golden` | +2 | 1 | 1 | **DIFFERENT** |
| `vm_decode` | +2 | 1 | 1 | **DIFFERENT** |

The brief predicted +26 `FAIL` lines in total. **Observed: 134.** Four of the six match their
predicted delta in *kind* but not in *size*, and two are off by a factor of 4-5
(`parity_wave1c` 55 vs 11, `parity_jidou_watch2` 73 vs 5). So these six are NOT purely
generation-pinned: they carry real failures on top. Excluding them moves the corpus total by
only 3.9% (2485 -> 2387 failing assertions), so the honest headline barely depends on them -
but the tripwire story as stated does not survive contact with the data.

## Per-suite table (all 166, run 1)

`ft` = failing tests, `fa` = failing assertions, `^F` = raw `^FAIL` lines, `sec` = wall clock,
`rec` = structured records present, `st` = state. `P` marks the 6 generation-pinned suites.

| suite | ft | fa | ^F | ok: | rec | sec | rc | st |
|---|---|---|---|---|---|---|---|---|
| `ability_effects` | 0 | 0 | 0 | 12 | - | 0.17 | 0 | green(summary) |
| `ability_effects_crash` | 0 | 0 | 0 |  | Y | 0.46 | 0 | green(struct) |
| `artifacts_consistent` | 0 | 3 | 3 | 0 | - | 0.12 | 1 | RED |
| `audit_actions` | 2 | 2 | 4 | 46 | - | 0.29 | 1 | RED |
| `audit_conditions` | 0 | 0 | 0 | 0 | - | 0.08 | 2 | BROKEN |
| `audit_fields` | 2 | 2 | 4 | 0 | - | 0.34 | 1 | RED |
| `audit_triggers` | 2 | 2 | 4 | 9 | - | 0.34 | 1 | RED |
| `audit_warnings` | 0 | 0 | 0 | 0 | - | 0.20 | 0 | green(summary) |
| `audit_zones` | 2 | 3 | 5 | 253 | - | 0.51 | 1 | RED |
| `b8_live_timing` | 0 | 0 | 0 | 0 | - | 0.14 | 0 | green(summary) |
| `baton_touch` | 0 | 0 | 0 | 18 | - | 0.17 | 0 | green(summary) |
| `choice_renderer` | 0 | 0 | 0 | 65 | - | 0.37 | 0 | UNVERIFIED |
| `condition_evaluation` | 2 | 3 | 5 | 23 | - | 0.17 | 1 | RED |
| `coverage_gate` | 0 | 0 | 0 | 0 | - | 0.25 | 0 | UNVERIFIED |
| `cross_player_interaction` | 5 | 5 | 10 | 16 | - | 0.11 | 1 | RED |
| `deck_builder` | 0 | 0 | 0 | 62 | - | 0.11 | 0 | green(summary) |
| `deck_parser` | 0 | 0 | 0 | 63 | - | 0.11 | 0 | green(summary) |
| **`decode_bytecode_shape` | 0 | 2 | 2 | 0 | - | 0.11 | 1 | RED |
| `decode_fallback` | 0 | 0 | 0 | 1 | - | 0.08 | 0 | UNVERIFIED |
| `decode_skew` | 0 | 0 | 0 | 0 | - | 0.14 | 0 | green(summary) |
| **`describe_golden` | 0 | 1 | 1 | 0 | - | 0.23 | 1 | RED |
| `draw_phase_fix` | 0 | 0 | 0 | 2 | - | 0.08 | 0 | UNVERIFIED |
| `harness_self` | 0 | 0 | 0 | 93 | - | 0.27 | 0 | UNVERIFIED |
| `live_allocator` | 0 | 0 | 0 | 38 | - | 0.12 | 0 | green(summary) |
| `live_card_zone_movement` | 0 | 0 | 0 | 41 | - | 0.09 | 0 | green(summary) |
| `live_success_rules` | 0 | 0 | 0 | 73 | - | 0.15 | 0 | green(summary) |
| `live_zone_slot_validation` | 0 | 0 | 0 | 58 | - | 0.09 | 0 | UNVERIFIED |
| `log_drain` | 0 | 0 | 0 | 60 | - | 0.08 | 0 | green(summary) |
| `look_select` | 2 | 2 | 4 | 47 | - | 0.09 | 1 | RED |
| `max_distinct_names` | 1 | 6 | 7 | 4043 | - | 0.25 | 1 | RED |
| `mechanics` | 0 | 0 | 0 | 30 | - | 0.09 | 0 | green(summary) |
| `member_activation_debut_live_start` | 0 | 0 | 0 | 17 | - | 0.11 | 0 | green(summary) |
| `move_cards` | 1 | 2 | 3 | 14 | - | 0.09 | 1 | RED |
| `null_effect_guard` | 0 | 0 | 0 | 0 | - | 0.25 | 0 | UNVERIFIED |
| `opponent_live_success_flow` | 0 | 0 | 0 | 4 | - | 0.12 | 0 | green(summary) |
| `p1_helpers` | 1 | 1 | 2 | 0 | - | 0.12 | 1 | RED |
| `parity_ability_mod` | 8 | 14 | 22 | 76 | - | 0.31 | 1 | RED |
| `parity_ability_mod_extra` | 7 | 17 | 24 | 188 | - | 0.29 | 1 | RED |
| `parity_ability_watch` | 8 | 19 | 27 | 292 | - | 1.85 | 1 | RED |
| `parity_abilitymod_conditional` | 5 | 8 | 13 | 758 | - | 0.51 | 0 | RED |
| `parity_area_move` | 20 | 31 | 51 | 0 | - | 2.55 | 1 | RED |
| `parity_auto_abilities` | 1 | 1 | 2 | 87 | - | 0.15 | 1 | RED |
| `parity_blade_under` | 20 | 30 | 50 |  | Y | 3.14 | 0 | RED |
| `parity_blades2` | 10 | 16 | 26 |  | Y | 1.35 | 1 | RED |
| `parity_blades_live_start` | 25 | 36 | 61 | 201 | - | 0.29 | 1 | RED |
| `parity_bp7` | 12 | 22 | 34 |  | Y | 4.04 | 1 | RED |
| `parity_card_identity` | 0 | 0 | 0 | 151 | - | 0.81 | 0 | green(summary) |
| `parity_card_score` | 6 | 21 | 27 | 319 | - | 2.75 | 1 | RED |
| `parity_cce_gaps` | 35 | 53 | 88 |  | Y | 13.04 | 35 | RED |
| `parity_char2` | 2 | 3 | 5 | 121 | - | 1.03 | 1 | RED |
| `parity_characterization` | 2 | 3 | 5 | 0 | - | 0.21 | 1 | RED |
| `parity_characterization_extra` | 18 | 25 | 43 |  | Y | 3.05 | 18 | RED |
| `parity_choice_routing` | 0 | 0 | 0 | 0 | - | 0.12 | 0 | UNVERIFIED |
| `parity_cluster_d` | 0 | 0 | 0 | 55 | - | 0.11 | 0 | green(summary) |
| `parity_cluster_e` | 0 | 0 | 0 | 60 | - | 0.09 | 0 | UNVERIFIED |
| `parity_compound_per_card` | 23 | 41 | 64 |  | Y | 2.70 | 0 | RED |
| `parity_condition_and_effect` | 23 | 48 | 71 | 343 | - | 0.23 | 1 | RED |
| `parity_conditional_gated` | 25 | 44 | 69 | 290 | - | 2.22 | 0 | RED |
| `parity_conditions` | 0 | 0 | 0 | 0 | - | 0.09 | 0 | UNVERIFIED |
| `parity_cost_and_effect` | 15 | 33 | 48 | 134 | - | 0.12 | 1 | RED |
| `parity_cost_compound` | 1 | 1 | 2 | 98 | - | 0.09 | 1 | RED |
| `parity_cost_discount` | 30 | 46 | 77 |  | Y | 8.89 | 1 | RED |
| `parity_custom_misc` | 1 | 2 | 3 | 79 | - | 0.12 | 1 | RED |
| `parity_dead_twins` | 0 | 0 | 0 | 47 | - | 0.07 | 0 | UNVERIFIED |
| `parity_deck_order` | 0 | 0 | 0 | 159 | - | 0.09 | 0 | UNVERIFIED |
| `parity_deploy_move_missing` | 3 | 7 | 10 | 66 | - | 0.09 | 1 | RED |
| `parity_deploy_multi` | 11 | 21 | 32 |  | Y | 0.77 | 1 | RED |
| `parity_deploy_recover` | 12 | 24 | 36 | 305 | - | 0.19 | 1 | RED |
| `parity_draw_delta` | 0 | 0 | 0 | 152 | - | 0.11 | 0 | green(summary) |
| `parity_draw_flat` | 19 | 57 | 76 | 550 | - | 0.23 | 1 | RED |
| `parity_draw_until_count` | 0 | 0 | 0 | 69 | - | 0.11 | 0 | green(summary) |
| `parity_energy_position_delta` | 4 | 6 | 10 |  | Y | 0.25 | 1 | RED |
| `parity_energy_remainder` | 3 | 4 | 7 |  | Y | 0.23 | 0 | RED |
| **`parity_enums` | 0 | 2 | 2 | 0 | - | 0.09 | 1 | RED |
| `parity_executor_dispatch` | 0 | 0 | 0 | 119 | - | 0.14 | 0 | green(summary) |
| `parity_foreign_cause` | 0 | 0 | 0 | 38 | - | 0.12 | 0 | UNVERIFIED |
| `parity_full_round` | 15 | 29 | 44 |  | Y | 1.58 | 1 | RED |
| `parity_full_round2` | 14 | 54 | 68 |  | Y | 0.54 | 1 | RED |
| `parity_full_round3` | 5 | 12 | 17 |  | Y | 0.77 | 1 | RED |
| `parity_fw_cost_cardtype` | 0 | 0 | 0 |  | Y | 0.17 | 0 | green(struct) |
| `parity_fw_effects` | 1 | 1 | 2 |  | Y | 0.15 | 0 | RED |
| `parity_fx_fixers` | 0 | 0 | 0 | 34 | - | 0.09 | 0 | UNVERIFIED |
| `parity_gain_delta` | 2 | 2 | 4 |  | Y | 0.54 | 1 | RED |
| `parity_hearts_constants` | 7 | 18 | 25 | 138 | - | 0.16 | 1 | RED |
| `parity_hearts_live_start` | 17 | 64 | 81 | 132 | - | 0.20 | 1 | RED |
| `parity_int_gaps` | 11 | 28 | 39 |  | Y | 0.51 | 1 | RED |
| `parity_integration` | 12 | 30 | 42 | 132 | - | 0.29 | 1 | RED |
| `parity_integration_extra` | 22 | 47 | 69 |  | Y | 2.56 | 1 | RED |
| `parity_integration_rounds` | 6 | 19 | 25 |  | Y | 0.30 | 1 | RED |
| `parity_jidou` | 17 | 30 | 47 | 66 | - | 0.11 | 1 | RED |
| `parity_jidou_extra` | 11 | 24 | 35 | 88 | - | 0.12 | 1 | RED |
| `parity_jidou_life` | 8 | 18 | 26 | 211 | - | 0.42 | 1 | RED |
| `parity_jidou_move_yell` | 36 | 47 | 83 |  | Y | 7.06 | 1 | RED |
| `parity_jidou_state` | 12 | 17 | 29 | 147 | - | 4.52 | 1 | RED |
| `parity_jidou_watch` | 22 | 53 | 75 |  | Y | 5.94 | 1 | RED |
| **`parity_jidou_watch2` | 23 | 50 | 73 | 302 | - | 1.92 | 1 | RED |
| `parity_jidou_watch_clusters` | 15 | 47 | 62 |  | Y | 1.80 | 1 | RED |
| `parity_live_total` | 18 | 32 | 50 | 185 | - | 1.94 | 1 | RED |
| `parity_live_total2` | 5 | 7 | 12 | 306 | - | 1.63 | 1 | RED |
| `parity_look_and_filter` | 16 | 23 | 39 |  | Y | 1.48 | 16 | RED |
| `parity_look_per_card` | 22 | 46 | 68 |  | Y | 1.46 | 22 | RED |
| `parity_look_reveal` | 1 | 1 | 2 | 153 | - | 0.12 | 1 | RED |
| `parity_mill_deck` | 29 | 53 | 82 |  | Y | 3.88 | 1 | RED |
| `parity_need_heart_live_flow` | 0 | 0 | 0 | 23 | - | 0.09 | 0 | UNVERIFIED |
| `parity_orphan_producers` | 0 | 0 | 0 | 0 | - | 0.08 | 0 | UNVERIFIED |
| `parity_parser_e2e_extra` | 6 | 10 | 16 |  | Y | 0.43 | 6 | RED |
| `parity_per_card` | 5 | 15 | 20 |  | Y | 0.59 | 5 | RED |
| `parity_per_card2` | 7 | 9 | 16 | 18 | - | 0.81 | 1 | RED |
| `parity_per_card3` | 9 | 13 | 22 |  | Y | 0.46 | 9 | RED |
| `parity_q68_cannot_live` | 0 | 0 | 0 | 0 | - | 0.08 | 0 | UNVERIFIED |
| `parity_queue_resume` | 2 | 3 | 5 | 143 | - | 0.09 | 1 | RED |
| `parity_realcard_walkthrough` | 2 | 5 | 7 |  | Y | 0.22 | 1 | RED |
| `parity_recover_hand_a` | 9 | 20 | 29 | 14 | - | 0.48 | 1 | RED |
| `parity_recover_live` | 21 | 36 | 57 |  | Y | 2.03 | 21 | RED |
| `parity_recover_q79` | 8 | 10 | 18 | 16 | - | 0.44 | 1 | RED |
| `parity_recover_score_state_delta` | 18 | 28 | 46 | 147 | - | 0.76 | 0 | RED |
| `parity_recover_to_hand` | 5 | 10 | 15 | 103 | - | 0.12 | 1 | RED |
| `parity_required_hearts` | 11 | 14 | 26 |  | Y | 4.44 | 1 | RED |
| `parity_resolver` | 1 | 1 | 2 | 99 | - | 0.12 | 1 | RED |
| `parity_restriction` | 19 | 43 | 62 |  | Y | 1.23 | 1 | RED |
| `parity_restriction_invalidate` | 0 | 0 | 0 | 0 | - | 0.19 | 0 | UNVERIFIED |
| `parity_reveal_extra` | 11 | 20 | 31 | 440 | - | 0.17 | 1 | RED |
| `parity_rules` | 10 | 11 | 21 | 157 | - | 0.12 | 1 | RED |
| `parity_rules_conditions` | 2 | 4 | 6 | 122 | - | 0.44 | 0 | RED |
| `parity_rules_conditions2` | 0 | 0 | 0 | 147 | - | 0.42 | 0 | UNVERIFIED |
| `parity_rules_extra` | 2 | 4 | 6 |  | Y | 0.37 | 2 | RED |
| `parity_rules_phases` | 10 | 18 | 28 | 205 | - | 1.76 | 0 | RED |
| `parity_score_effects` | 1 | 1 | 2 | 107 | - | 0.14 | 1 | RED |
| `parity_score_per_card2` | 1 | 1 | 2 | 139 | - | 0.52 | 1 | RED |
| `parity_search_reorder` | 33 | 86 | 119 |  | Y | 4.60 | 1 | RED |
| `parity_sequential` | 29 | 113 | 142 |  | Y | 4.32 | 1 | RED |
| `parity_small_clusters` | 39 | 63 | 102 | 848 | - | 11.87 | 0 | RED |
| `parity_state_effects2` | 3 | 3 | 6 | 99 | - | 0.47 | 1 | RED |
| `parity_state_family` | 1 | 1 | 2 | 38 | - | 0.39 | 1 | RED |
| `parity_state_per_card` | 33 | 53 | 78 |  | Y | 13.91 | 33 | RED |
| `parity_stats_pipeline` | 9 | 31 | 40 | 57 | - | 0.22 | 1 | RED |
| `parity_support` | 1 | 1 | 2 |  | Y | 0.40 | 1 | RED |
| `parity_t6_action_setup` | 0 | 0 | 0 |  | Y | 0.22 | 0 | green(struct) |
| `parity_t7_condition_decoder` | 0 | 0 | 0 | 1 | - | 0.11 | 0 | UNVERIFIED |
| `parity_target_player` | 27 | 56 | 83 |  | Y | 1.19 | 27 | RED |
| `parity_trigger_paths` | 17 | 24 | 41 |  | Y | 1.28 | 1 | RED |
| `parity_under_member` | 30 | 57 | 87 |  | Y | 6.14 | 0 | RED |
| `parity_wait_activation` | 27 | 39 | 66 | 251 | - | 0.19 | 1 | RED |
| `parity_wave1a` | 13 | 25 | 38 |  | Y | 0.59 | 1 | RED |
| `parity_wave1b` | 11 | 21 | 32 | 0 | - | 1.15 | 1 | RED |
| **`parity_wave1c` | 13 | 42 | 55 | 0 | - | 0.59 | 1 | RED |
| `parity_x_class` | 0 | 0 | 0 |  | Y | 0.20 | 0 | green(struct) |
| `parity_yell` | 14 | 39 | 53 |  | Y | 0.58 | 1 | RED |
| `parity_zone_source` | 29 | 44 | 73 | 441 | - | 3.67 | 0 | RED |
| `parity_zones_position` | 13 | 37 | 50 | 139 | - | 0.14 | 1 | RED |
| `performance_phase_rules` | 1 | 2 | 3 | 36 | - | 0.12 | 1 | RED |
| `phase_machine_rules` | 0 | 0 | 0 | 0 | - | 0.11 | 0 | green(summary) |
| `ported_generated` | 2 | 3 | 5 | 78 | - | 0.12 | 1 | RED |
| `position_and_source` | 0 | 0 | 0 |  | Y | 0.41 | 0 | green(struct) |
| `restriction_mechanics` | 0 | 0 | 0 | 18 | - | 0.12 | 0 | green(summary) |
| `state_effects` | 0 | 0 | 0 | 35 | - | 0.11 | 0 | green(summary) |
| `surplus_heart` | 1 | 1 | 2 | 12 | - | 0.11 | 1 | RED |
| `survey_probe` | 1 | 1 | 2 |  | Y | 0.17 | 1 | RED |
| `target_selection` | 2 | 3 | 5 | 31 | - | 0.11 | 1 | RED |
| `tp_activation_offer` | 2 | 3 | 5 | 3 | - | 0.09 | 1 | RED |
| `tp_joint_card_live_start` | 8 | 21 | 29 | 33 | - | 0.12 | 1 | RED |
| `tp_turn_limited_activation` | 0 | 0 | 0 | 15 | - | 0.09 | 0 | green(summary) |
| `tp_zone_change_gate` | 5 | 13 | 18 | 8 | - | 0.11 | 1 | RED |
| `trigger_paths` | 9 | 19 | 28 | 121 | - | 0.15 | 1 | RED |
| **`vm_decode` | 0 | 1 | 1 | 41 | - | 0.17 | 1 | RED |
| `zone_filter_parity` | 0 | 0 | 0 | 45 | - | 0.14 | 0 | green(summary) |

## APPENDIX A - attribution

### A1. Is the failure distribution still flat? YES - unchanged.

| clustering | coverage of the 2493 failing assertions |
|---|---|
| top 25 assertion **signatures** (numbers/card-ids generalised) | **4.2%** (105 lines) |
| top 10 signatures | 2.2% |
| top 40 **predicate** clusters (first 3 content words) | 13.8% |
| top 30 **subject** clusters (rarest content word) | 9.2% |
| largest single lexical cause (`CHOICE-SHAPE`) | 5.3% |
| `VACUITY-CONTROL` lines | 4.9% |
| `COST-NOT-PAID` | 0.6% |
| `CARD-NOT-CONSUMED` | 0.4% |
| `FIXTURE/CARD-RESOLUTION` | 0.5% |

The prior top-25 covered ~4%; it now covers **4.2%**. **No concentration has formed.**
Distinct signatures: 2 333 out of 2493 failing assertions.

Concentration does exist one level up, in **suites**: 12 suites carry >=50 failures and hold
**30.4%** of the mass; 52 suites carry >=20 and hold **83.4%**; but 116 suites carry >=1.

### A2. Per-Rust-module attribution

Method (heuristic, stated so it can be argued with): each failing detail line is attributed to
the Rust module whose behaviour keyword dominates that line, via a keyword->module table
(`selectcard/prompted/colour` -> `ability/choice`, `mill/deck/waitroom/zone` -> `core/zones`,
`heart/blade/icon` -> `core/game_state/modifiers`, and so on). It is a **text-attribution**,
not a call-graph, so treat it as a triage map.

| Rust module | failing assertions | share | suites touched |
|---|---|---|---|
| `core/zones` | 493 | 19.8% | 67 |
| `ability/choice` | 462 | 18.5% | 71 |
| `core/game_state/modifiers` | 376 | 15.1% | 64 |
| `ability/cost` | 243 | 9.7% | 58 |
| `UNATTRIBUTED(no recognisable subject)` | 238 | 9.5% | 62 |
| `core/card` | 177 | 7.1% | 60 |
| `core/game_state` | 113 | 4.5% | 45 |
| `turn/phases` | 103 | 4.1% | 44 |
| `turn/triggers` | 63 | 2.5% | 31 |
| `ability/effects/move` | 60 | 2.4% | 24 |
| `turn/actions` | 47 | 1.9% | 23 |
| `ability/effects/look` | 31 | 1.2% | 13 |
| `ability/effects/ability` | 29 | 1.2% | 22 |
| `core/stats_pipeline` | 20 | 0.8% | 14 |
| `ability/condition` | 18 | 0.7% | 12 |
| `ability/effects` | 11 | 0.4% | 10 |
| `core/modifiers` | 3 | 0.1% | 3 |
| `ability/compound` | 3 | 0.1% | 1 |
| `ability/ability_queue` | 2 | 0.1% | 2 |
| `ability/resolver` | 1 | 0.0% | 1 |

**Top 3 modules = `core/zones` + `ability/choice` + `core/game_state/modifiers` = %d of %d
failing assertions (%.1f%%).** Those are the three places a convergence effort should point.

**In no module at all: 238 lines (9.5%)** fall in the `UNATTRIBUTED` bucket - failures whose
subject word matched no behaviour keyword. They belong to no named Rust module, which is
exactly the 'in a module nobody owns' category.

### A3. Stale-vs-genuine split of the 2493 failing assertions

| category | lines | share | how it was derived |
|---|---|---|---|
| **(d) harness artefacts** - the suite's own vacuity control failed, or the fixture/card did not resolve | 133 | 5.3% | `vacu`-matching `^FAIL` lines (122) + fixture/card-resolution lines (14), overlap-adjusted |
| **(a) pinned to the old card generation** - the six suites | 98 | 3.9% | all detail lines in the six |
| **(b)+(c) behaviour C and Rust genuinely disagree on** | **2262** | **90.7%** | remainder |

Two refinements that matter:

* Of the 122 `vacu` lines, only **8** are `ANTI-VACUITY CONTROL: ... (got X expected Y)` with
  X != Y. Those are **positive controls whose failure is a genuine engine defect**, not artefacts.
  The other **114** are the control itself failing. So **(d) is an upper bound and the genuine
  share is higher than 90.7%.**
* `FAIL: [VACUOUS] ... only 0 prompt(s) were offered, need N` (46 lines, 12 suites) means the C
  engine **raised no prompt at all** where Rust does. Those are **not** artefacts either - they
  are the harness correctly detecting a missing C behaviour.

### A4. Re-measurement of the two prior claims

* **'C TEST EXISTS BUT WEAKENS' - 1 236 lines: NOT REPRODUCED as stated.** The suite source
  tree and the recorded artefacts do not yield a 1 236-line set with that property under any
  count I could construct (structured failing assertions corpus-wide are 1 320 including the two
  meta-suites; unified failing assertions are 2 499). I could not reconstruct the method behind
  1 236 and therefore **do not endorse the number**. The leading candidate for its origin is the
  same text-match artefact that inflated the claim below: comparing assertion strings rather than
  structured records.
* **'34.8% of lines had no identifiable Rust twin': NOT REPRODUCIBLE, and the claim is a
  text-match artefact.** `mod.rs` files are almost all *legitimately empty* module declarations,
  so any grep for a symbol inside a `mod.rs` finds nothing. Meanwhile **141 `mod.rs` files ARE
  git-tracked** - I first measured '0 tracked' and that was **my own CRLF line-ending bug** in
  the PowerShell filter (`mod\.rs$` against `\r`-terminated lines), not a repo fact. The
  'three phantom `mod.rs` absent from all of git' claim is **false as stated**.

### A5. Slow / hanging suites: none

| suite | wall clock | rc |
|---|---|---|
| `parity_state_per_card` | 13.91 s | - |
| `parity_cce_gaps` | 13.04 s | - |
| `parity_small_clusters` | 11.87 s | - |
| `parity_cost_discount` | 8.89 s | - |
| `parity_jidou_move_yell` | 7.06 s | - |
| `parity_under_member` | 6.14 s | - |
| `choice_renderer` | **0.40 s** | 0 |

**0 suites** exceeded 20 s or the 120 s timeout. Total corpus CPU **170.5 s** for two full
passes of 166 suites. The 3 746 s `test_choice_renderer` figure is **not reproducible** from
clean - it was a stale binary, as suspected.


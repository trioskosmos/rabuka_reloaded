# BASELINE — whole-tree parity measurement, `engine_c_wip` vs Rust

**Measured:** 2026-10-03, in an isolated snapshot, two strictly-sequential runs that agree byte-for-byte.
**Snapshot root:** `C:\Users\trios\AppData\Local\Temp\kilo\baseline\engine_c_wip`
**Manifest:** `C:\Users\trios\AppData\Local\Temp\kilo\baseline\MANIFEST.sha256` (240 entries: every `.c`, `.h`, `.py`, `.sh`, `.ps1` and the `Makefile`)
**Measured by:** the measurement authority. **No `.c`, `.h`, `Makefile`, `.sh`, `.ps1`, `.py` or test file was modified.** The snapshot is a copy; the shared tree was only read. No git write of any kind was performed (`stash@{0} session-wip` = `c458ab6f85ce5559041defd25165789b62cd8f21` was never popped, applied, or dropped).

---

## 1. HEADLINE

| quantity | value |
|---|---|
| **FAIL assertions (whole tree)** | **2 530** |
| &nbsp;&nbsp;of which `FAIL: ` shape (stderr) | 2 350 |
| &nbsp;&nbsp;of which `FAIL [` shape (stderr) | 180 |
| &nbsp;&nbsp;of which on **stdout** | **0** |
| distinct FAIL line texts | 2 393 |
| distinct FAIL lines, integers normalised to `N` | 2 271 |
| **per-test `FAILED` verdicts** | **949** |
| **`CRASH` verdicts** | **1** |
| per-test `ok` verdicts | 1 176 |
| per-test verdicts total (ok+FAILED+CRASH) | 2 126 |
| passing assertion echoes (`ok: …`) | 16 051 |
| suites **RED** | **108** |
| suites **GREEN** (ran, produced output, zero failure evidence) | **28** |
| suites GREEN-UNVERIFIED (ran, no failure evidence, no tally to confirm) | 5 |
| suites NO-OUTPUT (neither stream produced a byte) | 1 |
| **total suites** | **142** |
| `SETUP BUG` lines (fixture could not be built) | 5 |
| `VACUOUS` lines (assertion not proven non-vacuous) | 21 |
| build failures | **0** |
| hangs | **0** |

---

## 2. RUN 1 vs RUN 2 — DO THEY AGREE?

# YES. EXACTLY. 142 / 142, ON BOTH STREAMS, WITH NO NORMALISATION AT ALL.

This is the strongest form the check can take. The comparison was first done with
the snapshot path and any `0x…` heap pointer replaced by placeholders — that was
**not necessary**: with **zero** normalisation,

* `stdout` sorted line multiset identical in **142 / 142** suites
* `stderr` sorted line multiset identical in **142 / 142** suites
* whole-tree `stdout` multiset identical — **23 905** lines in run 1, **23 905** in run 2
* whole-tree `stderr` multiset identical — **30 032** lines in run 1, **30 032** in run 2
* FAIL assertion count identical: **2 530** vs **2 530** (delta **0**)
* failed-test count identical: **949** vs **949** (delta **0**)
* RED/GREEN identical: 108 / 28 / 5 / 1 in both passes

**Conclusion: there is NO nondeterminism in this tree.** Not hash-iteration order,
not uninitialised memory, not pointer-value dependence, not a stray file-static.
`tests/test_state_effects.c`-style suites do print live heap pointers on stderr
(`RESUME_CAPTURE … eff=0xa001990a0`), and those pointers came out **identical in both
runs** — the mingw heap is deterministic here. The historical concern that two runs of
byte-identical source could differ by 202 FAIL lines did **not** reproduce, and the
isolation rules in §3 are the reason it did not.

### How the run was made deterministic
* One snapshot, one build, **no rebuild between the two passes**.
* **Strictly sequential**: one process at a time, in alphabetical suite order, 142 suites
  per pass, 284 process launches total, 600 s watchdog per suite. Nothing was parallel.
* **`stdout` and `stderr` captured to SEPARATE files** via
  `Start-Process -RedirectStandardOutput … -RedirectStandardError …` (raw-byte
  redirection). `2>&1` was never used. Sanity check: the longest captured build line was
  **1 225 characters** and the longest stderr line **179 characters** — far above the
  119-character truncation threshold, so no line was cut. Japanese text in FAIL messages
  round-trips correctly (verified by decoding the captures as UTF-8 in Python; the
  mojibake you see if you `Get-Content` these files from PowerShell 5.1 is PowerShell's
  ANSI display, not the file).
* **No suite ever shared a `tests/*.log`.** All 142 stale `tests/*.log` files and all 62
  stale `tests/_*.out` / `_*.err` captures were deleted from the snapshot before the
  build. `make run-all-tests` was never invoked.

---

## 3. SNAPSHOT

Taken from the live tree at the start of the session, excluding **all** `*.o`,
**all** `rb_engine*` binaries, **all** `tests/*.log`, and the stale `_*.out`/`_*.err`
captures. Live tree: 626 files → snapshot: 319 files. Exactly 103 files were excluded
(49 `.o` + 54 `rb_engine*`), and 204 further files were stale captures/`.exe` debris
removed after the copy. **`make clean` was then run and exited 0.**

The snapshot is a **point-in-time copy of a tree other agents are still editing.** The
manifest is the only thing that pins what was measured. It is quoted in full in
`BASELINE_manifest_sha256.txt`. If the live tree's manifest differs from it, this is a
*different* measurement and must be reported as one.

Git state observed read-only at snapshot time: `HEAD = f152a50f`
(`fix(engine): 「これにより…」 counts the cards this ability moved`), with
`3cffff64`, `0ba3a112`, `61970a60`, `9109d17f`, `cf7731a2`, `077df271`, `f1d1c926` behind
it, and 28 modified tracked files including `engine_c_wip/Makefile`,
`engine_c_wip/include/rabuka.h`, `engine_c_wip/src/ability/effects/custom.c`,
`engine_c_wip/tests/replay.c` and `engine_c_wip/tests/test_parity_wave1b.c`.

---

## 4. DATA BLOB — measured against

`engine_c_wip/src/abilities_strings.bin`, verified at **three checkpoints**
(before the first build, after `make all` + `make web`, and after **both** test passes):

```
bytes  = 726831
sha256 = 89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344
```

**Unchanged at all three checkpoints. The measurement is not void.**

`src/cards.bin`: `631890` bytes, `728208504fee62171429ebb7bbdd8e12263e1c03b816c947c6fc98aa2676fac4`.

`make data` was made a green no-op by **seeding the snapshot's `../cards/build/` with the
canonical blobs** — `cards.bin` copied from the live tree, `abilities_strings.bin` copied
from the live `engine_c_wip/src/` (the *good* 726 831-byte copy), never from the stale
`cards/build/`. `make data` exit **0**, and both guards reported:

```
data guard: ../cards/build/cards.bin == src/cards.bin (631890 bytes, sha256 728208504fee6217…), already installed.
data guard: ../cards/build/abilities_strings.bin == src/abilities_strings.bin (726831 bytes, sha256 89204df0bbde08a0…), already installed.
```

For contrast, the **live** tree's `cards/build/abilities_strings.bin` is still
**726 649** bytes / `6225b9b531f4875a…`, so `make data` there remains RED by design.
That refusal is correct and is not breakage. It was bypassed by seeding, not by
weakening the guard.

---

## 5. BUILD

| step | exit | errors | warnings |
|---|---|---|---|
| `make clean` | **0** | 0 | 0 |
| `make all` (run 1) | **0** | **0** | **120** |
| `make all` (run 2, after a second `make clean`) | **0** | **0** | **120** |
| `make web` | **0** | **0** | **0** |
| `make data` | **0** | 0 | 0 |

`make all` was executed **twice from scratch** and the two stderr captures are
**identical line for line** (`Compare-Object` difference = **0**, 574 lines each). The
build is deterministic.

Artifacts: `rb_engine.exe` = 1 336 611 B, `rb_web_server.exe` = 1 382 786 B.

**120 warnings, 0 errors.** By file:

| n | file | n | file |
|---|---|---|---|
| 25 | `src/ability/util.c` | 5 | `src/ability/effects/move.c` |
| 16 | `src/ability/effects/look.c` | 4 | `src/ability/condition.c` |
| 12 | `src/ability/choice.c` | 3 | `src/core/data.c` |
| 10 | `src/ability/effects/rules.c` | 3 | `src/turn/triggers.c` |
| 9 | `src/ability/effects/state.c` | 2 | `src/game/game_setup.c` |
| 8 | `src/core/game_state_abilities.c` | 2 | `src/core/game_state_core.c` |
| 6 | `src/ability/types.c` | 2 | `src/ability/effects/draw.c` |
| 6 | `src/ability/cost.c` | 2 | `src/ability/effects/misc.c` |
| | | 1 each | `zones.c`, `ability_effects.c`, `vm.c`, `resolver.c`, `move_missing.c` |

By category: `-Wstringop-truncation` 41, `-Wunused-function` 29, unclassified 16,
`-Wunused-variable` 13, `-Wunused-but-set-variable` 6, `-Wcomment` 4, `-Wrestrict` 3,
`-Wmisleading-indentation` 2, `-Wdiscarded-qualifiers` 2, `-Waddress` 2,
`-Wparentheses` 1.

All 142 suites compiled: **BUILD_RED = 0**, zero `error:` and zero
`undefined reference` lines across all 142 compile captures.

---

## 6. PER-SUITE TABLE (run 1; run 2 is identical)

Harness shapes, established by reading the 142 sources then confirmed against output:

* **FORK** (72 files) — forks a child per test; the parent prints one verdict per test.
  `tests`/`ok`/`FAILED`/`CRASH` are per-test and directly comparable.
* **FLAT** (59) — single `main()`, in-process `CHECK`; prints `ok: <msg>` per passing
  *assertion*. It never enumerates tests, so `tests` is 0 by construction and
  `assertions ok` is the meaningful column.
* **COUNTERS** (6) — prints explicit tallies.
* **DIAG** (5) — no assertion macro at all.

| suite | shape | tests | ok | FAILED | CRASH | FAIL lines | assertions ok | ms | colour |
|---|---|---|---|---|---|---|---|---|---|
| `ability_effects` | FLAT | 0 | 0 | 0 | 0 | 0 | 12 | 151 | GREEN |
| `ability_effects_crash` | FORK | 9 | 9 | 0 | 0 | 0 | 0 | 830 | GREEN |
| `artifacts_consistent` | DIAG | 0 | 0 | 0 | 0 | 0 | 0 | 125 | GREEN_UNVERIFIED |
| `audit_actions` | FORK | 4 | 4 | 0 | 0 | 2 | 39 | 632 | RED |
| `audit_conditions` | FORK | 12 | 7 | 5 | 0 | 2 | 5 | 1104 | RED |
| `audit_fields` | FORK | 16 | 14 | 2 | 0 | 3 | 0 | 997 | RED |
| `audit_triggers` | FORK | 0 | 0 | 0 | 0 | 5 | 4 | 594 | RED |
| `audit_zones` | FORK | 13 | 13 | 0 | 0 | 3 | 152 | 1227 | RED |
| `b8_live_timing` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 255 | GREEN |
| `baton_touch` | FLAT | 0 | 0 | 0 | 0 | 0 | 18 | 266 | GREEN |
| `condition_evaluation` | FLAT | 0 | 0 | 0 | 0 | 3 | 23 | 1196 | RED |
| `cross_player_interaction` | FLAT | 0 | 0 | 0 | 0 | 1 | 20 | 231 | RED |
| `deck_builder` | FLAT | 0 | 0 | 0 | 0 | 0 | 39 | 118 | GREEN |
| `deck_parser` | FLAT | 0 | 0 | 0 | 0 | 0 | 29 | 105 | GREEN |
| `decode_fallback` | DIAG | 0 | 0 | 0 | 0 | 0 | 1 | 127 | GREEN |
| `decode_skew` | DIAG | 16 | 16 | 0 | 0 | 0 | 0 | 275 | GREEN |
| `draw_phase_fix` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 302 | **NO_OUTPUT** |
| `live_allocator` | FLAT | 0 | 0 | 0 | 0 | 0 | 38 | 468 | GREEN |
| `live_card_zone_movement` | FLAT | 0 | 0 | 0 | 0 | 0 | 41 | 452 | GREEN |
| `live_success_rules` | FLAT | 0 | 0 | 0 | 0 | 0 | 73 | 1562 | GREEN |
| `log_drain` | FLAT | 0 | 0 | 0 | 0 | 0 | 60 | 197 | GREEN |
| `look_select` | FLAT | 0 | 0 | 0 | 0 | 2 | 47 | 348 | RED |
| `max_distinct_names` | FLAT | 1 | 1 | 0 | 0 | 6 | 4043 | 931 | RED |
| `mechanics` | FLAT | 0 | 0 | 0 | 0 | 0 | 30 | 842 | GREEN |
| `member_activation_debut_live_start` | FLAT | 0 | 0 | 0 | 0 | 0 | 17 | 866 | GREEN |
| `move_cards` | FLAT | 0 | 0 | 0 | 0 | 2 | 14 | 183 | RED |
| `opponent_live_success_flow` | FLAT | 0 | 0 | 0 | 0 | 0 | 4 | 434 | GREEN |
| `p1_helpers` | FLAT | 0 | 0 | 0 | 0 | 1 | 0 | 283 | RED |
| `parity_ability_mod` | FLAT | 0 | 0 | 0 | 0 | 14 | 76 | 978 | RED |
| `parity_ability_mod_extra` | FLAT | 0 | 0 | 0 | 0 | 17 | 188 | 2753 | RED |
| `parity_ability_watch` | FORK | 32 | 23 | 9 | 0 | 20 | 291 | 3842 | RED |
| `parity_abilitymod_conditional` | FORK | 6 | 1 | 5 | 0 | 8 | 758 | 5642 | RED |
| `parity_area_move` | FORK | 40 | 19 | 21 | 0 | 32 | 0 | 8223 | RED |
| `parity_auto_abilities` | FLAT | 1 | 1 | 0 | 0 | 1 | 87 | 234 | RED |
| `parity_blade_under` | FORK | 47 | 27 | 20 | 0 | 30 | 0 | 7697 | RED |
| `parity_blades2` | FORK | 28 | 18 | 10 | 0 | 16 | 0 | 3226 | RED |
| `parity_blades_live_start` | FLAT | 1 | 1 | 0 | 0 | 34 | 199 | 1620 | RED |
| `parity_bp7` | FORK | 29 | 16 | 13 | 0 | 24 | 0 | 4543 | RED |
| `parity_card_identity` | FLAT | 0 | 0 | 0 | 0 | 0 | 151 | 463 | GREEN |
| `parity_card_score` | FORK | 0 | 0 | 0 | 0 | 21 | 301 | 2785 | RED |
| `parity_cce_gaps` | FORK | 68 | 30 | 37 | **1** | 53 | 0 | 12622 | RED |
| `parity_char2` | FORK | 21 | 19 | 2 | 0 | 3 | 121 | 4194 | RED |
| `parity_characterization` | FLAT | 0 | 0 | 0 | 0 | 3 | 0 | 574 | RED |
| `parity_characterization_extra` | FORK | 54 | 35 | 19 | 0 | 26 | 0 | 15047 | RED |
| `parity_compound_per_card` | FORK | 47 | 24 | 23 | 0 | 41 | 0 | 7029 | RED |
| `parity_condition_and_effect` | FLAT | 0 | 0 | 0 | 0 | 52 | 339 | 1509 | RED |
| `parity_conditional_gated` | FORK | 26 | 0 | 26 | 0 | 46 | 288 | 8638 | RED |
| `parity_conditions` | DIAG | 0 | 0 | 0 | 0 | 0 | 0 | 601 | GREEN_UNVERIFIED |
| `parity_cost_and_effect` | FLAT | 1 | 1 | 0 | 0 | 1 | 14 | 315 | RED |
| `parity_cost_compound` | FLAT | 1 | 1 | 0 | 0 | 0 | 99 | 248 | GREEN |
| `parity_cost_discount` | FORK | 86 | 56 | 30 | 0 | 47 | 5 | 15344 | RED |
| `parity_custom_misc` | FLAT | 0 | 0 | 0 | 0 | 2 | 79 | 320 | RED |
| `parity_deploy_move_missing` | FLAT | 0 | 0 | 0 | 0 | 7 | 66 | 432 | RED |
| `parity_deploy_multi` | FORK | 22 | 11 | 11 | 0 | 21 | 0 | 2988 | RED |
| `parity_deploy_recover` | COUNTERS | 0 | 0 | 0 | 0 | 24 | 305 | 1542 | RED |
| `parity_draw_delta` | FLAT | 1 | 1 | 0 | 0 | 0 | 152 | 429 | GREEN |
| `parity_draw_flat` | FLAT | 0 | 0 | 0 | 0 | 74 | 533 | 1939 | RED |
| `parity_draw_until_count` | FLAT | 0 | 0 | 0 | 0 | 0 | 69 | 368 | GREEN |
| `parity_energy_position_delta` | FORK | 7 | 3 | 4 | 0 | 6 | 0 | 706 | RED |
| `parity_energy_remainder` | FORK | 7 | 4 | 3 | 0 | 4 | 0 | 847 | RED |
| `parity_enums` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 182 | GREEN_UNVERIFIED |
| `parity_executor_dispatch` | FLAT | 0 | 0 | 0 | 0 | 0 | 118 | 534 | GREEN |
| `parity_full_round` | FORK | 36 | 21 | 15 | 0 | 29 | 0 | 5188 | RED |
| `parity_full_round2` | FORK | 27 | 13 | 14 | 0 | 53 | 0 | 2201 | RED |
| `parity_full_round3` | FORK | 25 | 20 | 5 | 0 | 12 | 0 | 3891 | RED |
| `parity_gain_delta` | FORK | 16 | 14 | 2 | 0 | 2 | 0 | 1318 | RED |
| `parity_hearts_constants` | FLAT | 0 | 0 | 0 | 0 | 19 | 137 | 493 | RED |
| `parity_hearts_live_start` | FLAT | 0 | 0 | 0 | 0 | 64 | 132 | 1419 | RED |
| `parity_int_gaps` | FORK | 15 | 4 | 11 | 0 | 25 | 0 | 1963 | RED |
| `parity_integration` | FLAT | 1 | 1 | 0 | 0 | 31 | 131 | 3798 | RED |
| `parity_integration_extra` | FORK | 47 | 25 | 22 | 0 | 47 | 0 | 9788 | RED |
| `parity_integration_rounds` | FORK | 10 | 4 | 6 | 0 | 19 | 0 | 1118 | RED |
| `parity_jidou` | FLAT | 6 | 6 | 0 | 0 | 26 | 70 | 689 | RED |
| `parity_jidou_extra` | FLAT | 0 | 0 | 0 | 0 | 25 | 87 | 605 | RED |
| `parity_jidou_life` | FORK | 16 | 7 | 9 | 0 | 22 | 207 | 2083 | RED |
| `parity_jidou_move_yell` | FORK | 88 | 48 | 40 | 0 | 49 | 0 | 17834 | RED |
| `parity_jidou_state` | FORK | 12 | 0 | 12 | 0 | 17 | 147 | 11218 | RED |
| `parity_jidou_watch` | FORK | 59 | 36 | 23 | 0 | 56 | 234 | 8117 | RED |
| `parity_jidou_watch2` | FORK | 31 | 9 | 22 | 0 | 46 | 306 | 3369 | RED |
| `parity_jidou_watch_clusters` | FORK | 35 | 20 | 15 | 0 | 47 | 370 | 5417 | RED |
| `parity_live_total` | FORK | 20 | 0 | 20 | 0 | 33 | 184 | 5524 | RED |
| `parity_live_total2` | FORK | 1 | 1 | 0 | 0 | 10 | 271 | 4719 | RED |
| `parity_look_and_filter` | FORK | 65 | 49 | 16 | 0 | 27 | 0 | 5881 | RED |
| `parity_look_per_card` | FORK | 48 | 25 | 23 | 0 | 37 | 0 | 4552 | RED |
| `parity_look_reveal` | COUNTERS | 0 | 0 | 0 | 0 | 1 | 151 | 551 | RED |
| `parity_mill_deck` | FORK | 61 | 31 | 30 | 0 | 53 | 109 | 8395 | RED |
| `parity_parser_e2e_extra` | FORK | 17 | 11 | 6 | 0 | 8 | 0 | 1815 | RED |
| `parity_per_card` | FORK | 21 | 16 | 5 | 0 | 15 | 0 | 1755 | RED |
| `parity_per_card2` | FORK | 0 | 0 | 0 | 0 | 10 | 0 | 3110 | RED |
| `parity_per_card3` | FORK | 21 | 12 | 9 | 0 | 13 | 0 | 2146 | RED |
| `parity_queue_resume` | FLAT | 0 | 0 | 0 | 0 | 1 | 0 | 236 | RED |
| `parity_realcard_walkthrough` | FORK | 8 | 6 | 2 | 0 | 5 | 0 | 1114 | RED |
| `parity_recover_hand_a` | FORK | 22 | 13 | 9 | 0 | 17 | 0 | 1825 | RED |
| `parity_recover_live` | FORK | 63 | 42 | 21 | 0 | 35 | 0 | 6073 | RED |
| `parity_recover_q79` | FORK | 24 | 14 | 10 | 0 | 6 | 0 | 1631 | RED |
| `parity_recover_score_state_delta` | FORK | 18 | 0 | 18 | 0 | 28 | 147 | 2782 | RED |
| `parity_recover_to_hand` | COUNTERS | 0 | 0 | 0 | 0 | 10 | 103 | 464 | RED |
| `parity_required_hearts` | FORK | 57 | 45 | 12 | 0 | 16 | 180 | 9682 | RED |
| `parity_restriction` | FORK | 31 | 12 | 19 | 0 | 43 | 95 | 4525 | RED |
| `parity_restriction_invalidate` | FORK | 6 | 6 | 0 | 0 | 0 | 0 | 557 | GREEN |
| `parity_reveal_extra` | COUNTERS | 0 | 0 | 0 | 0 | 20 | 440 | 1961 | RED |
| `parity_rules` | FLAT | 3 | 3 | 0 | 0 | 11 | 157 | 428 | RED |
| `parity_rules_conditions` | FORK | 5 | 2 | 3 | 0 | 6 | 114 | 1246 | RED |
| `parity_rules_conditions2` | FORK | 0 | 0 | 0 | 0 | 0 | 133 | 1276 | GREEN |
| `parity_rules_extra` | FORK | 16 | 14 | 2 | 0 | 4 | 0 | 764 | RED |
| `parity_rules_phases` | FORK | 15 | 5 | 10 | 0 | 18 | 205 | 6584 | RED |
| `parity_score_effects` | FLAT | 2 | 2 | 0 | 0 | 1 | 107 | 168 | RED |
| `parity_score_per_card2` | FORK | 0 | 0 | 0 | 0 | 1 | 124 | 1847 | RED |
| `parity_search_reorder` | FORK | 79 | 45 | 34 | 0 | 122 | 227 | 10065 | RED |
| `parity_sequential` | FORK | 52 | 23 | 29 | 0 | 113 | 0 | 9094 | RED |
| `parity_small_clusters` | FORK | 43 | 1 | 42 | 0 | 70 | 841 | 18564 | RED |
| `parity_state_effects2` | COUNTERS | 0 | 0 | 0 | 0 | 3 | 99 | 541 | RED |
| `parity_state_per_card` | FORK | 82 | 48 | 34 | 0 | 46 | 0 | 12917 | RED |
| `parity_stats_pipeline` | FLAT | 2 | 2 | 0 | 0 | 31 | 57 | 207 | RED |
| `parity_support` | FORK | 13 | 12 | 1 | 0 | 1 | 0 | 1083 | RED |
| `parity_target_player` | FORK | 46 | 17 | 29 | 0 | 54 | 0 | 5932 | RED |
| `parity_trigger_paths` | FORK | 45 | 30 | 15 | 0 | 23 | 282 | 4054 | RED |
| `parity_under_member` | FORK | 83 | 52 | 31 | 0 | 60 | 0 | 13353 | RED |
| `parity_wait_activation` | COUNTERS | 0 | 0 | 0 | 0 | 41 | 249 | 2684 | RED |
| `parity_wave1a` | FORK | 21 | 8 | 13 | 0 | 23 | 0 | 2875 | RED |
| `parity_wave1b` | FORK | 31 | 17 | 14 | 0 | 39 | 0 | 3472 | RED |
| `parity_wave1c` | FORK | 20 | 10 | 10 | 0 | 31 | 0 | 2307 | RED |
| `parity_yell` | FORK | 15 | 1 | 14 | 0 | 39 | 0 | 3210 | RED |
| `parity_zone_source` | FORK | 29 | 0 | 29 | 0 | 51 | 434 | 9734 | RED |
| `parity_zones_position` | FLAT | 0 | 0 | 0 | 0 | 38 | 138 | 616 | RED |
| `performance_phase_rules` | FLAT | 0 | 0 | 0 | 0 | 2 | 36 | 1728 | RED |
| `phase_machine_rules` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 629 | GREEN |
| `ported_generated` | FLAT | 0 | 0 | 0 | 0 | 3 | 78 | 550 | RED |
| `position_and_source` | FORK | 17 | 17 | 0 | 0 | 0 | 0 | 785 | GREEN |
| `restriction_mechanics` | FLAT | 0 | 0 | 0 | 0 | 0 | 18 | 246 | GREEN |
| `state_effects` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 217 | GREEN_UNVERIFIED |
| `surplus_heart` | FLAT | 0 | 0 | 0 | 0 | 1 | 12 | 489 | RED |
| `survey_probe` | FORK | 4 | 1 | 3 | 0 | 5 | 0 | 451 | RED |
| `target_selection` | FLAT | 0 | 0 | 0 | 0 | 3 | 31 | 324 | RED |
| `tp_activation_offer` | FLAT | 0 | 0 | 0 | 0 | 3 | 3 | 316 | RED |
| `tp_joint_card_live_start` | FLAT | 0 | 0 | 0 | 0 | 21 | 33 | 729 | RED |
| `tp_turn_limited_activation` | FLAT | 0 | 0 | 0 | 0 | 0 | 15 | 313 | GREEN |
| `tp_zone_change_gate` | FLAT | 0 | 0 | 0 | 0 | 13 | 8 | 335 | RED |
| `trigger_paths` | FLAT | 0 | 0 | 0 | 0 | 19 | 121 | 472 | RED |
| `vm_decode` | FLAT | 0 | 0 | 0 | 0 | 0 | 35 | 241 | GREEN |
| `zone_filter_parity` | FLAT | 0 | 0 | 0 | 0 | 0 | 45 | 214 | GREEN |
| `zzprobe` | DIAG | 0 | 0 | 0 | 0 | 0 | 0 | 198 | GREEN_UNVERIFIED |

`tests`/`ok`/`FAILED`/`CRASH` are per-test and only meaningful for FORK and COUNTERS
shapes. For FLAT suites read `FAIL lines` and `assertions ok` instead.

---

## 7. HOW RED/GREEN WAS DECIDED — AND WHY EXIT CODES WERE IGNORED

Exit codes were **not used at any point**. They are not a signal in this corpus, and
this run demonstrates exactly why:

* `parity_conditional_gated` — **26 / 26 tests FAILED**, 46 FAIL lines, and it still
  finishes.
* `parity_under_member` (`tests/test_parity_under_member.c`) — carries **60** FAIL
  lines.
* `parity_live_total` — 20 / 20 FAILED. `parity_jidou_state` — 12 / 12 FAILED.
  `parity_small_clusters` — 42 of 43 FAILED. `parity_recover_score_state_delta` — 18 / 18.
  `parity_conditional_gated`, `parity_live_total`, `parity_jidou_state` and
  `parity_recover_score_state_delta` are suites whose *every* enumerated test fails.

Red is decided on **failure evidence only**: any `FAILED` verdict line, any `CRASH`
verdict line, any `SETUPBUG` verdict line, or any `FAIL`-prefixed assertion line.
A suite with none of those and which produced output is GREEN.

### The 6 suites that cannot be classified from their own output
| suite | what it actually emitted | classification |
|---|---|---|
| `artifacts_consistent` | `=== 12 checks, 0 failed ===` / `RESULT: consistent. 6032 interned strings, 726831 bytes, all digests match.` | **GREEN, verified.** The artifact gate is clean at the canonical blob. |
| `parity_enums` | `parity_enums: 7191 assertions, 0 failures` | **GREEN, verified.** |
| `parity_conditions` | 53 stdout + 77 stderr lines of `DIAG`/`trace` dumps. Contains **no assertion macro at all**. | diagnostic tool, not a test |
| `zzprobe` | 172 stdout lines: key histogram + `decode_fallbacks=0 (delta)` | diagnostic tool, not a test |
| `state_effects` | **0 bytes stdout**, 7 stderr lines, all `RESUME_*` / `CHOICE_RESUME_END` resolver traces. **No verdict line of any kind.** | ran; reports nothing |
| `draw_phase_fix` | **0 bytes on both streams** | see §8 — cannot report red at all |

---

## 8. BUGS FOUND — REPORTED, NOT FIXED

I edited no source. Every item below is a measurement-authority finding for the fixer
agents.

### 8.1 `tests/test_draw_phase_fix.c:6-13` — the suite is structurally incapable of reporting red
```c
#define ASSERT_EQ(actual, expected, message) do { \
    ...
        fprintf(stderr, "%s: got %d, expected %d\n", message, actual_value, expected_value); \
        return; \            /* <-- returns from the test fn, prints no FAIL, bumps no counter */
```
The macro emits **no `FAIL` prefix**, **increments no failure counter**, and `return`s
early. Its `main()` prints nothing on success. Measured result: **0 bytes stdout, 0 bytes
stderr** — indistinguishable from a silent pass. Every other assertion in this corpus is
invisible to a FAIL-line count because of this one macro. A failure here would be
reported as *green*. This is a measurement hole, not a red suite.

### 8.2 `tests/test_parity_cce_gaps.c:3601` — engine abort (SIGABRT / signal 6)
`run("a22b_resolver_path_pays", a22_resolver_pays_cost);`
Captured: `*** FATAL SIGNAL 6 inside test: a22b_resolver_path_pays` and
`CRASH    a22b_resolver_path_pays  <-- unexpected child exit 134`.
The child's own message says "The engine, not this test, is at fault." Reproducible
identically in both passes.

### 8.3 `tests/test_parity_recover_hand_a.c:1058` — engine segfault (SIGSEGV / signal 11)
The test named `test_f_zero_success_card_does_not_retrieve` dies with
`*** FATAL SIGNAL 11 inside test: test_f_zero_success_card_does_not_retrieve`.
**This crash is NOT counted in the `CRASH` verdict total** — that suite's harness emits
`FAILED: <name> (N failures)` rather than `CRASH <name>`, so the segfault is silently
folded into an ordinary failure count. There are therefore **2 engine faults, not 1**.

### 8.4 47 suites carry FAIL lines but report ZERO failed tests
**683 FAIL assertion lines across 47 suites** have no per-test `FAILED` verdict
anywhere in their output — the harness either does not enumerate tests (FLAT/COUNTERS) or
fails to propagate the child's status. The largest: `parity_search_reorder` 122,
`parity_sequential` 113, `parity_hearts_live_start` 64, `parity_mill_deck` 53,
`parity_condition_and_effect` 52, `parity_restriction` 43.

Consequence: **"failed tests" is not a well-defined number in this corpus.** 949 is what
the FORK/COUNTERS harnesses report. The true figure is bounded by
**949 ≤ failed tests ≤ 949 + 683**.

### 8.5 `sizeof(GameState)` is 885 912 bytes, not the ~780 KB the Makefile claims
Measured with a throwaway probe in the snapshot (not added to the tree):
```
sizeof(GameState)       = 885912 bytes
sizeof(TestGame)        = 885912 bytes
sizeof(RbLiveSnapshot)  = 1360 bytes
RB_MAX_LIVE_CARDS       = 3
```
`Makefile:3-5` says "sizeof(GameState) is ~780 KB". It is **865 KB**. Two of them is
1 771 824 bytes, which overflows the 1 MB Windows stack — the build only survives because
every link target carries `-Wl,--stack,16777216` (`Makefile:6`). That flag is
load-bearing, not cosmetic.

**1 001 non-static `GameState`/`TestGame` local declarations remain across 58 test
files** (largest: `test_parity_card_score.c` 74, `test_parity_hearts_constants.c` 67,
`test_parity_wait_activation.c` 66, `test_parity_characterization_extra.c` 54,
`test_parity_score_effects.c` 51). Checklist item J8 converted 230 of these in five
files; the sweep was never finished. Several functions hold two at once
(`test_parity_jidou_extra.c:900` and `:906`, `test_parity_stats_pipeline.c:340`,
`test_parity_reveal_extra.c:995`, `test_parity_queue_resume.c:312`) — those are the
1.7 MB stack frames.

### 8.6 Three diagnostic prefixes a naive FAIL scan misses
`SETUP BUG` (5 lines) and `VACUOUS` (21 lines) are emitted on stderr but are not `FAIL`
lines, so a `grep '^FAIL'` tally scores them zero. They mean the opposite of green:
* `VACUOUS` in `parity_required_hearts` (13) and `parity_state_per_card` (8) says a
  passing assertion was **not proven non-vacuous** — its paired positive is red, so the
  zero proves nothing.
* `SETUP BUG` in `audit_conditions` (3), `parity_blades_live_start` (2) means the
  fixture was never built, so the test asserted nothing.
One is a wrong-card defect: `SETUP BUG: expected card PL!HS-pb6-002-R, resolved id=-1 (<none>)`.

### 8.7 There is no third FAIL shape, and zero FAIL lines on stdout
`artifacts_consistent` and `decode_skew` *can* print `  FAIL  …` to **stdout**, but
neither did (the artifact gate is clean). Recorded because a future measurement must
watch both streams.

---

## 9. PERFORMANCE — NOTHING HANGS

**0 hangs of 142**, 0 timeouts, 0 suites that lost their output. Slowest 15:

| suite | run 1 | FAIL lines |
|---|---|---|
| `parity_small_clusters` | 18.564 s | 70 |
| `parity_jidou_move_yell` | 17.834 s | 49 |
| `parity_cost_discount` | 15.344 s | 47 |
| `parity_characterization_extra` | 15.047 s | 26 |
| `parity_under_member` | 13.353 s | 60 |
| `parity_state_per_card` | 12.917 s | 46 |
| `parity_cce_gaps` | 12.622 s | 53 |
| `parity_jidou_state` | 11.218 s | 17 |
| `parity_search_reorder` | 10.065 s | 122 |
| `parity_integration_extra` | 9.788 s | 47 |
| `parity_zone_source` | 9.734 s | 51 |
| `parity_required_hearts` | 9.682 s | 16 |
| `parity_sequential` | 9.094 s | 113 |
| `parity_conditional_gated` | 8.638 s | 46 |
| `parity_mill_deck` | 8.395 s | 53 |

Nothing is near the 600 s watchdog. `max_distinct_names` emits **4 043** passing-assertion
lines in 931 ms and is the only suite whose stdout dwarfs its FAIL count.

---

## 10. STALE EXPECTATIONS vs GENUINE DIVERGENCE

An honest mechanical split, plus its limits.

| bucket | FAIL lines | share |
|---|---|---|
| in a suite whose own source **declares an expected gap / known C-side gap** (`EXPECTED_GAP`, "gap closed", "RED as pinned", "no C counterpart", …) | **1 549** | 61.2 % |
| in a suite with **no such declaration** | **981** | 38.8 % |
| **total** | **2 530** | 100 % |

53 suites declare a gap; they carry 1 549 FAIL lines. Top contributors:
`parity_search_reorder` (8 declarations / 122 lines), `parity_sequential` (3 / 113),
`parity_small_clusters` (1 / 70), `parity_under_member` (4 / 60), `parity_jidou_watch`
(6 / 56), `parity_target_player` (2 / 54), `parity_cce_gaps` (5 / 53), `parity_full_round2`
(1 / 53), `parity_mill_deck` (9 / 53), `parity_zone_source` (1 / 51).

**Limits of this split, stated plainly:**
* "declares an expected gap" means the C file *says* the divergence is known. It does
  **not** prove the expectation is stale. Many of these are genuine engine gaps that
  were documented rather than fixed. This is an upper bound on "stale", not a count of it.
* Only **2** FAIL lines in the whole tree carry an inline citation of a Rust twin
  `*_test.rs:line`, so the "the test cites Rust and Rust disagrees" signal is nearly
  absent and cannot carry the classification.
* The real stale-vs-genuine verdict needs a human reading each assertion against its Rust
  twin. Cluster J already closed five of the former (J4 ×2, J10 ×2, plus the
  `R1_shared_pr_debut_waits_low_cost_opponents` fix). The remaining work is exactly this.
* The 26 `SETUP BUG` + `VACUOUS` lines (§8.6) are a **third** category — not stale, not
  divergent, but *unproven*. They should not be counted as either.

**Rough split offered for triage:** of 2 530, treat **≈1 549 as "known/documented, needs
a decision"** and **≈981 as undocumented divergence** (the honest candidate list for
genuine engine work), and keep **26 aside as unproven**.

---

## 11. REGRESSION WATCH — AND A COMPLETE RECONCILIATION

Three historical figures exist. They are **not comparable at face value**, and the
difference is fully explained.

### 11.1 Versus the task's stated baseline — 3 571 FAIL / 1 139 failed tests / 109 red / 33 green
| | baseline | measured | delta |
|---|---|---|---|
| FAIL assertions | 3 571 | **2 530** | **−1 041** |
| failed tests | 1 139 | 949 (harness-reported; true value ≤ 1 632) | **−190 at best** |
| red suites | 109 | **108** | **−1** |
| green suites | 33 | 34 (28 verified + the 6 no-evidence) | +1 |

The −1 041 cannot be attributed: the suite corpus and engine have both changed since
that figure was taken, and no snapshot of that state exists to re-measure. I will not
guess at it. The **red/green** figure, which is the one that survives, moved by **one
suite**.

### 11.2 Versus the `PARITY_CHECKLIST.md` G/H closure table — 2 348 FAIL lines / 98 red / 44 green
This one reconciles **exactly**, and the reason is a measurement-scope difference, not a
regression.

`PARITY_CHECKLIST.md` records **2 348** "FAIL lines in stderr" and **98 RED / 44 GREEN**.
My stderr FAIL total is 2 530, which splits cleanly:

```
FAIL:   (space shape)  ->  2350
FAIL [  (bracket shape) ->   180
                          -----
                          2530
```

The G/H figure of 2 348 is a match for the **`FAIL: ` shape alone** (2 350), not for the
total. And a scanner that only matches `FAIL: ` **cannot see** the 180 `FAIL [` lines,
which are emitted by exactly **10 suites** whose entire failure output uses the bracket
shape:

| suite | `FAIL [` lines |
|---|---|
| `parity_full_round2` | 53 |
| `parity_integration_extra` | 47 |
| `parity_full_round` | 29 |
| `parity_integration_rounds` | 19 |
| `parity_full_round3` | 12 |
| `parity_energy_position_delta` | 6 |
| `parity_support` | 1 |
| `audit_triggers` | 5 |
| `audit_fields` | 3 |
| `parity_realcard_walkthrough` | 5 |
| **total** | **180** |

So: **108 RED − 10 bracket-only suites = 98 RED** ✓ and **28 GREEN + 10 = 38**… against
their 44, the remaining 6 is exactly my 5 GREEN_UNVERIFIED + 1 NO_OUTPUT
(`artifacts_consistent`, `parity_conditions`, `parity_enums`, `state_effects`, `zzprobe`,
`draw_phase_fix`) — suites that emit no verdict line a scanner can classify, which their
harness-based counter would have credited as green. 28 + 10 + 6 = **44** ✓

**Every one of the 44 reconciles.** There was no regression. The previous number was a
strictly narrower measurement; mine is a superset of it.

### 11.3 Independent cross-check — 7 of 7 recorded per-suite figures match EXACTLY
`PARITY_CHECKLIST.md` records per-suite FAIL counts for seven suites. All seven reproduce
**exactly**:

| suite | checklist | measured |
|---|---|---|
| `parity_zones_position` (J6) | 38 | **38** |
| `parity_small_clusters` (J10) | 70 | **70** |
| `parity_cce_gaps` (J4) | 53 | **53** |
| `parity_integration` (J7) | 31 | **31** |
| `parity_under_member` (J5) | 60 | **60** |
| `parity_wave1b` (J12) | 39 | **39** |
| `parity_executor_dispatch` (J2) | 0 | **0** |

Seven for seven. The engine has not moved in any way these suites can detect, which is
consistent with §11.2: the entire apparent delta is counting scope.

### 11.4 The residual +2
On the identical `FAIL: `-only scope, 2 350 vs 2 348 is **+2 lines**. The tree did move
(`HEAD = f152a50f`, "「これにより…」 counts the cards this ability moved", plus
`3cffff64`, and 28 modified tracked files). I cannot attribute those 2 lines to a
specific commit without the old snapshot, so I am not going to. It is +2 out of 2 348 —
0.08 %.

---

## 12. WHAT I DID NOT DO

* Did not modify any `.c`, `.h`, `Makefile`, `.sh`, `.ps1`, `.py` or test file.
* Did not run `make run-all-tests`, did not weaken/ignore/skip any test, did not run any
  generator, did not bypass the `tools/guard_data_blob.sh` refusal in the live tree.
* Did not perform any git write. No stash pop/apply/drop, no `checkout --`, no `restore`,
  no `reset --hard`, no commit. All git use was read-only.
* Did not read any checked-in `.log` as a result. All 142 stale `tests/*.log` were deleted
  from the snapshot and every number here comes from a fresh capture.
* Did not run the suite in `engine_c_wip`. Everything ran in
  `C:\Users\trios\AppData\Local\Temp\kilo\baseline\engine_c_wip`.

## 13. REPRODUCING THIS

```
snapshot   C:\Users\trios\AppData\Local\Temp\kilo\baseline\engine_c_wip
manifest   C:\Users\trios\AppData\Local\Temp\kilo\baseline\MANIFEST.sha256   (240 entries)
raw runs   C:\Users\trios\AppData\Local\Temp\kilo\baseline\logs\runs\run{1,2}_<suite>.{out,err}
build logs C:\Users\trios\AppData\Local\Temp\kilo\baseline\logs\{all2,web,clean2}.{out,err}
indices    …\logs\runs\run{1,2}_index.csv      (per-suite wall clock, line counts)
analysis   …\logs\{suite_table,ranking_run1}.json
driver     …\baseline\{measure_suites,run_suites,suite_table,rank2,classify}.{ps1,py}
```
Re-run `suite_table.py` then `rank2.py` to regenerate every number in this document.
If `MANIFEST.sha256` no longer matches the tree you are looking at, this is a different
measurement.
# COVERAGE HARNESS — the defects that invalidate every coverage number

**Author:** coverage audit (test-coverage cluster). **Date:** 2026-10-04.

## 0. HOW THIS WAS MEASURED

Every suite in the C corpus was **built and run in a snapshot**, never in the live tree.

* snapshot `C:\Users\trios\AppData\Local\Temp\kilo\cov\tree`, `robocopy` **excluding `*.o`,
  `rb_engine*`, `*.obj`, `*.log`**; verified afterwards: **0 `.o` files** in the snapshot and
  **0 in the live tree**.
* `src/abilities_strings.bin` verified **726 831 B / `89204df0bbde08a0…`** before the build, and
  again after the last suite. `src/cards.bin` present at **63 189 B** and loaded without incident
  in 159 suites (its digest was not independently pinned here; `test_artifacts_consistent` pins
  `abilities_strings.bin`, not this file).
* **`make data` was never run** (the in-repo generator refuses by design and `cards/build/` is
  726 649 B / STALE). The 50 `OBJ_LIB` objects were built once with `make all`; each suite was
  then linked with an explicit `rm -f "$bin"` first, so **no stale link** was possible.
* **Strictly sequential**, 159 suites, 1 028 s total, slowest 33 s.
* stdout and stderr captured **separately** for every suite, plus build diagnostics and the exit
  code. **The exit code is recorded but is not treated as a signal** (see §3).
* **No source file and no test file was edited.** Only the four `COVERAGE_*.md` reports were
  written into the repo.

| | value |
|---|---:|
| suites attempted | **159** = 156 `test_*.c` + `replay.c` + `debug_eli.c` + `tmp_list.c` |
| build failures | **0** |
| build warnings | **172**, across **44** suites |
| suites that ran | 159 |
| `ok:` lines printed | **18 273** |
| `FAIL` lines (stdout + stderr) | **3 486** |

⚠ The project's own figure of "**142** suites" is **wrong**: the corpus contains **156**
`test_*.c` files, of which `make run-all-tests` iterates all 156 and `make t` can reach all 159
(including the 3 non-`test_` names, by stem). A `FAIL` count quoted against 142 is counting
against a list that does not exist.

---

## 1. ⛔ SUITES THAT PRODUCE NO OUTPUT

A suite that prints nothing and exits 0 is **indistinguishable from a passing suite** to every
measurement in this project. Four suites are in that state.

### 1.1 `tests/tmp_list.c` — the worst one

```
stdout: 0 bytes    stderr: 0 bytes    exit: 0    build: OK (2 s)
```

**Completely silent, and reads as passing.** It is also not referenced by `make run-all-tests`'s
glob (`test_*.c`), so it is neither run nor reported as absent.

### 1.2 `tests/test_audit_conditions.c` — a 1 002-line suite that runs nothing

```
stdout: 0 bytes
stderr: 56 bytes = "load_cards failed" / "load_cards failed" / "cannot load card db"
exit:   2
```

It **never loads the card database**, so all **14** of its assertion-message literals are dead
code. A `^FAIL` scan scores it **0 failures out of 0** — a perfect score for zero coverage.

**Root cause is a known landmine:** `load_card_db()` falls back to `../cards/build` when
`rb_load("src")` fails, and `rb_load` is cwd-sensitive, so it fails under an isolated out-of-tree
build. The snapshot **does** contain `src/cards.bin`; the fix is to pass the right `rb_load`
directory, not to relax the check.

### 1.3 `tests/test_state_effects.c` — SIGSEGV, zero assertions

```
stdout: 0 bytes
stderr: 529 bytes, ending at
        [RESUME_EXEC] def=0xa00195ec0 action=energy_state_change is_cost=0 host=-1
exit:   139   (SIGSEGV)
```

33 assertion-message literals, none evaluated. The trace shows `target=0x0` on the final line — a
null `target` reaching `energy_state_change`, the same shape as the `-Waddress` cluster-W rows
W1/W2 (`choice.c:1083`, `choice.c:5037`, two always-true address comparisons). **This is a source
bug for the `effects/energy` owner, not a test bug.**

### 1.4 `tests/test_p1_helpers.c` — 249 message literals, zero output

```
stdout: 0 bytes    stderr: 551 bytes    exit: 1
```

Its entire result stream is on stderr, and the only failure line is
`FAIL: opponent spawn routes choice to opponent`. **249 literals, 1 reported failure.** A
stdout-only scan sees a suite that is 249× green.

### 1.5 The 17 suites that print zero `ok:` lines

Not all broken, but **17 suites have a private result vocabulary**, so a scan keyed on `ok:`
silently measures nothing for them:

```
tmp_list                        out=   0 B  err=   0 B  exit=0     <- §1.1
test_audit_conditions           out=   0 B  err=  56 B  exit=2     <- §1.2
test_state_effects              out=   0 B  err= 529 B  exit=139   <- §1.3
test_p1_helpers                 out=   0 B  err= 551 B  exit=1     <- §1.4
test_phase_machine_rules        out=  37 B  err=5401 B  exit=0
test_b8_live_timing             out=  33 B  err= 722 B  exit=0
test_audit_warnings             out=  36 B  err= 454 B  exit=0
test_describe_golden            out= 169 B  err=   0 B  exit=0
test_artifacts_consistent       out= 318 B  err=   0 B  exit=0
test_parity_enums               out= 163 B  err=   0 B  exit=0
test_parity_fw_effects          out= 735 B  err= 163 B  exit=0     <- exit 0 WITH a FAIL line
test_parity_characterization    out= 675 B  err= 218 B  exit=1
test_parity_conditions          out=2775 B  err=5823 B  exit=0
test_parity_t6_action_setup     out= 864 B  err= 180 B  exit=0
test_parity_fw_cost_cardtype    out= 738 B  err=   0 B  exit=0
test_decode_skew                out=2920 B  err=   0 B  exit=0
test_zzprobe                    out=8162 B  err=   0 B  exit=0     <- scratch file in circulation
debug_eli                       out= 301 B  err=  18 B  exit=0     <- scratch file in circulation
```

Two of these deserve their own note:

* **`test_phase_machine_rules`** prints `ALL PHASE MACHINE RULE CHECKS PASSED` — a **PASSED
  banner with zero assertions printed**. 5 401 bytes of stderr trace and one line of stdout. It
  cannot fail visibly.
* **`test_parity_fw_effects`** exits **0** while carrying a `FAIL` line. See §3.

`debug_eli.c`, `tmp_list.c` and `test_zzprobe.c` are **scratch files sitting in `tests/`** and are
counted as suites. Move them out.

---

## 2. ⛔ `FAIL` LINES WITHOUT A COLON — 1 209 of 3 486

### 2.1 The corpus emits six result prefixes, only two of which carry a colon

| prefix | lines | what a `^FAIL:` scan sees |
|---|---:|---|
| `FAIL:` | **2 277** | counted |
| `FAILED` (`FAILED:` and `FAILED   `) | **990** | counted (starts with `FAIL`) |
| `FAIL [` — bracket form, **no colon** | **175** | counted only by `^FAIL` |
| `CRASH` | **2** | **nothing** |
| `VACUOUS` | **44** | **nothing** |
| `SETUP BUG:` | **2** | **nothing** |

**`^FAIL` (no colon, case-sensitive, stdout **and** stderr) = 3 486. `^FAIL:` = 2 277. The
difference is 1 209 lines — 34.7 % of every failure in this corpus.**

### 2.2 Nine suites where a `^FAIL:` scan reports **ZERO** failures

These suites emit **only** `FAIL [` and `FAILED` forms. A `^FAIL:`-based gate calls all nine
perfectly green.

| suite | `FAIL` lines a `^FAIL:` scan misses | of which `FAIL [` | exit |
|---|---:|---:|---|
| `test_parity_integration_extra` | **69** | 47 | 1 |
| `test_parity_full_round2` | **68** | 54 | 1 |
| `test_parity_full_round` | **44** | 29 | 1 |
| `test_parity_integration_rounds` | **25** | 19 | 1 |
| `test_parity_full_round3` | **17** | 12 | 1 |
| `test_parity_energy_position_delta` | **10** | 6 | 1 |
| `test_parity_realcard_walkthrough` | **7** | 5 | 1 |
| `test_audit_fields` | **4** | 2 | 1 |
| `test_parity_support` | **2** | 1 | 1 |
| **total** | **246** | **175** | |

Verbatim samples:

```
test_parity_full_round2   FAILED   N1_positive_exact_path
                          FAILED   N2b_positive_twin_of_N2
test_audit_fields        FAILED M_heart_total_limit
                          FAILED N_choose_target_steps
                          FAIL [heart_total_limit] M_heart_total_limit: heart_total_limit/operator must reach the appear-time filter
test_parity_support      FAILED   B_mandatory_select_card_not_skippable
                          FAIL [B_mandatory_select_card_not_skippable]: §B: 鬼塚夏美's ライブ成功時 must raise its mandatory hand-discard prompt …
```

### 2.3 On PowerShell, `-match` will inflate this further

`-match` is case-insensitive, so a summary line such as `failures (shared) : 54` counts as a
failure. Use `-cmatch`, and stringify make's stderr with `ForEach-Object { "$_" }`. Documented in
`PARITY_CHECKLIST.md`; repeated here because it is the third time this project has published a
failure count on the wrong surface.

---

## 3. ⛔ THE EXIT CODE IS NOT A SIGNAL — 12 suites prove it in the other direction

**`run_rc == 0` while carrying `FAIL` lines — 12 suites, 548 failures:**

| suite | `FAIL` lines | suite | `FAIL` lines |
|---|---:|---|---:|
| `test_parity_small_clusters` | **104** | `test_parity_rules_phases` | 28 |
| `test_parity_under_member` | **87** | `test_parity_abilitymod_conditional` | 13 |
| `test_parity_conditional_gated` | **69** | `test_parity_rules_conditions` | 10 |
| `test_parity_zone_source` | **69** | `test_parity_energy_remainder` | 7 |
| `test_parity_compound_per_card` | **64** | `test_parity_fw_effects` | 1 |
| `test_parity_blade_under` | **50** | | |
| `test_parity_recover_score_state_delta` | **46** | | |

**`run_rc != 0` while carrying ZERO `FAIL` lines — 2 suites:**

| suite | exit | stdout | what actually happened |
|---|---:|---:|---|
| `test_audit_conditions` | 2 | 0 B | never loaded the card DB (§1.2) |
| `test_state_effects` | 139 | 0 B | SIGSEGV (§1.3) |

Both directions of disagreement are populated. **No gate may key on `$?`.**

---

## 4. ⛔ `VACUOUS` AND `SETUP BUG` — the published tally is wrong in both count and location

### 4.1 What is actually there

**44 lines begin with `VACUOUS`** (the scannable form) and **2 lines begin with `SETUP BUG:`** —
in **three** suites, not the ten `PARITY_CHECKLIST.md` H5 names:

| suite | `^VACUOUS` lines | distinct vacuous cases | what they are |
|---|---:|---:|---|
| `test_parity_required_hearts` | **26** | **13** | genuinely vacuous |
| `test_parity_cost_discount` | **10** | **10** | genuinely vacuous |
| `test_parity_state_per_card` | **8** | 8 | the suite's own **"PROMPT CONTROL"** markers — non-vacuity guards firing, not vacuous assertions |
| `test_parity_blades_live_start` | 0 | 0 | **2 `SETUP BUG:` lines** |

**The 23 genuinely vacuous cases** are listed with their RED paired positives and their
reachability in `COVERAGE_GAPS.md` §2.1. The 8 `state_per_card` lines are a distinct class: they
are the suite correctly *flagging* that a prompt must be proven before a downstream assertion means
anything. That is a control firing, and it is the right behaviour — but it is **not** proof, and it
must not be tallied as a vacuous assertion.

### 4.2 Why the published "43 `VACUOUS` + 5 `SETUP BUG`" is wrong

**314 lines in the corpus CONTAIN the substring `vacuous`; only 44 BEGIN with it.**

The 314 are dominated by per-case summary lines that read
`… 0 assertion(s), 0 failure(s), 0 setup bug(s), 0 vacuous` — a line that **reports zero**
vacuous assertions is itself counted as one by any substring scan. The suite distribution of the
314 is not the distribution of the 44:

| suite | lines containing `vacuous` | lines **starting** with it |
|---|---:|---:|
| `test_parity_cost_discount` | 100 | **10** |
| `test_parity_required_hearts` | 97 | **26** |
| `test_parity_full_round` | 22 | 0 |
| `test_parity_look_per_card` | 16 | **0** |
| `test_parity_state_per_card` | 9 | **8** |
| `test_parity_target_player` | 8 | **0** |
| `test_parity_recover_q79` | 6 | **0** |
| … 21 more suites | 56 | **0** |
| **total** | **314** | **44** |

**Every one of the ten suites `PARITY_CHECKLIST.md` names for this defect emits zero `VACUOUS`
lines today**, and the live ones are three suites the checklist does not mention. `SETUP BUG`:
**2**, both in one suite, not five.

**Fix:** anchor on `^VACUOUS`, and rename the summary field from `vacuous` to
`vacuous_asserts` so it stops matching a case-insensitive substring scan.

### 4.3 The two `SETUP BUG` lines, verbatim — both are wrong-card fixtures

```
test_parity_blades_live_start  SETUP BUG: expected card PL!HS-pb6-002-R, resolved id=-1 (<none>)
test_parity_blades_live_start  SETUP BUG: expected card PL!HS-bp1-002-CL, resolved id=2793 (村野さやか)
```

* `PL!HS-pb6-002-R` resolves to **id = −1** — the card does not exist in the database.
* `PL!HS-bp1-002-CL` resolves to **id = 2793 = a different card**.

**Every assertion in those cases tests the wrong card and passes.** `A=gain_resource` blade is
**272/985** abilities, so a silently-wrong blade fixture is expensive.

### 4.4 A fourth measurement defect found while tabulating §4.1 — the cross-stream name split

The same case appears under **two different names** on the two streams:

```
stdout   VACUOUS  f_center_zero_heart03      <-- ...
stderr   VACUOUS: f_center_rin_zero_heart03 -- ...
```

**Any per-case join across stdout and stderr loses this case.** This is the same vocabulary split
as `FAIL:` / `FAILED` / `FAIL [`, one level up: the case *identity* itself is not stable across
streams.

---

## 5. ⛔ A GUARD MISSING A SETUP LEG

`tests/test_parity_hearts_live_start.c:1598-1630`, the Q246 fixture.

```c
1601:  int kosuzu2 = mid(&tg2, "PL!HS-pb1-005-R");     // NEVER ident()-pinned
1602:  int dia2    = mid(&tg2, "PL!S-bp3-004-R");      // NEVER ident()-pinned
1603:  int kotori2 = mid(&tg2, "PL!-bp3-003-R");
1605:  if (!ident(&tg2, joint2,  "LL-bp6-001-R＋")) return;
1606:  if (!ident(&tg2, kotori2, "PL!-bp3-003-R"))   return;
1607:  if (filler2 < 0) return;
```

Fixture 1 (`:1567-1570`) pins all 4 of its cards. Fixture 3 (`:1638-1640`) pins all 3 of its
cards. **Fixture 2 pins only 3 of its 5** — `kosuzu2` and `dia2` are unpinned.

If either card number ever fails to resolve, `mid()` substitutes a different card, `ident()` is
never asked, and the two assertions that carry the entire point of the case —

```c
1627:  CHECK_EQ(heart_of(&tg2, joint2, H01), 0, "H01 must NOT come from the unselected kotori");
1629:  CHECK_EQ(heart_of(&tg2, joint2, H03), 0, "H03 must NOT come from the unselected kotori");
```

— silently become assertions about the wrong cards. **Both still pass, because both expect `0`.**
That is precisely the mechanism that turns a negative assertion into a vacuous one, and it is the
same shape as the cluster-W finding of a test **staging an indeterminate stack value**: here the
indeterminate value is `mid()`'s return, which is `-1`-shaped on failure and is only sometimes
caught.

**The fixture data itself is correct** — verified against `cards/cards.json`:
`PL!HS-pb1-005-R` = H04/H05/H06, `PL!S-bp3-004-R` = H02/H05, `PL!-bp3-003-R` = H01/H03/H06; and
`heart_of()` at `:139` reads `rb_mods_get_heart(&state.mods, …)`, the modifier total, not the printed
slice — so the `== 1` expectations measure the grant delta as intended. **Only the two guard legs
are missing.**

---

## 6. ⛔ A TEST ASSERTING MORE THAN ITS RUST TWIN

### 6.1 `tests/test_parity_jidou_watch.c:675` — the assertion does not test its own message

```c
670:  int before = wait_len(&game);
671:  test_pass(&game);   /* LiveCardSetFirstAttacker → P2Turn  */
672:  test_pass(&game);   /* P2Turn → FirstAttackerPerformance → LiveStart */
674:  int after = wait_len(&game);
675:  CHECK(after >= before,
676:        "a non-ライブ card forced into the live slot is removed before "
677:        "ライブ開始時, so the Q213 heart-requirement reduction cannot count it");
```

`wait_len()` is the **waitroom** length. Evicting a card from the **live slot** cannot shrink the
waitroom, so `after >= before` holds by construction. **The message claims the card is removed
before ライブ開始時; the assertion checks that the waitroom did not shrink.** It cannot fail for the
reason its own message gives. This is the `W-c` class (`COVERAGE_MAP.md` §4.1) and it is
load-bearing here: the same case is the one place the C's eviction order is pinned.

### 6.2 `tests/test_parity_jidou_watch.c:680` — pinned to the C's *wrong* behaviour

```c
680:  CHECK(!test_zone_has_id(&game, 0, "live", hasetsu),
681:        "the member card is no longer in the live zone at ライブ開始時");
```

This asserts the wrongly-set member is **gone from the live zone by ライブ開始時**. **Rust keeps it
until AFTER the live.** The divergence is already recorded twice in `PARITY_CHECKLIST.md`:

* T3 header item 5 — `live_face_up[]`: *"Rust evicts a wrongly-set member AFTER the live, C evicts
  BEFORE"*;
* T6 open item 3 — `check_invalid_resolution_zone`.

**So this test is green precisely because the engine is wrong, and it will go RED when the engine
is fixed. A test that fails on the fix is a test that blocks the fix.**

### 6.3 The q246 fixture-order claim, resolved

The brief cited "a q246 fixture order" divergence. **It is not an ordering bug and not an
inverted expectation — it is the missing guard in §5.** The fixture's card data is correct, the
`heart_of()` accessor reads the right surface, and the hand order `{kosuzu2, dia2, kotori2,
filler2}` with pick `{0, 1}` correctly selects the two eligible cards and leaves `kotori2`
unselected. The only defect is that `kosuzu2` and `dia2` are never identity-pinned.

---

## 7. ⛔ SUITE NAMES IN CIRCULATION THAT DO NOT EXIST

**46 names.** A name that does not exist reads as passing, because `make` emits no `FAIL` for a
suite that was never built. This is the most dangerous property of this harness: a suite everyone
believes in, testing nothing.

### 7.1 Tier A — would be run as `make t T=<name>`; 18 names

Verified absent: **no `tests/test_<name>.c` and no `tests/<name>.c`.**

| # | phantom name | cited in |
|---:|---|---|
| 1 | `parity_card_filters` | `NOZOMI_ROOT_CAUSE.md`, `PARITY_CHECKLIST.md` |
| 2 | `parity_card_text` | `AUDIT_actions_setup.md`, `ORPHANS_AND_SCOPE.md`, `PARITY_CHECKLIST.md` |
| 3 | `parity_condition_decode` | `NOZOMI_ROOT_CAUSE.md`, `PARITY_CHECKLIST.md` |
| 4 | `parity_draw_top` | `PARITY_CHECKLIST.md` |
| 5 | `parity_dynamic_count` | `AUDIT_effects_state.md`, `ORPHANS_AND_SCOPE.md`, `PARITY_CHECKLIST.md` |
| 6 | `parity_effect_conditions` | `NOZOMI_ROOT_CAUSE.md`, `PARITY_CHECKLIST.md` |
| 7 | `parity_lookup` | `PARITY_CHECKLIST.md` |
| 8 | `parity_max_distinct_names` | `PARITY_CHECKLIST.md` |
| 9 | `parity_nazuna_at_deck_top` | `PARITY_CHECKLIST.md` |
| 10 | `parity_resolver` | `NOZOMI_ROOT_CAUSE.md` |
| 11 | `parity_survey_probe` | `PARITY_CHECKLIST.md` |
| 12 | `parity_tp_joint` | `PARITY_CHECKLIST.md` |
| 13 | `parity_tp_zone_change_gate` | `PARITY_CHECKLIST.md` |
| 14 | `parity_vm` | `NOZOMI_ROOT_CAUSE.md` |
| 15 | `parity_watchers` | `PARITY_CHECKLIST.md` |
| 16 | `audit_cards` | `AUDIT_actions_setup.md`, `AUDIT_effects_state.md`, `ORPHANS_AND_SCOPE.md`, `PARITY_CHECKLIST.md` |
| 17 | `audit_game_state` | `AUDIT_actions_setup.md`, `PARITY_CHECKLIST.md` |
| 18 | `audit_modifiers` | `PARITY_CHECKLIST.md` |

**Seven of the 18 are rename-only phantoms** — the real file exists under another name, so the fix
is to correct the documents, **not** to create a file:

| phantom | the file that actually exists |
|---|---|
| `parity_vm` | `tests/test_vm_decode.c` |
| `parity_max_distinct_names` | `tests/test_max_distinct_names.c` |
| `parity_survey_probe` | `tests/test_survey_probe.c` |
| `parity_tp_joint` | `tests/test_tp_joint_card_live_start.c` |
| `parity_tp_zone_change_gate` | `tests/test_tp_zone_change_gate.c` |
| `parity_condition_decode` | `tests/test_parity_t7_condition_decoder.c` |
| `audit_x_class` *(inverted prefix)* | `tests/test_parity_x_class.c` |

**Two of the 18 are VERIFIED-NOOP** — the behaviour does not exist in the corpus at all:

| phantom | measurement |
|---|---|
| `parity_dynamic_count` | `X=dynamic_count` = **0 nodes** across all 2 116 pairs |
| `parity_card_text` | **0/985** — there is no display layer in C at all (T5-1) |
| `parity_nazuna_at_deck_top` | `PL!N-bp7-028-L` **is** ability **805**, one of the two K7-skewed slices — see §8 |

**The remaining nine are real coverage holes**: `parity_card_filters` (273/985 reach),
`parity_effect_conditions` (23/985), `parity_draw_top` (119/985), `parity_lookup` +
`parity_watchers` (17/985), `parity_resolver`, `audit_cards`, `audit_game_state`,
`audit_modifiers`.

### 7.2 Tier B — in circulation, not suite names, but they read as suite names; 24 names

**B1 — tool names conflated with suite names (7).** `tools/` holds these; a reader looking for
`tests/test_audit_cluster_d.c` finds nothing and concludes the suite is missing.

`audit_cluster_d` · `audit_w_gate` · `audit_x_class` · `audit_x_const` · `audit_x_lookat` ·
`audit_t7_decoders` · `audit_t7_resource` · `audit_t7_wire` · `audit_t7_zeroheart`
(cited in `BASELINE2.md`, `PARITY_CHECKLIST.md`, `NOZOMI_ROOT_CAUSE.md`, `PARSER_ENGINE_COMPARISON.md`,
`AUDIT_decoders_resource.md`)

**B2 — case names from `TEST_PARITY_SURVEY.md` listed bare, so they read as suites (9).**

`audit_additive_modifier_can_cause_failure` · `audit_additive_modifier_stacks_on_base` ·
`audit_allocations_match_filled` · `audit_both_players_fail_no_winner` ·
`audit_p1_passes_p2_fails_p1_wins` · `audit_set_zero_removes_requirement` ·
`audit_sufficient_hearts_passes` · `audit_two_cards_second_fails` ·
`audit_zero_stage_live_card_must_fail`

**B3 — other `audit_*` claims in circulation (8).**

`audit_emitted_keys` · `audit_field_coverage` · `audit_fixture_annotations` ·
`audit_id_in_assert` · `audit_placeholders` · `audit_vs_official` *(cited in
`PBLL02_ABILITY_REVIEW.md`, `PBLL02_TEST_AUDIT.md`, `PROGRESS.md`, `BASELINE2.md`,
`PARSER_ENGINE_COMPARISON.md`)*

### 7.3 Already documented as phantoms — 4, still absent

Confirmed still absent, and mentioned only in the passages that record them as phantoms:

| name | documented at |
|---|---|
| `test_setup` | `PARITY_CHECKLIST.md:1479`, `AUDIT_actions_setup.md:591` |
| `card_data_decode` | `PARITY_CHECKLIST.md:1482`, `AUDIT_actions_setup.md:593` |
| `card_info_show` | `PARITY_CHECKLIST.md:1482`, `AUDIT_actions_setup.md:593` |
| `card_info_parity` | `PARITY_CHECKLIST.md:1483`, `AUDIT_actions_setup.md:594` |

**18 + 24 + 4 = 46.**

### 7.4 A fifth phantom class, on the **Rust** side

`engine/tests/test_modules/characterization/unique_abilities_test.rs` — 1 836 bytes, **zero
`#[test]`** functions, despite the `_test.rs` name. And three generated `mod.rs` files declare test
modules that have never existed in git:
`ability_gates/mod.rs` → `bounded_fan_cost_equality_gate_test`;
`effects/hearts/mod.rs` → `set_heart_type_multi_target_test`;
`expansion/mod.rs` → `pbl02_card_table_test`.

`build.rs` regenerates and silently deletes them, so `cargo test` is unaffected — but they are
**three test families named in a checked-in file that do not exist**, which is the same failure
mode as §7.

### 7.5 And one Rust suite that exists but never runs

`engine/tests/test_modules/integration/pvp_room_test.rs` — **346 lines, 14 `#[test]`, committed to
git, `EXCLUDED_MODULES` in `engine/build.rs`.** It is on disk, it is in git, and it is never
compiled. **14 Rust tests, zero executed.** See `COVERAGE_GAPS.md` §4 G-A0.

---

## 8. ⚠ K7 — every test that depends on the two skewed ability slices

`cards/build/abilities_strings.bin` is **726 649 B / STALE** against the canonical
**726 831 B / `89204df0bbde08a0…`**, and Rust's `OFFSET_DELTAS` disagrees with C's
`RBKA_OFFSET_DELTAS` at **2 of 985** slices. **For these two the C may be correct and any
Rust-derived expectation is suspect.**

| ability | identity | C tests pinned to it |
|---:|---|---|
| **325** | `PL!-pb2-012-P＋` (ab#1) and `PL!-pb2-012-R` (ab#1); use_limit 1; cost is `sequential_cost`; effect is `move_cards` hand→discard filtered on `group_names = Printemps` | **`tests/test_parity_wave1b.c`** §C — `:2396`, `:2661`, header `:18` |
| **805** | `PL!N-bp7-028-L` (ab#0) *Cooking with Love*; effect carries `conditional_action` + `optional_action` + `heart_colors` + `shuffle` + `group_names`; no cost | **`tests/test_parity_hearts_live_start.c`** at **4 sites** — `:889-890`, `:912-913`, `:941-942`, `:973-974` |

Both are 1/985 abilities, so the corpus exposure is small. **The risk is entirely in the tests:**
any expectation these 5 sites carry is derived from a slice where the two engines disagree about
where the bytecode starts. `tests/test_describe_golden.c` already excludes exactly 325 and 805 via
its `EXCLUDED_SKEW` set — **it is the only test in the corpus that knows the skew exists**, and that
set should be asserted to be *exactly* `{325, 805}` so a third divergence cannot slip in unnoticed.

---

## 9. ⛔ The defect that makes all of the above undetectable

**`PASSED` verdict lines: 0. Corpus-wide. Against 990 `FAILED` verdicts.**

No suite in this corpus can report success. There is no `ok: PASSED`, no `ALL GREEN`, no
`verdict_ok`. `test_phase_machine_rules` prints `ALL PHASE MACHINE RULE CHECKS PASSED` and is the
only positive verdict string anywhere — and it prints it alongside **zero assertions** and 5 401
bytes of trace.

Therefore, for any suite in this corpus:

> **a suite that segfaults before its first assertion produces exactly the same stdout/stderr
> shape as a suite that passed every assertion** — unless you read the exit code, and §3 proves
> the exit code is populated in both directions.

This single property is why `tmp_list` (§1.1), `test_audit_conditions` (§1.2),
`test_state_effects` (§1.3), `test_p1_helpers` (§1.4), the 9 suites of §2.2 and the 12 suites of
§3 have survived in a corpus whose every report claims a `FAIL` count. **Every one of them scores
0 failures on a `^FAIL:` scan.**

The fix is one line per suite: `printf("PASSED: %s\n", name)` on a clean exit, plus a
manifest-vs-glob diff so a name with no file fails loudly. Both are task 0.1 / 0.2 in
`COVERAGE_WORKLIST.md`.

---

## 10. Complete defect index

| # | defect | measured damage | §  |
|---:|---|---|:--:|
| D-1 | `tmp_list` fully silent, exit 0 | a suite that tests nothing reads as passing | 1.1 |
| D-2 | `test_audit_conditions` never loads the card DB | 1 002 lines, 0 assertions, scores 0/0 | 1.2 |
| D-3 | `test_state_effects` SIGSEGV | 33 literals, none evaluated | 1.3 |
| D-4 | `test_p1_helpers` 0 B stdout | 249 literals, 1 reported failure | 1.4 |
| D-5 | 17 suites print zero `ok:` lines | private result vocabularies | 1.5 |
| D-6 | `^FAIL:` blind to 1 209 of 3 486 failures | 246 in 9 suites scored **0** | 2 |
| D-7 | 12 suites exit 0 with 548 `FAIL` lines | exit status unusable | 3 |
| D-8 | 2 suites exit non-zero with 0 `FAIL` lines | exit status unusable, other direction | 3 |
| D-9 | the "43 `VACUOUS`" tally is a substring scan | wrong count **and** wrong location; 44 `^VACUOUS` in 3 suites | 4.1–4.2 |
| D-10 | 2 `SETUP BUG` wrong-card fixtures (one resolves to id −1) | every assertion in those cases tests the wrong card | 4.3 |
| D-11 | the same case named differently on stdout and stderr | per-case joins lose cases | 4.4 |
| D-12 | q246 fixture missing 2 `ident()` guard legs | 2 negative assertions pass about the wrong cards | 5 |
| D-13 | `jidou_watch.c:675` asserts the waitroom, not the live slot | cannot fail for the reason its message gives | 6.1 |
| D-14 | `jidou_watch.c:680` pinned to the C's wrong eviction order | will go RED when the engine is fixed | 6.2 |
| D-15 | **46** suite names in circulation with no file | each reads as passing | 7 |
| D-16 | `pvp_room_test.rs` committed but `EXCLUDED_MODULES` | 14 Rust tests never compiled | 7.5 |
| D-17 | `unique_abilities_test.rs` has 0 `#[test]`; 3 generated `mod.rs` name nonexistent modules | 4 phantom Rust test families | 7.4 |
| D-18 | `PASSED` never printed anywhere | every other defect is undetectable | 9 |
| D-19 | 3 scratch files (`tmp_list.c`, `debug_eli.c`, `test_zzprobe.c`) counted as suites | inflates the suite count and the "142" figure | 1.5 |
| D-20 | 172 build warnings across 44 suites | includes the always-true `-Waddress` rows W1/W2 | 0 |

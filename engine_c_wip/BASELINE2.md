# BASELINE2 — whole-tree parity measurement, `engine_c_wip` vs Rust

**Measured:** 2026-10-04, in an isolated snapshot, two strictly-sequential runs that agree
byte-for-byte with **zero normalisation**.
**Snapshot root:** `C:\Users\trios\AppData\Local\Temp\kilo\base2\engine_c_wip`
**Manifest:** `C:\Users\trios\AppData\Local\Temp\kilo\base2\MANIFEST.sha256` — **265 entries**
(every `.c`, `.h`, `.py`, `.sh`, `.ps1` and the `Makefile`). Quoted in full in §12.
**Prior baseline for comparison:** `engine_c_wip/BASELINE.md` (2026-10-03) — 2 530 FAIL /
949 failed tests / 108 red / 28 green / 5 unverified / 1 silent of 142. Its snapshot
(`Temp/kilo/baseline`) survived intact and still matches its own 240-entry manifest, so
**every delta in §9 is a line-level diff against real captured bytes, not an estimate.**

**Measured by:** the measurement authority.
**No `.c`, `.h`, `Makefile`, `.sh`, `.ps1`, `.py` or test file was modified — not in the live
tree, not in the snapshot.** The snapshot is a copy; the live tree was only read. No git
write of any kind was performed: `stash@{0} session-wip`
(`c458ab6f85ce5559041defd25165789b62cd8f21`) was never popped, applied or dropped.
The only files this measurement wrote are its own three report files plus one manifest copy,
all under `engine_c_wip/`.

---

## 1. HEADLINE

| quantity | BASELINE (2026-10-03) | **BASELINE2 (this run)** | delta |
|---|---|---|---|
| **FAIL assertions, comparable scope** | **2 530** | **2 466** | **−64** |
| &nbsp;&nbsp;`FAIL: ` shape (stderr) | 2 350 | 2 291 | −59 |
| &nbsp;&nbsp;`FAIL [` shape (stderr) | 180 | 175 | −5 |
| &nbsp;&nbsp;`  FAIL  ` shape (stdout) | 0 | **0** | 0 |
| `FAIL (VACUOUS):` shape (stderr, excluded from the 2 466) | 44 | 44 | 0 |
| `VACUOUS` bare shape (stderr, excluded) | 21 | 21 | 0 |
| distinct FAIL assertion texts | 2 393 | 2 349 | −44 |
| distinct after integers → `N` | 2 271 | 2 224 | −47 |
| **per-test `FAILED` verdicts** | 949 | **997** | **+48** |
| **`CRASH` verdicts** | 1 | **2** | +1 |
| `FAILED` lines on **stdout** (new harness layer) | 0 | 997 | +997 |
| per-test verdict lines total (`ok` + `FAILED` + `CRASH`) | 2 126 | 20 029 | +17 903 |
| passing assertion echoes (`ok: …`) | 16 051 | 16 894 | +843 |
| suites **RED** | 108 | **114** | +6 |
| suites **GREEN** (ran, produced output, has verdict/tally evidence) | 28 | **29** | +1 |
| suites GREEN-UNVERIFIED (ran, output, no verdict/tally) | 5 | **12** | +7 |
| suites NO-OUTPUT (neither stream produced a byte) | 1 | **0** | **−1** |
| **total suites run** | 142 | **155** | +13 |
| `SETUP BUG` lines | 5 | **12** | +7 |
| `VACUOUS` + `FAIL (VACUOUS)` lines | 65 | 65 | 0 |
| engine faults (abnormal process termination) | 4 | **4** | 0 |
| `make all` errors / warnings | 0 / 120 | **0 / 104** | −16 warnings |
| `make warnings-audit` | — | **PASS** (12 diagnostics, 12 pins, 0 new, 0 stale) | |
| `make warnings-audit-test` | — | **PASS** (155 sources, 0 red, 0 pinned) | |
| hangs | 0 | **0** | 0 |
| slowest suite | `parity_small_clusters` 18.564 s | `parity_small_clusters` **17.534 s** | −1.03 s |

### 1.1 The one number you must read with the other

A `^FAIL` count with **no colon**, case-sensitive, on **stdout AND stderr** — the rule this
brief mandates — gives **3 507** lines today. That is *not* comparable to 2 530, because the
corpus gained a **fourth and fifth `FAIL`-prefixed shape** that the 2026-10-03 baseline had
no source for:

| stream | shape | run 1 | run 2 | counted in the 2 466? |
|---|---|---|---|---|
| stderr | `FAIL: ` | 2 291 | 2 291 | **yes** |
| stderr | `FAIL [` | 175 | 175 | **yes** |
| stdout | `  FAIL  ` | 0 | 0 | **yes** (zero occurrences) |
| stderr | `FAIL (VACUOUS): ` | 44 | 44 | no — unproven, not divergent |
| stdout | `FAILED  <test> (N check(s), M failure(s))` | 868 | 868 | no — per-test VERDICT |
| stdout | `FAILED <test>` | 37 | 37 | no — per-test VERDICT |
| stdout | `FAILED: <test>` | 92 | 92 | no — per-test VERDICT |
| | **total `^FAIL`** | **3 507** | **3 507** | **2 466 of it** |

The 997 `FAILED*` lines are cluster H's new per-test verdict layer (`__func__`-named,
one line per failing test body). They are the reason "failed tests" moved 949 → 997 while
assertion lines moved 2 530 → 2 466. **Both numbers are in §1; neither is an estimate.**

---

## 2. RUN 1 vs RUN 2 — DO THEY AGREE?

# YES. EXACTLY. 155 / 155, ON BOTH STREAMS, WITH **ZERO** NORMALISATION.

* `stdout` sorted-line multiset identical in **155 / 155** suites
* `stderr` sorted-line multiset identical in **155 / 155** suites
* FAIL assertion count identical: **2 466** vs **2 466** (delta **0**)
* `^FAIL` superset count identical: **3 507** vs **3 507** (delta **0**)
* failed-test count identical: **997** vs **997** (delta **0**)
* `CRASH` verdicts identical: **2** vs **2**
* RED / GREEN / unverified / silent identical: 114 / 29 / 12 / 0 in both passes
* every per-suite wall clock, line count and byte count matched its counterpart within
  normal scheduler jitter; the slowest suite was `parity_small_clusters` at 17.534 s (run 1)
  and 20.935 s (run 2) — the only suite above 21 s, and both are far under the 600 s
  watchdog.

**There is NO nondeterminism in this tree.** No hash-iteration order, no uninitialised
memory, no pointer-value dependence, no stray file-static. Heap pointers printed on stderr
(`[RESUME_CAPTURE] def=0xa001990d0 …`, `[DEBUT_COST_PENDING] … eff=0xa00199ca0`) came out
**identical in both runs** — the mingw heap is deterministic here. The historical concern
that two runs of byte-identical source could differ has still not reproduced, and the
isolation rules in §3 are the reason.

### How the run was made deterministic
* One snapshot, one build, **no rebuild between the two passes**.
* **Strictly sequential**: one process at a time, alphabetical suite order, 155 suites per
  pass, 310 process launches, 600 s watchdog per suite, 0 timeouts.
  Total wall clock: **252.5 s** (run 1), **244.5 s** (run 2).
* **`stdout` and `stderr` captured to SEPARATE raw-byte files** via Python
  `subprocess` with two independent file handles. `2>&1` was never used, so no stream
  interleaving and no PowerShell `ErrorRecord` mangling (trap 3). Sanity check: the longest
  captured stdout line is 1 411 bytes (`max_distinct_names`) and the longest stderr line
  1 023 bytes (`parity_hearts_live_start`) — no truncation anywhere.
* **No suite ever shared a `tests/*.log`.** All 142 stale `tests/*.log` files were excluded
  from the copy. `make run-all-tests` was never invoked.
* **Exit codes are recorded but never used to decide red/green** (§7), because this corpus
  proves they are not a signal: 11 suites exit with a code equal to their own failure count,
  and 2 terminate abnormally while still having emitted a plausible-looking tail.

---

## 3. SNAPSHOT

Taken from the live tree, excluding **all** `*.o`, **all** `rb_engine*` binaries and **all**
`tests/*.log`. Live tree `engine_c_wip`: 566 files → snapshot: 423 files. The live tree
contained **0** `.o` and **0** `rb_engine*` at snapshot time, so the stale-link trap could
not fire from the copy; `make clean` was then run anyway and exited 0 with 0 objects
remaining.

Quiescence was checked mechanically: the snapshot manifest and a manifest of the live tree
taken immediately afterwards are **byte-identical (265 entries, 0 differences)**, so the
snapshot is a coherent state, not a torn copy.

Git state observed read-only at snapshot time: `HEAD = f152a50f`
(`fix(engine): 「これにより…」 counts the cards this ability moved`) — **unchanged from the
2026-10-03 baseline.** All work since then is uncommitted working-tree edits.

### ⚠ THE LIVE TREE MOVED **AFTER** THE SNAPSHOT WAS TAKEN — read this before using these numbers

This measurement describes the **snapshot**, not the live tree as it stands now. A manifest
of the live tree taken after both passes finished shows **6 differences against the
measured snapshot**, and `HEAD` has since advanced to `cf315a9a`:

| file | state |
|---|---|
| `src/ability/effects/move_missing.c` | **CHANGED** — this is nozomi's BLOCKED-ON #2, `maybe_prompt_deck_order` |
| `src/core/card.c` | **CHANGED** |
| `src/core/game_state_abilities.c` | **CHANGED** |
| `tests/test_parity_deck_order.c` | **CHANGED** |
| `tests/test_parity_foreign_cause.c` | **NEW** (a 156th suite) |
| `tools/census_deck_order.c` | **NEW** |

The live tree was quiescent for the 20 s after that check (0 further differences), so the
drift is a discrete landing, not an ongoing edit. **Consequence for the numbers in this
document:**

* The snapshot is internally consistent and the two runs describe it exactly — that is not
  in question.
* **§9's deck-order analysis is now stale.** This report measures the tree in which
  `move_missing.c` had **no** `maybe_prompt_deck_order`; that producer has since landed, so
  the 35 replacement `nozomi:` assertions and the `parity_int_gaps` +5 that I attribute to
  the missing producer **should shrink on the next measurement**. That is the single most
  likely place for the whole-tree number to move after this document was written.
* Anything the landed `card.c` / `game_state_abilities.c` / `move_missing.c` edits touch is
  likewise not represented here.

Re-measure before acting on any per-suite number in §6. The manifest in §12 is what pins
what these numbers describe.

---

## 4. DATA BLOB — measured against

`engine_c_wip/src/abilities_strings.bin`, verified at **four** checkpoints — before
`make clean`, after `make all` + `make web`, after `make data`, and after **both** test
passes:

```
bytes  = 726831
sha256 = 89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344
```

**Unchanged at all four. The measurement is not void.**

`src/cards.bin`: `631890` bytes, `728208504fee62171429ebb7bbdd8e12263e1c03b816c947c6fc98aa2676fac4`,
also unchanged.

**The measurement blob is `726831 B / sha256 89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344`.**

`make data` was made a green no-op by **seeding the snapshot's `../cards/build/` with the
canonical blobs** — `cards.bin` and `abilities_strings.bin` both copied from the live
`engine_c_wip/src/`, never from the stale `cards/build/`. `make data` exit **0**, and both
guards reported:

```
data guard: ../cards/build/cards.bin == src/cards.bin (631890 bytes, sha256 728208504fee62171429ebb7bbdd8e12263e1c03b816c947c6fc98aa2676fac4), already installed.
data guard: ../cards/build/abilities_strings.bin == src/abilities_strings.bin (726831 bytes, sha256 89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344), already installed.
```

For contrast, the **live** tree's `cards/build/abilities_strings.bin` is still
**726 649** bytes / `6225b9b531f4875afdb2ad5458bcf763bc18c157b58d87a53c9f30eddd189473`,
so `make data` there remains RED by design. **That refusal is correct and is not breakage.**
It was bypassed here by seeding, not by weakening the guard.

---

## 5. BUILD

| step | exit | errors | warnings |
|---|---|---|---|
| `make clean` | **0** | 0 | 0 |
| `make all` | **0** | **0** | **104** |
| `make web` | **0** | 0 | 0 |
| `make data` | **0** | 0 | 0 |
| 155 × suite compile+link | **0** | **0** | 161 |

Artifacts: `rb_engine.exe` = **1 401 818 B**, `rb_web_server.exe` = **1 426 753 B**.

### 5.1 Warning gate, reported separately (trap 5 respected: a real `-c` build, never `-fsyntax-only`)

| n | file | n | file |
|---|---|---|---|
| 25 | `src/ability/util.c` | 3 | `src/core/data.c` |
| 15 | `src/ability/effects/look.c` | 3 | `src/turn/triggers.c` |
| 10 | `src/ability/effects/rules.c` | 2 | `src/ability/condition.c` |
| 9 | `src/ability/choice.c` | 2 | `src/ability/effects/state.c` |
| 7 | `src/core/game_state_abilities.c` | 2 | `src/ability/effects/misc.c` |
| 6 | `src/ability/types.c` | 2 | `src/ability/effects/draw.c` |
| 5 | `src/ability/cost.c` | 2 | `src/game/game_setup.c` |
| 5 | `src/ability/effects/move.c` | 2 | `src/core/game_state_core.c` |
| | | 1 each | `src/ability/vm.c`, `effects/ability_effects.c`, `effects/move_missing.c`, `core/zones.c` |

By category: `-Wstringop-truncation` 41, `-Wunused-function` 28, `-Wformat-truncation=` 15,
`-Wunused-variable` 8, `-Wcomment` 4, `-Wrestrict` 3, `-Wmisleading-indentation` 2,
`-Wunused-but-set-variable` 2, unclassified 1
(`src/game/game_setup.c:218 'RB_SETUP_MAX_DOUBLE_BATON' redefined`).

**Migration from the baseline's 120:** `-Wparentheses` 1 → **0**, `-Waddress` 2 → **0**,
`-Wdiscarded-qualifiers` 2 → **0**, unclassified 16 → 1, `-Wunused-variable` 13 → 8,
`-Wunused-but-set-variable` 6 → 2, `-Wunused-function` 29 → 28, and
`-Wformat-truncation=` 0 → 15 because cluster W **added the class** to `CFLAGS` rather
than leaving it ungated. Net −16.

`make warnings-audit` → **exit 0**.
```
warnings-audit: bug-predicting diagnostics found = 12, baseline pins = 12
warnings-audit: baseline-still-present=12  NEW=0  stale-pins=0
warnings-audit: PASS -- 12 diagnostic(s), all pinned; 0 new.
```
The 12 pins are 9 `-Wformat-truncation=` (cluster-W rows FT-1..FT-9) + 3 `-Wrestrict` in
`src/core/data.c` (W5/W6/W7). `data.c` really does emit its 3 `-Wrestrict` under `-c`
(trap 5 confirmed empirically: same TU gives 0 under `-fsyntax-only`).

`make warnings-audit-test` → **exit 0**.
```
warnings-audit-test: 155 test sources, 0 red, 0 pinned  (flags: -Werror=array-bounds -Werror=maybe-uninitialized)
```
`-Warray-bounds` and `-Wmaybe-uninitialized` across the whole test corpus: **0**.

All 155 suites compiled: **BUILD_RED = 0**, zero `error:` and zero `undefined reference`
across all 155 compile captures. Suite-compile warning lines: 161 (`-Wunused-function` 87,
`-Wunused-variable` 38, `-Waddress` 15, `-Wcomment` 11, `-Wdiscarded-qualifiers` 2,
`-Wformat-truncation=` 2, `-Wunused-but-set-variable` 2, `-Wformat=` 2, unclassified 2).

---

## 6. `sizeof(GameState)` AND THE §A CEILING — §A IS RED, AND IT IS A STALE CONSTANT

Measured by the suite that exists to measure it, `tests/test_harness_self.c` §A, run from a
clean build:

```
sizeof: GameState=1195656 TestGame=1195656 RbPlayer=20824 RbBag=2052
```

`Makefile:3-4` claims `1195656` — **that claim is correct today.** `GameState == TestGame`
exactly (`A1` green).

**But §A2 and §A3 are RED, and the reason is a stale constant, not a broken struct:**

```
FAIL: A2 sizeof(GameState) keeps RB_MIN_SAFE_LOCALS of them inside the 16 MB linked stack
FAIL: A3 the multiplication the ceiling is derived from actually holds
```

* `tests/test_harness_self.c:97` — `#define RB_LINKED_STACK_BYTES 16777216u` (**16 MB**)
* `tests/test_harness_self.c:98` — `#define RB_MIN_SAFE_LOCALS 15`
* ⇒ ceiling = `16777216 / 15` = **1 118 481 B**, and `1 195 656 > 1 118 481` ⇒ **A2 fails by
  77 175 bytes.**
* `Makefile:25` — `LDFLAGS ?= -Wl,--stack,33554432` (**32 MB**), which the same file's
  comment at `:19-20` explains was raised from 16 MB precisely because the 16 MB ceiling was
  breached.
* ⇒ against the stack the binaries are **actually** linked with, the ceiling is
  `33554432 / 15` = **2 236 962 B** and `1 195 656 ≤ 2 236 962` ⇒ **A2 would pass.**

**Verdict: the C satisfies §A against the real 32 MB link stack, with 1 041 306 bytes of
headroom — and `tests/test_harness_self.c:97` is the bug, because it still pins 16 MB.** The
file's own prose (`:73-95`) is written entirely against the 16 MB number and is now wrong
throughout, including the "HEADROOM: 4745 bytes" claim at `:86`. This is reported, not
fixed.

Four other `harness_self` checks are also red, and all four are **ratchet** checks that
counted up because 13 new test files landed:

```
FAIL: B1 non-static GameState/TestGame locals in tests/ stay <= 1432 (now 1444)
FAIL: B2 files holding a non-static state local stay <= 75 (now 76)
FAIL: D1 test_id( call sites in tests/ stay <= 3013 (now 3017)
FAIL: D2 files using test_id( stay <= 102 (now 105)
```

---

## 7. HOW RED/GREEN WAS DECIDED — AND WHY EXIT CODES WERE IGNORED

Red is decided on **failure evidence only**: any `FAIL: `/`FAIL [`/`  FAIL  ` line, any
`FAILED` or `CRASH` verdict, any `SETUP BUG`, any `FATAL SIGNAL`. A suite with none of those
and which produced output is GREEN, or GREEN-UNVERIFIED if it also emitted no verdict or
tally of any kind.

Exit codes are not a signal here, and this run demonstrates it three ways:
* 11 suites exit with a code equal to their own failure count (`parity_state_per_card` 33,
  `parity_cce_gaps` 35, `parity_search_reorder` 1 …).
* `parity_conditional_gated` **69 FAIL assertions** and it still finishes.
* `parity_under_member` carries **57**; `parity_jidou_watch` **54**; `parity_full_round2` **54**.

### 7.1 The 12 GREEN-UNVERIFIED suites (ran, produced output, no verdict/tally of any kind)
`artifacts_consistent`, `audit_warnings`, `b8_live_timing`, `describe_golden`,
`parity_conditions`, `parity_enums`, `parity_fw_cost_cardtype`, `parity_fw_effects`,
`parity_t6_action_setup`, `phase_machine_rules`, `state_effects`, `zzprobe`.

Two of these are **verified green despite emitting no verdict line**:
`artifacts_consistent` prints `12 checks, 0 failed` / `consistent … 726831 bytes`; and
`describe_golden` prints its own tally. The other ten are diagnostics or single-line
smoke suites. **Named, counted, and not credited as passing.**

### 7.2 The 2 RED suites with FAIL assertions and ZERO per-test verdict lines
`p1_helpers` (1 FAIL line, **0 bytes stdout**) and `parity_characterization` (3 FAIL lines).
Reported explicitly rather than counted as 0 failed tests.

### 7.3 `NO_OUTPUT`: **0 suites.** The baseline's one silent suite is gone — see §8.1.

---

## 8. BUGS AND DEFECTS FOUND — REPORTED, NOT FIXED

I edited no source. Every item below is a measurement-authority finding for the fixer agents.

### 8.1 FIXED since the baseline (three harness defects, all real, all verified)
1. **`tests/test_draw_phase_fix.c:15-25`** — the blind `ASSERT_EQ` macro (no `FAIL` prefix,
   no counter, early `return`) is gone. The suite now prints `2 checks, 0 failures` and 2
   `ok:` lines; the file's header comment records the old shape verbatim. The baseline's
   NO-OUTPUT suite is now GREEN.
2. **`tests/test_parity_queue_resume.c`** — the SIGSEGV/SIGABRT that killed the binary
   mid-file (`rb_queue_get_entry(...)->effect_started` dereferencing NULL, and `free()` on
   string literals handed to `rb_effect_free`) is guarded. The suite went from **0 stdout
   lines** to 146, and `rb_unload()` now runs. It also gained a per-test verdict layer.
3. **`tests/test_parity_cce_gaps.c` / `tests/test_parity_recover_hand_a.c`** — the two
   engine faults the baseline recorded (SIGABRT `a22b_resolver_path_pays`, SIGSEGV
   `test_f_zero_success_card_does_not_retrieve`) **no longer occur**. Zero `FATAL SIGNAL`
   lines in either suite now.

### 8.2 ⚠ NEW REGRESSION, 2 SIGSEGVs, `tests/test_parity_look_per_card.c` — test file UNCHANGED
`n_tomari_accept_6` and `n_tomari_empty_hand` both die with **signal 11** inside the engine:
```
parity_look_per_card.err:220  *** FATAL SIGNAL 11 inside test: n_tomari_accept_6
parity_look_per_card.err:230  *** FATAL SIGNAL 11 inside test: n_tomari_empty_hand
parity_look_per_card.out:97   CRASH    n_tomari_accept_6  <-- unexpected child exit 139
parity_look_per_card.out:100  CRASH    n_tomari_empty_hand  <-- unexpected child exit 139
```
Crash point, from the suite's own stderr, immediately before the fault in both cases:
```
[RESUME_ANY]      kind=2 zone= target=pay_cost_all:discard_all route=1 mode=0 idx=1
[RESUME_CAPTURE]  def=0xa00199ad0 target=0x0 eff=0xa00199ad0 cur=0 n=0 state=0 pending=1
*** FATAL SIGNAL 11 … Assertions before the fault: 9, failures: 0, prompts: 2
```
Both cases are the 「好きな枚数」 whole-hand `pay_cost_all{discard_all}` resume with
`route=1 mode=0`, two prompts deep. `tests/test_parity_look_per_card.c` is **byte-identical**
to the baseline, and the baseline ran this suite with **0 CRASH verdicts and 37 FAIL
assertions**; it now runs with **2 CRASH verdicts and 30 FAIL assertions**. The crash
*replaces* 7 red assertions — 4 of them now never print because the child dies first. So
this is a genuine engine-caused regression **and** it is hiding 4 previously-visible
failures. The suspect area is the cost-resume path (`src/ability/cost.c` changed;
`src/ability/ability_queue.c`, `src/ability/resolver.c`, `src/ability/vm.c` changed).
**Not narrowed further without editing source, which I did not do.**

### 8.3 ⚠ TWO PRE-EXISTING ENGINE FAULTS THE BASELINE UNDER-COUNTED
Exit codes on this toolchain are provably `signal << 8`: a 3-line `int*p=0;return *p;`
program exits **2816** (= 11 << 8, SIGSEGV) and `abort()` exits **1536** (= 6 << 8,
SIGABRT). Measured against that:

| suite | rc | signal | proof it dies early | in baseline? |
|---|---|---|---|---|
| `state_effects` | 2816 | **SIGSEGV** | `tests/test_state_effects.c:288 printf("ALL STATE EFFECT CHECKS PASSED")` never printed; 0 bytes stdout | **yes — identical**, stderr is line-for-line the baseline's capture |
| `parity_cost_and_effect` | 1536 | **SIGABRT** | `tests/test_parity_cost_and_effect.c:1873 printf("\n%d checks, %d failures")` never printed | **yes — identical**, same line counts, same final stderr line `MARK:free_keepalive pending=0` |

Both are FLAT single-`main` suites, so a crash just truncates and neither emits a `CRASH`
verdict. The 2026-10-03 baseline recorded 1 `CRASH` verdict and called it "2 engine faults,
not 1"; **the true count was 4 then and is 4 now** — the two moved from
{`parity_cce_gaps`, `parity_recover_hand_a`} to {`parity_look_per_card` ×2} while
`state_effects` and `parity_cost_and_effect` were missed by both measurements because
nothing in the corpus records exit codes.

### 8.4 ⚠ `tests/test_harness_self.c:97` — the §A stack constant is stale. See §6.

### 8.5 ⚠ THE `live_face_up` REGRESSION IS **36 LINES**, NOT 10 — and the C is right
`src/turn/phase.c:703` `if (!g->live_face_up[is_p1 ? 0 : 1]) return;` is a faithful port of
`turn/actions/mod.rs:1732-1736`, with all three writers present
(`phase.c:1356`, `:1432` clear; `:1658` set). **The gate is correct.** What is wrong is that
**four suites hand-build a board and call `rb_check_timing` without setting
`live_face_up[seat] = 1`** — the exact line Rust's own twin carries at
`engine/tests/test_modules/rules/conditions/live_card_zone_movement_test.rs:112-113`.
Zero occurrences of `live_face_up` in any of the four test files.

| suite | test source | baseline | now | delta | failing check |
|---|---|---|---|---|---|
| `live_card_zone_movement` | **UNCHANGED** | 0 | **12** | +12 | invalid member / invalid energy leaves live zone, lands in waitroom / energy deck, movement recorded |
| `parity_rules_conditions2` | verdict line only | 0 | **16** | +16 | B1/B1p/B2/B4 — non-live card leaves live zone, per-owner waitroom, `g->batch_movements` records |
| `mechanics` | **UNCHANGED** | 0 | **5** | +5 | `tests/test_mechanics.c:97-135` — invalid live member / live energy |
| `parity_rules_conditions` | **UNCHANGED** | 6 | **7** | +1 | D1b CONTROL/D1b — rewritten from "a face-DOWN non-live card survives" to "IS removed" |
| | | | **total** | **+34** | |

One further +3 lands in `parity_hearts_live_start` (unchanged test source) on the fixture
`tests/test_parity_hearts_live_start.c:1576`, which forces a member `filler` into the live
slot — same root cause by construction, but the assertion text is about the bp6 discard
prompt, so I state it as the live candidate rather than as proof. **Against the recorded
`+10` regression this measures +34, and the Y.3 figure was taken on a 27-suite corpus that
contained neither `live_card_zone_movement` nor `parity_rules_conditions2`.**

### 8.6 The 47 → 26 suites that hid their FAIL lines: **26 remain**
685 FAIL assertion lines across 26 suites have no per-test `FAILED` verdict anywhere in
their output. Largest: `parity_search_reorder` 112, `parity_sequential` 113,
`parity_hearts_live_start` 67, `parity_mill_deck` 53, `parity_condition_and_effect` 48,
`parity_zone_source` 42, `parity_restriction` 43, `parity_compound_per_card` 41.
Consequence: **"failed tests" is still a range, not a number: 997 ≤ true ≤ 997 + 685.**
The baseline's range was 949 … 1 632. Both ends moved; the width shrank by 1.

### 8.7 12 `SETUP BUG` lines — the fixture was never built, so those tests asserted nothing
`audit_conditions` (3):
```
SETUP BUG: [PL!S-bp6-002-P] has no condition v=2 key=aggregate in abilities 0..7
SETUP BUG: [PL!N-bp7-022-N] has no condition v=7 key=group_names in abilities 0..7
```
`parity_blades_live_start` (2), one of which is a **wrong-card defect**:
```
SETUP BUG: expected card PL!HS-pb6-002-R, resolved id=-1 (<none>)
SETUP BUG: expected card PL!HS-bp1-002-CL, resolved id=2793 (村野さやか)
```
Baseline had 5. **Third category — not stale, not divergent, but unproven.**

### 8.8 `tests/test_parity_queue_resume.c` — two of its own new CHECKs are red
Cluster H added, in `test_optional_look_select_skip_terminates`:
```
FAIL: the drained entry is still in the queue after the skip
FAIL: the drained entry is marked effect_started so it is not re-run (got -1 expected 1)
```
The guard prevents a NULL dereference but the sentinel `-1` then fails the equality, which
is the intended design — so this is a **real, newly-visible divergence** (`ability_queue.c`
clears the entry before the assertion looks at it), not a harness artefact.

### 8.9 No engine source file was deleted, and 26 changed
New files: 13 `tests/test_*.c`, 12 `tools/*`. Changed: `Makefile`, `include/rabuka.h`,
`include/test_game.h`, **26** `src/**` files, **33** `tests/**` + `tools/guard_data_blob.sh`.
Full list in §12's manifest delta.

---

## 9. RECONCILIATION OF THE DELTA — 2 530 → 2 466

This is a **line-level diff against the surviving baseline captures**
(`Temp/kilo/baseline/logs/runs/run1_<suite>.{out,err}`, whose snapshot still matches its
240-entry manifest exactly). Failure texts are compared after stripping the new
`[tests\file.c:NNN]` citation suffix and normalising integers to `N`, because cluster H
added file:line citations to `parity_zones_position` and friends — without that, 37 of 38
lines there read as changed when nothing changed.

```
over the 142 baseline suites      2 530 -> 2 459     -71
in the 13 new suites                     ->     7      +7
WHOLE TREE                        2 530 -> 2 466     -64
```

**182 FAIL lines left the tree; 111 FAIL lines entered it.** Every closed and every added
pattern is enumerated in `logs/diff_norm.txt`. The reconciliation:

### 9.1 What closed (182 lines) — mapped to `PARITY_CHECKLIST.md`

| closed pattern | lines | the closed item |
|---|---|---|
| `nozomi: expected a SelectTarget order prompt, got SelectCard` | 12 | nozomi **BLOCKED-ON #1** — the `look_at` prompt in `look.c:387-403` deleted |
| `nozomi: the SelectTarget target is "order"` | 12 | nozomi **BLOCKED-ON #1 + #3** |
| `F: the reveal is filtered to live cards` | 9 | nozomi **BLOCKED-ON #5** — `cost.c:1671` extras-vs-struct-field |
| `nozomi: nothing joins the hand (got N expected N)` | 6 | same cluster, downstream |
| `nozomi: nothing is discarded (all three return)` | 6 | same cluster, downstream |
| `A3+ / A4 / A5` jidou ターン2回 allowance family (`parity_wave1b`) | 18 | **T4-3** use-key widening to `(card_id<<16)\|idx` + the re-scan guard |
| `parity_draw_flat` batch (see `diff_norm.txt`) | 17 | **C1** `rb_validate_mandatory_cost` at the offer site + **C3** distinct dedupe + `fresh instance next turn: offered (got 0 expected 1)` |
| `parity_zone_source` remainder | 9 | **nozomi #5** — `cost.c:1671` |
| `kaho:` / `tomari_accept:` / `tomari_skip:` look-at + deck-top family | 10 | the `look_at` verb fix, **partially** — see §9.3 |
| `§K highest_cost_respects_modifier`, `J: left self effective …`, `BN: own stage total …`, `suzu 17b`, `a cost-N member on stage -> self gains heart-N`, `§K pbN_cost_N_with_N_success` | 8 | **D5** `eval_highest_cost` + **D4** `cost_total` / effective-cost arms |
| `Proof / Na / Nb: stage cost N >= N -> the look-and-select prompt fired` | 5 | **D4** `cost_total` (13 pairs) |
| `[rurino_use_limit_blocks_second_same_turn]` ×3 | 3 | **T4** use-limit / re-scan-key family |
| `constant_blade_recompute…` / `manual_additive_survives_constant_recompute…` | 3 | **E** constant-scan recompute |
| `[heart_total_limit] card_filter_from_effect must copy heart_total_limit …` | 1 | **A17 / H1** producer |
| ~50 further single-line closures across 24 suites | ~62 | spread; enumerated in `logs/diff_norm.txt` |

The three largest waves the brief named reconcile here:
* **cluster W (−44):** the `-Wparentheses` / `-Waddress` / `-Wdiscarded-qualifiers` classes
  are **0** and their assertions are in the closed set (`highest_cost_respects_modifier`,
  `§K pbN_cost_*`, `J_cost_modifier`, `suzu 17b`, the 蓮ノ空 negatives, the `waited`-member
  riders, `two waits still unlock the heart-N rider`) — ~20 lines here, the rest of W's
  yield lands in the *warning* column (§5), not the FAIL column.
* **final wave effects/cost (−21 / −3):** `parity_search_reorder` −10, `parity_zone_source`
  −9, `parity_look_per_card` −7, `parity_look_and_filter` −6, `parity_target_player` −6,
  `parity_condition_and_effect` −4, `parity_cce_gaps` −2, `parity_cost_compound` ±0.
* **extras class (−20):** the struct-field-read-from-the-extras-table family —
  `F: the reveal is filtered to live cards` 9, the `kaho` look-at shape 10,
  `card_filter_from_effect` 1.

### 9.2 What entered (111 lines) — mapped

| added pattern | lines | the cause |
|---|---|---|
| `nozomi: the mandatory order prompt is open` + `vacuity control: …` + 13 more `nozomi:` / `nz(2):` texts | 35 | the deck-order cluster is **half** fixed: `look_at` no longer opens `SelectCard`, but `move/mod.rs:1915-1932`'s `maybe_prompt_deck_order` still has **no C producer**, so the prompt is now never raised at all instead of raised in the wrong shape. `src/ability/effects/move_missing.c` is **unchanged** since the baseline — the nozomi BLOCKED-ON #2 patch is not in this tree. |
| `live_face_up` class (§8.5) | 34 | the correct gate, met by tests that do not set the flag |
| `parity_int_gaps` 「4枚まで好きな順番で」 deck-order family | +5 net | same deck-order cluster, second suite |
| `parity_jidou` partner gating (`should NOT activate energy (got 2 expected 0)`) | 4 | Nijigasaki-partner filter; test source unchanged |
| `parity_queue_resume` drained-entry CHECKs | 2 | §8.8 — cluster H's own new assertions |
| `parity_hearts_live_start` bp6 discard prompt | 3 | live-zone fixture, §8.5 |
| `parity_cost_compound: paying that unpayable cost fails (got 1 expected 0)` | 1 | **C1/C6** area — the mandatory-cost refusal |
| `parity_full_round2: N3: the card P2 selected is at P2 left (got -1 expected 2642)` | 1 | `N3_sync_opponent_gets_choice` |

### 9.3 The honest summary of the movement
* **Net −64 assertion lines.** The brief's expectation of "≈ −80 net" was in the right
  region; the measured figure is −64 because **+45 lines of new nozomi/deck-order
  assertions were added to the same suite whose 36 lines closed**, and because the
  `live_face_up` gate costs **+34**, not the recorded +10.
* **Failures moved, they did not vanish.** 182 out, 111 in. **61 % of every line that closed
  was replaced by a new line.** The one suite that got worse on every axis is
  `parity_search_reorder`: −10 net but +35 texts / −45 texts, i.e. the whole defect was
  *renamed*, not fixed.
* Red/green moved **108 → 114 RED** while the assertion count fell. That is the harness
  layer doing its job: 13 new suites (7 of them RED), plus 6 suites that were GREEN and now
  carry failures. It is **not** a regression in engine quality, and it is why red/green and
  FAIL-count must be read as two separate axes.

---

## 10. PERFORMANCE — NOTHING HANGS

**0 hangs of 155, 0 timeouts, 0 suites that lost their output, 0 lost verdicts.**
Slowest 18 (run 1 wall clock):

| suite | run 1 | run 2 | FAIL lines |
|---|---|---|---|
| `parity_small_clusters` | 17.534 s | 20.935 s | 64 |
| `parity_cost_discount` | 12.435 s | 12.638 s | 47 |
| `parity_jidou_move_yell` | 12.374 s | 13.102 s | 47 |
| `parity_under_member` | 11.999 s | 12.465 s | 57 |
| `parity_state_per_card` | 10.904 s | 11.534 s | 45 |
| `parity_search_reorder` | 8.934 s | 9.220 s | 112 |
| `parity_cce_gaps` | 8.326 s | 7.938 s | 51 |
| `parity_required_hearts` | 7.963 s | 8.492 s | 16 |
| `parity_sequential` | 7.364 s | 7.494 s | 113 |
| `parity_jidou_watch` | 6.962 s | 6.952 s | 54 |
| `parity_zone_source` | 6.902 s | 7.030 s | 42 |
| `parity_mill_deck` | 6.549 s | 6.479 s | 53 |
| `parity_integration_extra` | 5.413 s | 4.170 s | 47 |
| `parity_blade_under` | 5.127 s | 7.969 s | 30 |
| `parity_compound_per_card` | 4.508 s | 3.716 s | 41 |
| `parity_live_total2` | 4.335 s | 4.460 s | 10 |
| `parity_look_and_filter` | 4.146 s | 3.560 s | 21 |
| `parity_jidou_state` | 4.090 s | 3.317 s | 17 |

Nothing is within 29× of the 600 s watchdog. **`test_choice_renderer` runs in 0.156 s
(run 1) / 0.087 s (run 2) from a clean build — the historical 3 746 s was a stale binary,
re-confirmed.** `max_distinct_names` emits 4 043 `ok:` echo lines and is the only suite
whose stdout dwarfs its FAIL count. New suites are all sub-second except `describe_golden`
(0.535 s) and `harness_self` (0.567 s).

---

## 11. STALE EXPECTATIONS vs GENUINE DIVERGENCE

The identical mechanical proxy to `BASELINE.md` §10 (a suite whose own C source matches
`EXPECTED_GAP|gap closed|RED as pinned|red by design|stale expectation|does not exist in
Rust|no C counterpart`):

| bucket | BASELINE | NOW |
|---|---|---|
| in a suite whose source **declares an expected gap** | 1 549 (61.2 %) | **1 512 (61.3 %)** — 54 suites |
| in a suite with **no such declaration** | 981 (38.8 %) | **954 (38.7 %)** |
| total | 2 530 | **2 466** |

**The ratio is unchanged to one decimal.** Top gap-declaring suites now:
`parity_sequential` 113, `parity_search_reorder` 112, `parity_small_clusters` 64,
`parity_under_member` 57, `parity_full_round2` 54, `parity_jidou_watch` 54,
`parity_mill_deck` 53, `parity_cce_gaps` 51, `parity_target_player` 48,
`parity_integration_extra` 47, `parity_jidou_watch_clusters` 47, `parity_jidou_watch2` 46.

**Limits of this split, stated plainly — unchanged from the baseline and worth repeating:**
* "declares an expected gap" means the C file *says* the divergence is known. It does **not**
  prove the expectation is stale. Many are genuine engine gaps that were documented rather
  than fixed. **This is an upper bound on "stale", not a count of it.**
* The 65 `VACUOUS` + `FAIL (VACUOUS)` lines are a **third** category — not stale, not
  divergent, but **unproven**. A passing assertion whose paired positive is red proves
  nothing. Keep them aside: 44 in 12 suites, unchanged in count from the baseline, but
  redistributed (`parity_look_per_card` 14, `parity_required_hearts` 13,
  `parity_target_player` 8, `parity_state_per_card` 8, `parity_recover_q79` 6,
  `parity_recover_hand_a` 4, `parity_wave1c` 3, …).
* Plus the **12** `SETUP BUG` lines (§8.7) — also unproven, also aside.
* So of 2 466: **≈1 512 "known/documented, needs a decision"**, **≈954 undocumented
  divergence** (the honest candidate list for genuine engine work), **65 unproven aside**.

---

## 12. SNAPSHOT MANIFEST — SHA-256 of every `.c` / `.h` / `Makefile` / `.py` / `.sh` / `.ps1` measured

265 entries. Copied verbatim from
`C:\Users\trios\AppData\Local\Temp\kilo\base2\MANIFEST.sha256`.
The baseline's 240-entry manifest is at
`C:\Users\trios\AppData\Local\Temp\kilo\baseline\MANIFEST.sha256`; the 25 new entries are
13 `tests/test_*.c` and 12 `tools/*`, and the 59 changed entries are listed in §8.9.

```
fdaa4695224ebee85925f2c93b137a83c3a46b106fed2695d4b928e9fd4797c9  Makefile
2741c269b99afee68278f7444020aae7faaa0cf2c514ab4ec3090ff0a3594dc9  assemble.py
3d1521590c92babf23f6b15a110038ed1906813980c3551084e765a692ab9d37  bcdump_tmp.py
46c0c2b6cc4d74ecb03cca4ed1915f98d6444043fb831b14841eab13723ae787  include/deck_builder.h
cce01916525ca52c51201b82aee94657d4e67187a9390d9427bc2e81b25c19e7  include/deck_parser.h
491ccdf31a5c4e75eb0b5c27523f8f15b0c71f0482cae79b5efe11ac46d9501e  include/rabuka.h
e7dcebd7635adfe74a5dd130c95b2f93bbfd34eb4e7aae2163d3611096544846  include/test_game.h
e76aa12211335708e01a64e6306f341d9dbad710278b88c26c4b094f2bd6fe95  include/web_server.h
f2904b45bffa4a31a7acea314151dda8eaa85ceb9ffe3278a4bef8f2e3d71a70  platforms/cdi/build_cdi.sh
b09a8fec06458ba4d7f50d45b3aa8ad2ed67b31ce0ce835eda1ee4fd61fa16d7  platforms/cdi/cdi_lib.c
01149ce6c4ca4ac199eabc814369c04e0c2583984e1ea4a6fc230b179cad647e  platforms/cdi/cdi_main.c
bffe0cde78a89e76246511ca5478bb8a4257a8ee77040448ad2a60989e2eb673  platforms/cdi/link_test.sh
1c3fa2e32e17587e773431e6b9bc2cee9058a583c338e8077a4815f2fb84a8fd  platforms/cdi/probe.sh
a156bd6f7157f1e2302fbe0e5ad6e4c728b95db5d1caa11735d176c7ec70dd88  platforms/cdi/sizes.sh
50261ab23d66bba1650a533c4af828e105816a0ea024add5c6d9cd427c595ae1  platforms/genesis/build_genesis.sh
b3f31e2d71e9b4ba044a9f8c5e125c37fe2ee40f9effb63af8d1c26990345683  platforms/genesis/gen_font.py
ec31aaf05f1d34c8ee30d0491054bda64e4eb66ffc697df4c082538639601142  platforms/genesis/genesis_lib.c
b645386a1c71856a8bd157105223952dd870f90401abc2a7f0de935273d955ad  platforms/genesis/genesis_main.c
138758f3ba8d8accaaf2dc7e6dfd7c0792c1ebf28bd8c7eed0237ec3459f202d  platforms/genesis/pad_checksum.py
fbe7d962f38d4f55e7ccdf7d6875277874d5122b9853d76db4f05f7ec1875df7  platforms/genesis/vdp.c
893c9f65d006fd7a484bab2ef7cff66fa934647063a909cf5cc766447578de96  platforms/genesis/vdp.h
e9ce983bf7a4a8ab0ce0db2dc279258f5e7c34e9e21fb23ca49a49b02b196aff  platforms/genesis/vdp_font.h
d95b4028f18b23aea99e94c7e078027d8f06d298a513bcaac4874a2d6482c12e  size_audit.py
94e9966039765b2a926317736cbfa9f72fd3dc2c531c24650cac97dd608d456b  src/ability/abilities_gen.c
934614ab96829ae9cb4f9b0ff4694ccb73e63da05a9aa42d16cb317fc7192c38  src/ability/ability_queue.c
4c2ef9557dd053ebd5a4f3b7b93cfad7a71ab516e116947ad05043794a6acd19  src/ability/choice.c
747bf25d7c34162169d5607aab21cd6a9ecfb6715c14d0323d07b2387a3cb54b  src/ability/choice_renderer.c
a7f9c01e9f4d75ae6c95de32b71d39a1a4fe911e09519354924030b28bf281b5  src/ability/compound.c
62aaf3818e5187983a14a89c27c906a5cfc93f612a69962a7e62599455a32d1e  src/ability/condition.c
3f0dd575d605f86c8431ae30f7a91567d5b728d0858090ce6fec94c46a552c9f  src/ability/cost.c
88027f844771466b37430ef1c6928b15d75b84aa1f2d11e46e3d77ffb797d156  src/ability/debug.c
4b36fb4d59247592ce18606d902359bec3cb2ac736043da069c77ab074056e2b  src/ability/describe.c
f9947e0e9669d62235fb5d48af6b92179acdb4a96d360ab9e5b104444dd625e7  src/ability/dynamic_count.c
d994d2d8fe2f1cad56559ef047876402ef83ead7d48479f82311e1be64aebb98  src/ability/effects/ability.c
c19475156b9d482484bb77809024459b99e0367b6e9794cb1c38c9ccc464dbb3  src/ability/effects/ability_effects.c
4e7d2d216d08f5049aa57094782c8a01501bafb52ae569dab21dd72e036a5465  src/ability/effects/custom.c
8137f6d97ed091d5708585d3fd04843bad15fa122f09032c4273a5f83dbb64b3  src/ability/effects/draw.c
c2a75f984bce5979ba984a5d9fe8e08ebb631363eba078597430b5dfe9c323ab  src/ability/effects/executor.c
c6aa577830f4d7fe6ffe2b9a0dbeb01d4892e5b94c270af20a26688c4a3139f5  src/ability/effects/look.c
ebd7365295221c6192bbe951b4a65016d90e30e4a8ae4d12668f890f62cffaf1  src/ability/effects/misc.c
8360911c8f088baa18f1da5041d0b653b59773ba706ac4b7b1a49d7ee3c3ec14  src/ability/effects/move.c
1fdb0e5063944e713ed1e5c68dd5d00cb7d41045086ba4cbed4b134243392fa2  src/ability/effects/move_missing.c
c735b4f536be4571ecc2ae74f752bab528eaa24221a2e32f77903d5f2e9e0be9  src/ability/effects/rules.c
c0dcc80ce8f602a5cd95e32421390c18534279a6e608f39618067da608fe548d  src/ability/effects/score.c
e00c23423ae71f4c64075dfaee7b8b123e5a386a18de66775857f784a4e3e414  src/ability/effects/state.c
86c40d1aa91ce277cc88b5c54915fa1bd35bb34cee2f9a9b2fb6dcba3fba37ad  src/ability/enums.c
e35bf2d098ebb77a975108aba881bf2d0531437e5a488dc1cffe7e5064696fdf  src/ability/log.c
1f7ae6e66ccd8a409bc87a17f5fb2daf32e67ce4f2e28c95a7e2b33219b5cdba  src/ability/resolver.c
e28f85933bdecfc531d56ab6a9993ef858adadf267fcaa4597d80c29a0ac51b4  src/ability/types.c
89730b86dbdb73ec257baa8bb8090a53fe273d1779e96d0df9e27327801ccd6f  src/ability/util.c
6fe735e7db7bba9aa982fc7722183099e5363dfa4926e24251f5d9a185fd066b  src/ability/vm.c
af996e8547f91cc1cba0efb944090f5e35dd935ec83c6a598c8ab6fa28a8feb4  src/bytecode_data.c
6e7e726e9f6cc6475d48c7e9b1a78c936781a2798cdc332ed7c145ac3563c688  src/core/alloc.c
6a93b364b2ffb2d1c5bb7312dfcfab908868f81855382222b3124f367b5e838b  src/core/alloc_counter.c
d6a085b746031a2fd23db8c0b6bf5577715323ba37f5e29b39837544df99c5cd  src/core/card.c
16edcd5f29cf1ea1c6feacc2c2f29bd03c68dda8770d01d999fd97faff74dad0  src/core/data.c
12de856cd80f71914181b47c1dd1348aeffbc4605b04ab99a1af0eca33fc44a4  src/core/game_state_abilities.c
a44c36414bff84bc97a8e183f487d4d338108a804f5178f209baeae250e38e38  src/core/game_state_core.c
59e6162d44dd80df3a5a0b06b2d58cc8ee36dd0b1a70aec272ff2e8d6a3dc918  src/core/gen_data_rom.c
a9d0fdbab49d4afadf91ee401c34c20be4e6b4046dad50bd779b145efe244366  src/core/generated/bytecode_blob.c
9bf115ffdbcc41a6931f4fd2227230e2d04cdc4a9ed673fb2326f657d178f309  src/core/generated/gen_data.c
f670b2b309c6d46bfbcd933b6f19d2d99622196076938ebd7bc8c82aeafebe97  src/core/generated/gen_data.h
b55c3684ac41463bb19035776af8b0130df62d9653d8d26ec24869564fdec2f7  src/core/generated/gen_data_cdi.c
d804483c3ebe4066ca8c78729737688e51af196cc4fabbee7b41fe061096ffc5  src/core/modifiers.c
fff33842aba091565d1b53b505e9293e453590c2daf9d043d82e97d2cd00f292  src/core/stats_pipeline.c
ee2919074f4ea1ed95c3f209a17cc22bbb1282cc83c07d8b62d5dfb8a19d7780  src/core/tracking.c
8d2c2c354e05b57c5beb8309805bd25d816b28759b19dbe5bb5bf3f7443fa04c  src/core/zones.c
58e17824fe98cf2df4d3aca2f07ce5261ae77f4efe11fd62d753a70e56a4e2a2  src/debug_umi.c
655ab567faf24ab12e8146b2463b86f980cab4faf92670d7546fd12adfabe17b  src/engine.c
94bfc202337bff1c7c0a24b6995dde3ec967b9b0362453143fa7d1463fef441b  src/game/deck_builder.c
1b7abdbd7d5e042265bfc543b62fa28bdae2ff5b19202c4fce20cc80fd108116  src/game/deck_parser.c
a830208ae3142b732a006a1dda5a063d8490559351fcec0e930e4e00e4bec9f2  src/game/game_setup.c
2377593b47e9619f8b9bd4e568551d10d3af94ae980f1ba30664b58813226a49  src/main.c
c70d96bc532710ea81807dfb5a9a3a391b0cf8979be9dc8056115dbdc7d7a2d2  src/test_game.c
f32d07255d7cba0f6556e7585256bc7b89827492c2aa8224445d60d5b77e73c0  src/turn/live.c
621779ba84987807e390b39c8ab6dcd30d95970155d46ac1b717101c9eddd707  src/turn/phase.c
b278658660bc4754790992a5ca43eac95bd67ad2396b35a8dffdc18e46e827af  src/turn/triggers.c
b42b0e395a6f02799341f8f633bdf26c681808789ca3e77255db42f4528c5c18  src/web_server.c
0a7ab513762fd6870c51ab40d660a01bed18dfa99567ed4dee6b579e764bfab6  tests/debug_eli.c
dadd1773a07223251546a09e7d666709022ea5566b151d1758fabf49b03515d2  tests/ported/ability_golden_test.c
10a77d876a9398d810d453ad6524638e9563689bd7b1f3e1414b9474b96dd44a  tests/ported/self_wait_discard_niji_live_recovery_test.c
0d2f147e8f85a4b69331dc5d2b3da4c465a5486fca628eb32477ca90875f5e6c  tests/replay.c
f5e14713a5b32bbeded40125d97902a693c2bfb62930cf9a75df18f456b123d6  tests/replay_json.h
332f748e8217a26e10eb0b383ab7d27f6af758985f8c8da8f91c38c7284332b9  tests/test_ability_effects.c
ed5b22330e31718f40a69bac6f6691007149a4626e5c5280692ce1c3858ffa91  tests/test_ability_effects_crash.c
6dc3f9d3dfc9367ac7284e6e2b6186d9ad485a8b1ffa493bd4d2d863fd00ef4b  tests/test_artifacts_consistent.c
5710c870598266156a7c3e8e352945d8dd117dd16d1fa5b5499f51bee9a4b8f2  tests/test_audit_actions.c
d5a959022c3260a13579e17664a9082568234a533bb6117dac13210b56ce89f2  tests/test_audit_conditions.c
505d99e2ac6f7df52da3a1806e6a9782241272c8d4068599bc26727f155de8c9  tests/test_audit_fields.c
94f26fb2c0067612c0866392aada3dd762ab94da1ebe9c65d9f6dfa4cd4fe982  tests/test_audit_triggers.c
704db853d2681aa244671bc01d9f2b0f9332ab4e52c6764e6b073ebe00782670  tests/test_audit_warnings.c
86bef5a847d3a7187a20613a927bfbfa88b9ce193a32e55627b6e4dfd9d00547  tests/test_audit_zones.c
c7918bdd8f1c2284bbc36c6e5301da1018dd78cdd7b56cae0d93a5fc04d56977  tests/test_b8_live_timing.c
88ad4428af2edaf55cb196dc68964854ef10da41b9656f4d8c7f499e7d170066  tests/test_baton_touch.c
7960799f69127038f7079a7e22877a169849c4a288316a26706089a2deba89dd  tests/test_choice_renderer.c
2e71635ecf405591c112f48cdd0117bf5fea7ceed00eb7f0fdfdfce450fc628c  tests/test_condition_evaluation.c
eba8cc55e608036d8b8bd6fd4e8a0b3bb0e5c3cc254ab5422c2232d3807aa376  tests/test_cross_player_interaction.c
8e44de1300791828e2a905e809010cb31ec94cb1ebe6f4ad4a31887b8f5a26a3  tests/test_deck_builder.c
356c5fa52499bb35dbc9d23f1cf344a2dd3979fd4d8ed1878f1fb83195ff5b10  tests/test_deck_parser.c
c809c1b82f623bd1a19df659fe7982702a61db80b56541faf4059a38c4c9addd  tests/test_decode_fallback.c
0dd67f4e8973189082a1e6fb2e71cebe0aaeabc2c1d2bb9299739e7669bd0c0f  tests/test_decode_skew.c
94b1bcff7deeaec4508270601987945477e02c188bdd42bdf84c1cafb0131bbb  tests/test_describe_golden.c
b093b695a0d5a9fe82cc820cb84f4f725523a318338dcac46651dd040c17d40a  tests/test_draw_phase_fix.c
f558616b9f0a1ffca4d00833c9e4d63932f83fa9fe447bf48071e5353933e266  tests/test_harness_self.c
2d5b038876fde4b9bfbf25f80f27fff7c9414ec25bd55366aa43f9aae209cf89  tests/test_live_allocator.c
f5ffd34e9b30cc9fe36aa43a8b3ef295859417a87313337443f69c14a1ac23d6  tests/test_live_card_zone_movement.c
44c2f9e0485499b27c5abafef1de2bba344611ae7bacb4a4b78e876e8dfc5538  tests/test_live_success_rules.c
ec802bee916e26eea0882997c3b1feefb4134477413eb6a89d17cee5c99e6d57  tests/test_log_drain.c
17bf30645794ccbcf73b2242fd8f3f275599c9f3416dbff8ce1d4da61b7f4c1f  tests/test_look_select.c
f7a01e6865a3fe597c0351adb8521e84224f260816c281d1123335e25594934c  tests/test_max_distinct_names.c
d11c12bb842aa5c25d759db4d385ace957244334be6b3dbc358ec0b1d1dd0670  tests/test_mechanics.c
4dedfd8f76b8a2e775332e3185ecafa9efd58e1261f90b880227ec8af70ae512  tests/test_member_activation_debut_live_start.c
8271df8cb923a1050a30d2102ae900d04e152ef25e52cdc84d870d094a1e25a9  tests/test_move_cards.c
66e383d51eb954605a68378511763a56373f45d0b5279e8ae0aa168eda67627d  tests/test_opponent_live_success_flow.c
e11a4d478f59fcaa649ab291248ccd751fee19d24aadfb7723fc612e6e3860ec  tests/test_p1_helpers.c
6488b0633e570e4b22b5b39414cd2fb94670f3904082e75fbbbff6728d79d3b1  tests/test_parity_ability_mod.c
f02d2931da5d21bd52fe642b589be254b2e79a62e763bf8ccd91cbf9afd0e0a4  tests/test_parity_ability_mod_extra.c
ef22e174745b1741564bf2b72dcc5a7737d5dd2449e0a39e5b41a6ee320ee94e  tests/test_parity_ability_watch.c
453a17388eb626c816061488c5215c0f32951b21dfc96716520a0dc102087811  tests/test_parity_abilitymod_conditional.c
2ee65b167f906bddca274087f168c47775bb27f92fb745a2271ac48a74262e06  tests/test_parity_area_move.c
6cf5bd09b8855647ab9be87f1ff13c06d78a47c6ce5c5448959cb44854ca1c01  tests/test_parity_auto_abilities.c
a169ffbfd18739f1e593c00a8ab78db7410cdd029249eb0fc2d7e457bced3100  tests/test_parity_blade_under.c
93dd9bc40bb744a4584323104abbbccf48d8e4f303823dca514a8582c9b81372  tests/test_parity_blades2.c
265e7e8ee4a87f5b8a0cbc538f10148d6e06a9584d90c6cf6735cd1e598d7221  tests/test_parity_blades_live_start.c
f006b2915c4859026ea0df8ac426ca2335b54775cc698252073c22759647556a  tests/test_parity_bp7.c
b3604d4a84ef2d05e9d1fbc5b31e8a20db20cab96890d53c66f7f881e9dd9511  tests/test_parity_card_identity.c
71c69bce7a8e759b0974e1594a5ae1de4a68e46b3e62fa3848fa5a67bf58db35  tests/test_parity_card_score.c
c62357bdffe600ae4583b7d412cf333c4cfeee2cbb05fd4370833e84c30340d8  tests/test_parity_cce_gaps.c
fe1279bf694c8aeaeec85f6bce8ed3d675cf11fd92a91b5dc73eb0b4948c5c52  tests/test_parity_char2.c
4bfca9aec6b20202e66dfdb5ddb4be72f0de5496b6fc86536500c9d8baf057a0  tests/test_parity_characterization.c
7166080efb40c570f4155b756f6cf0fdd15cced2c2465680d0a1c60f8d606e1b  tests/test_parity_characterization_extra.c
846a5eb8d7fc8dbd80502d44235efe9e5b918ec6cc95c0f983d6b92b6f22e514  tests/test_parity_cluster_d.c
57e933fcf5cdd7da7007c83a47a022675ad833f0d41d13393bf590103a9f92ff  tests/test_parity_cluster_e.c
99f87805469707255d7f4e12d80bd74901257aed1bd826d08adbc281d7bccd6c  tests/test_parity_compound_per_card.c
9c1cfd8bac206acba477d6e96fde90b68d068428fb8269942c5819e59a59ffb9  tests/test_parity_condition_and_effect.c
bef08eaf0afc577ec30c4523a1395781757df4a8e02e2ad19798db3483780646  tests/test_parity_conditional_gated.c
da4420089e4becced9532b762fc1f49281b3d9d2b2bf6601e173aedd7dca57da  tests/test_parity_conditions.c
5d4b4a0280d1f28333fac96bd20e5e04abf560204d53eb8dd8a08b86b90eab41  tests/test_parity_cost_and_effect.c
54788c54bb51fb454efdc5e9c0ebb37cba20525596b39615c9a1f4c4bd61874d  tests/test_parity_cost_compound.c
68e7bab94ea6eac78bebf5401b483be97274eac254442c2586a94f13ba5606c8  tests/test_parity_cost_discount.c
1f372e0b33fc097a99c05358a81222e95a6bcdf2c385a580495ea5bd08ea4aef  tests/test_parity_custom_misc.c
386188efa606815088b56c13579b89719e4fdb69bddc1e731fd5f9ef6d94e99a  tests/test_parity_deck_order.c
2d8715810d3f363bab7de3df7d6e73642c6ea1de2f1719b8cf51a4c3e58e1d2e  tests/test_parity_deploy_move_missing.c
c20373ebdd5cfe4943c55e5da179a0283728ddf604e3ead75ef716de58d7d2a2  tests/test_parity_deploy_multi.c
55ebef308932e83fb979a6e6ec2cc940c96c98d307f60196491b312440ab93c0  tests/test_parity_deploy_recover.c
cdc1ceddf2be206c094404612af198a81a7a9b03bc5a8fe2a2f08114d593bed8  tests/test_parity_draw_delta.c
879ad86b34eb14e6a1c37de38934d1aec63e97243e5484cdeede19bfc82dd00a  tests/test_parity_draw_flat.c
1985c3d48c0cb9bc2ec05ad9ff03fdbf88d6feb1b95727b1be8269fedc8bf07f  tests/test_parity_draw_until_count.c
13be02f46288df55ade2276767f1e233a0be4a5d9e1b35ce9e4b91050208135f  tests/test_parity_energy_position_delta.c
0056c57371e91811db4ff703580ec705f28eb341498871c7231cd9ea866fdf40  tests/test_parity_energy_remainder.c
036d2e9bbe55341f1e38b2bd94209473297b715954b512f8f45f05a6f6d342cf  tests/test_parity_enums.c
420c7e2692e220d285a3a26133c83832eb427445924d6d7432452ec269530d18  tests/test_parity_executor_dispatch.c
0f379e4c13e9d4afa5fa0e82ecefb3a34e7cc77d9dd9b366c4759b1d970e2b72  tests/test_parity_full_round.c
813be26069591698c496fa50e12809d597b22458a4588d382cebbcf7652c4404  tests/test_parity_full_round2.c
88d5b25d53dcffefd1c0ffa7b1862965966acaa0a25585101ef3293bd30f46de  tests/test_parity_full_round3.c
0495b8d45f91ac79ef13bbc4ad9c6644aea510ba4fe3b8e9fa99ef205862cfc6  tests/test_parity_fw_cost_cardtype.c
023c83beea3c87f55a7f3a748906f9d071e1e0a09c645b0170d462060a103466  tests/test_parity_fw_effects.c
db12d6c1ff088b44df15edfc7815f6ad1b492c5aff2cb36b26369159c635050e  tests/test_parity_gain_delta.c
968cdc50b3c2444ba57f9689bb14d482ba58a0bc3edfe3cfb456d42e107b2ebd  tests/test_parity_hearts_constants.c
e194273ec354639a280a28747685f3b2dc7790ebc4bdf6921e26e0b94e040755  tests/test_parity_hearts_live_start.c
3684370f6578515e51e9f2b2ef04121d7174093374ab07f65798bd19112d785c  tests/test_parity_int_gaps.c
79b9f4ec580546dd34b84d02a1d738131a255308aee0c475cd2dddc5d971febb  tests/test_parity_integration.c
139780157a9b46315e6f0d274841424bf5471ef6588e1b2a44b2995cc2160077  tests/test_parity_integration_extra.c
bc1187ffc320ec369005e9db5b6a538c6c19733f317f1144eeb549a47a2349b9  tests/test_parity_integration_rounds.c
a5ce692d3afd1136943df656c3ed6b0a8956fdff7d4223558beb94755ff75821  tests/test_parity_jidou.c
f34857d54603130b8c59f76def5a46f0290a6b8f81df5e7938b57d3e617368ce  tests/test_parity_jidou_extra.c
e97c3ba90829a02c401e856cab9e5746b63d4ba0f34f36dbef03c82926d516fd  tests/test_parity_jidou_life.c
04fbd0d4af3da8ed022f56571ffd8d1af736c2cf929983e6b1dcf0c75ac87e1e  tests/test_parity_jidou_move_yell.c
8f41254f72ea22a260682eb2bf4295308704cd6a27f0dc51bf3fd1e2453d2bdc  tests/test_parity_jidou_state.c
a3c3d6518604e17de0b7a1c44258360498083039f34d1bb5b4d5a0b452bd035f  tests/test_parity_jidou_watch.c
e87d1eb735232a87a244396f34d3462e4b8eb2e3e0e53b1b8ddf76d6dbdc8428  tests/test_parity_jidou_watch2.c
a02b261b46a9be28e169ea05c236946e7bc991501e512a9f975acc267abde643  tests/test_parity_jidou_watch_clusters.c
292b0fb5312ff849b9660a615dcf7573c2dca0220d0c078c150c9b44686dcb0a  tests/test_parity_live_total.c
ec782143b14e04645ff6cbdc27ea2dbbfa0188ccf82464c8f24f150547e42e52  tests/test_parity_live_total2.c
e5bffa8a8bfd42f46481c92f76a7ef55318838e00a96a2557d45645f167032aa  tests/test_parity_look_and_filter.c
dc6eecc2b04ba7724259636d0e7edd75a9b95738a5d2b4d9f9c884468159670b  tests/test_parity_look_per_card.c
f37101f96a33b49b3fef33de4406004dfbabb404b085a1139a99ce639a93585c  tests/test_parity_look_reveal.c
c448d8e8493dfac2348dfee3cf9ef5fe5e21c932450eb9c842be3ca38667ff68  tests/test_parity_mill_deck.c
5fd81294110a429f41dfe1870424cabf02242e96a7c28e33772631ea14615fa3  tests/test_parity_parser_e2e_extra.c
bd1303b956c44581667f1b7b92b40ce21b517398faef551a98b9bdf3c0c2cf0b  tests/test_parity_per_card.c
43e23e316bd332f9f3f56af1401384d3d4488dd5491dfab3f494a2f451f5dbfc  tests/test_parity_per_card2.c
bcf6cc7668f0fe066cf18bee42e5dafc489e05dd648aff17e5c165f2875c2f04  tests/test_parity_per_card3.c
030fef087626321d55d7f5987402e07cdfe461bbb770c09c6084c7992f82ae4f  tests/test_parity_queue_resume.c
412bad7ac9e58088dc685fa9def44d1d6ca4ca845b0d40b151bc0b0482bae69b  tests/test_parity_realcard_walkthrough.c
72009e631bdd4af108e5c25f310f1c17887a2109a18a29e98f8db39af7f96440  tests/test_parity_recover_hand_a.c
30a286a8fc7d292d966d2ffcbf48e787cc80f938d5ae30c843dcdc75f7bc86a3  tests/test_parity_recover_live.c
71c01ab1940fa5eb5c0da1ac4449eb4996764574116f3c87dd2517b89d9d6cf7  tests/test_parity_recover_q79.c
0c8074ed51d1bd369f05a6ccf9e99bd9d1f916430906336e1b2a59ba6b95f830  tests/test_parity_recover_score_state_delta.c
08f56994df2291704eafcae4d9158e935c3e75e19591a6d8435e75044dff908a  tests/test_parity_recover_to_hand.c
493a5780ba74f6df600a192e2b3a3694da5f4c96cca69e5edf98467cf692a794  tests/test_parity_required_hearts.c
a755388c839d350b20a694165c66aaf14bdc949b452039539779cdcc895fff93  tests/test_parity_restriction.c
fee12dd0d2ef2923f1abbc8194829ab5a40508eb6b151c3dee73303c812e32cb  tests/test_parity_restriction_invalidate.c
5fc8112310847cebc906783699c527d71085a7ae45fdcffc033b0d2ac56700da  tests/test_parity_reveal_extra.c
75919819dd2cf186d884d461c92f0c7d2a9747848594a86f2a44543eeb5dc0de  tests/test_parity_rules.c
d124e02e4a177fe207599253810a398fcb44efc294b8de41ecbd799848136e4f  tests/test_parity_rules_conditions.c
5030709b0e14fdc74d09f42c0550a47b74fe90701e5e4e5b9c3a96793d0f5abf  tests/test_parity_rules_conditions2.c
00b9e5c30a79317ce6417bd7be5ea440936a89603fff0b78fc2031cb59a32e48  tests/test_parity_rules_extra.c
010249f94ecb7e1cff53d6ef424933e94b3564b5b845851e66dec8ad54c90eda  tests/test_parity_rules_phases.c
e52c608d509e583df5085f60fc98d46e2ebc1d72896f5d96e3d531d74f78046f  tests/test_parity_score_effects.c
e8e9913b2162ee1b36081d779acab676b158812cfe179e7b81c9556c7847ef3d  tests/test_parity_score_per_card2.c
c9ef3f4b064c9d129340e72aa89b04b086292a2f943c520d5a96747a564fd057  tests/test_parity_search_reorder.c
40975741587a64483bfcce19c3b458ae2b388e46b2902bacf4862ba80a13c08e  tests/test_parity_sequential.c
b9cb52fc33eec21151c6ce86b4a1762ac8b9712b822245f07eff892e66064dc2  tests/test_parity_small_clusters.c
61f8593149b14f4087b728058aee78fff7142208f9178fdd5fb4dd439ef50018  tests/test_parity_state_effects2.c
271a295e898bc6377f1fb25908a9a3a6521d60a3f5fb3ffb098597219f903a89  tests/test_parity_state_family.c
0d0f5363f79f5f0fc3adf1967f71da79095ce3df5e123aa2cdfb25b7a15a3de9  tests/test_parity_state_per_card.c
ef27358e4c1b18db5baed523b92a334355a166ea597e7f8314ac93d5fe626b82  tests/test_parity_stats_pipeline.c
c9cae597772f0c7aae887206230bae98c739463d512b068f6bfbaa66c535b777  tests/test_parity_support.c
c88aecf0fb382bbe2f6f800b9ad116dd57ac05ef16cac289379e1a7caf6cce43  tests/test_parity_t6_action_setup.c
3454d450e4eb25e77cd50a39ea74edbc6cde307b64d9128ddaffeae20824bef1  tests/test_parity_t7_condition_decoder.c
b86dc0de73cfdf35d1c6aad5136c34440c480aa6aa166ce20a96e4f5e19fa07f  tests/test_parity_target_player.c
fe7a88e2c4f4ea10d44baa906731d1e95b0cdf3ac33e7ea32c0e9d85ba5ed6b0  tests/test_parity_trigger_paths.c
f84dacb23b1f7c7774e395d7885a0d16bc8c54fbc73825ca6fbb2219fd4a43d8  tests/test_parity_under_member.c
8794102738c43a35cf9219a427942467dbdc53feb99b4afea8e2a8e916b73dd2  tests/test_parity_wait_activation.c
89ea86449c2e8b3cf9f030ee673071f4b51b994996b2ac55b808f23f41e0d1b2  tests/test_parity_wave1a.c
d602209ee85e838ecda9edc91e6e9ff74dbf27217a528e26533d5afaaa859d89  tests/test_parity_wave1b.c
13b60a1e7d3f7994cdb6dae09d2fff99c1ec48c838f7aece1a0979274b09054a  tests/test_parity_wave1c.c
77350c73c9f381310f72d38c7bdc958dfac32763699b92ca74ca2e913df50cb4  tests/test_parity_x_class.c
30dddb45d152ed6a70826757cb04b1cdf8bae7e4db681269d2be77991946908a  tests/test_parity_yell.c
6fde95b7832d2340d7730527abba8a6bf5ff4109eedd1ae49e7da4240d25cd3c  tests/test_parity_zone_source.c
7520be285bf56ea9dd2f2f9f27a9c9d80e40f56cb258ef28ce0ca3348c4facd9  tests/test_parity_zones_position.c
e078c2685454bf686cd3a4874fd3ebb6d14776971be9be54fc4b994db95c5acb  tests/test_performance_phase_rules.c
d179f99a0207587c7d49edae1012f534e56a44c64b5f311a6f74658298b4dfd5  tests/test_phase_machine_rules.c
9bdcf9956e444112086780f6fa9016ae94368a86926ab82717c25c58adac53e7  tests/test_ported_generated.c
9c2c5af1fdd34a2fb0a24feb1f6af91b685d19dfb325978702e746aefd2a3486  tests/test_position_and_source.c
0c874f6cad3c53c5af60724ae23102432fab1630fe0c9aa34297b6cf5188ee96  tests/test_restriction_mechanics.c
71250d5d26776099842e4f91dd3e5a24136597371649f1a4ccad3a37a9cb3db1  tests/test_state_effects.c
5b31835318bc154d93f9627694e7b161911836b17b320d695575a12c155af3fd  tests/test_surplus_heart.c
3f0dd2c341c33b8c1a735f96464702a24e126a84c35bdf89beca8235748f17f6  tests/test_survey_probe.c
a7e87a2e395be6a5799d560af06703a238bfcc023723f095428e68d31f3cb03a  tests/test_target_selection.c
d0cb8319033c67fc8eb1c215bfcdd2a2b2b2a826b8c4ccb207f2d0032133430c  tests/test_tp_activation_offer.c
d2f9a37075d4429657907ca97e681d4e104cd844febea2090a578e0ff44209fb  tests/test_tp_joint_card_live_start.c
b3d213dfa2f30cb3a7d93a2b9d89b25542b75f21ec59a0095a4d4f6559b25a78  tests/test_tp_turn_limited_activation.c
586590bfed9bb019bfd8f31fc206df16edf2f16e2a67d85850588a351f766bd5  tests/test_tp_zone_change_gate.c
94c0a6327f77dcdcfe656f22db0a1c3f37c25f87e11c90aaead317c92fe354fa  tests/test_trigger_paths.c
d99058ee49988396bb1d2b1f3c226aaf922fb8582ce0def4b2348864ee836709  tests/test_vm_decode.c
2f1fbd35a8c116c4ea3d35f0a795b92bac31681eab2ddc2c256c6c69b65efe4b  tests/test_zone_filter_parity.c
fab2c25c1631f2f703c75f707eede963014f01e1a3332da189f53609a1ad6ce1  tests/test_zzprobe.c
7d6005e93481398c77748859d3e501b57a8c69817446afc623e18d5be238c5f0  tests/tmp_list.c
11fec57d6848151a82e6a4a8945d93f4f463bc34841b6addbdc362c0e6420b8a  tools/_baseline_head_port_one.py
766d3aa0ac9491ee3f0938437ab02bafcec1fcf4c4039b657bc77085c0ac41d6  tools/audit_actions.c
c0ee1678b712824fe7b70978d26a6e15a6651b309bb809e4a6d8a99b6eb49edb  tools/audit_cluster_d.c
6f794c9e9e2e32f873929615a42fb6b0b669fcdee474248a11326c2468f4ffb3  tools/audit_placeholders.py
c37e6832efea4c8be9f80768da2199bb7dc153d43c21e272832a3754f9ee0f94  tools/audit_t7_decoders.c
4be77545d928e924aa4658443e25583fd783a330d5622ad1c926ad348cab528e  tools/audit_t7_resource.c
813e62b0fda7714c30fccef04a60354b6807aa14e443a1607cef526de0d4508c  tools/audit_t7_wire.c
09c804c34480ae29bde324f7ec3e10b6ed6be16d1a1b3d46178a5cbba89e5ea8  tools/audit_t7_zeroheart.c
06373dbdde0d6039fcbb93ab154d0ba33b6d75b3c0963898f4e9223e526eddb1  tools/audit_w_gate.c
920e16f260eb04b4552f11191a64eecf66e1e624df45d7a117809798c5c27797  tools/build_audit_zones.sh
5d4df22dc2901c660a3e01592405efd149c24ab9d1bc4ef09fbd9be3e99425ac  tools/census_e.c
a7957dfb8b051bf6a3a43ce44c50ceb6d7c8849bf64701202842240fb37c92bc  tools/census_fw.c
f1dd7dd594a8a0e06a5d0bbde9b847293793c4b1051a48c52e7cdbe2217c043d  tools/dep_audit.py
968bc9aa8ca3ef7564f4500250f23c8887aac11a7e5517460be1703c85577b1b  tools/dump_cost.c
3d73e7d8ec2a2f271c78a63a09d69f464eb4790b90b9012610bc2da2cf5b5afc  tools/find_distinct_fixtures.c
e71e879569a82a802297f8309204488edd91b6442e050837bb029026e08e74e7  tools/gen_bytecode.py
ad07b8154cc421fd64aa8b7136e5826e332fbf58a92131775b951397eab8d1b1  tools/gen_from_rs.py
144f27ae92dfd8c51363093f52e0e76bfa6120b4aeff235a1d6fecd96dc9f01f  tools/gen_gen_data_bin.py
d869a284c4b2393e579ee11c9e748a1a3bbbc62c6db0f6f044de336e48f4a610  tools/guard_data_blob.sh
11b080646ba1d6bf49e3619f4f802c777d7b4791a78f6103a7e8f21d05ebecae  tools/isolated_build.sh
700e046cb8dad809cddf684e46ed34bb9752217dcee9cc8df1e58b4473677eda  tools/port_one.py
56e7817cc109f83852864f75e04ce0846f68647c92b53e1564c6f7382d4aea4e  tools/regen_all.sh
5615248fa4ee885914f6bbb269506b2ef0bddedf178c4dde5c6df6e2755053d8  tools/size_audit.py
68fd303cbceae786f572d758967208d309ebe9887b84b51a506d9eb5e0e034cb  tools/verify_artifacts.py
6ac42869d7c7956fc0444e3b613349f8b5ab3625b56cef8b9c80455afffb7721  tools/warnings_audit.sh
0ca5efcd0f9d066d7789065144954704545d949168d1ee8c6414fe6298678065  tools/warnings_audit_tests.sh
```

---

---

## 12b. PER-SUITE TABLE (run 1; run 2 is identical)

`tests/verdicts` counts every `ok` / `ok:` / `FAILED` / `CRASH` line, i.e. lines a
verdict-aware scanner can key a test name from. `FAIL assertions` is the comparable
2 466-scope column. `baseline FAIL` and `delta` are the §9 line-level diff.

| suite | shape | verdict lines | FAILED | CRASH | FAIL assertions | FAIL (VACUOUS) | `ok:` echoes | distinct FAIL texts | ms run1 | ms run2 | colour | baseline FAIL | delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ability_effects` | FLAT | 12 | 0 | 0 | 0 | 0 | 12 | 0 | 481 | 117 | GREEN | 0 | +0 |
| `ability_effects_crash` | FORK | 9 | 0 | 0 | 0 | 0 | 0 | 0 | 731 | 516 | GREEN | 0 | +0 |
| `artifacts_consistent` | DIAG | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 157 | 117 | GREEN_UNVERIFIED | 0 | +0 |
| `audit_actions` | FLAT | 48 | 2 | 0 | 2 | 0 | 46 | 2 | 348 | 392 | RED | 2 | +0 |
| `audit_conditions` | FLAT | 17 | 5 | 0 | 2 | 0 | 5 | 2 | 1168 | 882 | RED | 2 | +0 |
| `audit_fields` | FORK | 16 | 2 | 0 | 2 | 0 | 0 | 2 | 566 | 671 | RED | 3 | -1 |
| `audit_triggers` | FORK | 11 | 2 | 0 | 2 | 0 | 9 | 2 | 629 | 395 | RED | 5 | -3 |
| `audit_warnings` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 485 | 274 | GREEN_UNVERIFIED | NEW | NEW |
| `audit_zones` | FLAT | 255 | 2 | 0 | 3 | 0 | 253 | 3 | 929 | 1002 | RED | 3 | +0 |
| `b8_live_timing` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 164 | 97 | GREEN_UNVERIFIED | 0 | +0 |
| `baton_touch` | FLAT | 18 | 0 | 0 | 0 | 0 | 18 | 0 | 217 | 121 | GREEN | 0 | +0 |
| `choice_renderer` | FLAT | 65 | 0 | 0 | 0 | 0 | 65 | 0 | 156 | 87 | GREEN | NEW | NEW |
| `condition_evaluation` | FLAT | 25 | 2 | 0 | 3 | 0 | 23 | 3 | 221 | 176 | RED | 3 | +0 |
| `cross_player_interaction` | FLAT | 20 | 0 | 0 | 1 | 0 | 20 | 1 | 162 | 120 | RED | 1 | +0 |
| `deck_builder` | FLAT | 62 | 0 | 0 | 0 | 0 | 62 | 0 | 132 | 112 | GREEN | 0 | +0 |
| `deck_parser` | FLAT | 63 | 0 | 0 | 0 | 0 | 63 | 0 | 109 | 80 | GREEN | 0 | +0 |
| `decode_fallback` | DIAG | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 126 | 80 | GREEN | 0 | +0 |
| `decode_skew` | FLAT | 16 | 0 | 0 | 0 | 0 | 0 | 0 | 274 | 189 | GREEN | 0 | +0 |
| `describe_golden` | COUNTERS | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 535 | 417 | GREEN_UNVERIFIED | NEW | NEW |
| `draw_phase_fix` | COUNTERS | 2 | 0 | 0 | 0 | 0 | 2 | 0 | 146 | 96 | GREEN | 0 | +0 |
| `harness_self` | FLAT | 79 | 1 | 0 | 6 | 0 | 78 | 6 | 567 | 379 | RED | NEW | NEW |
| `live_allocator` | FLAT | 38 | 0 | 0 | 0 | 0 | 38 | 0 | 134 | 95 | GREEN | 0 | +0 |
| `live_card_zone_movement` | FLAT | 24 | 0 | 0 | 12 | 0 | 24 | 12 | 143 | 113 | RED | 0 | +12 |
| `live_success_rules` | FLAT | 73 | 0 | 0 | 0 | 0 | 73 | 0 | 211 | 211 | GREEN | 0 | +0 |
| `log_drain` | FLAT | 60 | 0 | 0 | 0 | 0 | 60 | 0 | 184 | 130 | GREEN | 0 | +0 |
| `look_select` | FLAT | 49 | 2 | 0 | 2 | 0 | 47 | 2 | 155 | 109 | RED | 2 | +0 |
| `max_distinct_names` | FLAT | 4043 | 0 | 0 | 6 | 0 | 4043 | 6 | 658 | 651 | RED | 6 | +0 |
| `mechanics` | FLAT | 25 | 0 | 0 | 5 | 0 | 25 | 5 | 141 | 97 | RED | 0 | +5 |
| `member_activation_debut_live_start` | FLAT | 17 | 0 | 0 | 0 | 0 | 17 | 0 | 143 | 114 | GREEN | 0 | +0 |
| `move_cards` | FLAT | 14 | 0 | 0 | 2 | 0 | 14 | 2 | 136 | 77 | RED | 2 | +0 |
| `opponent_live_success_flow` | FLAT | 4 | 0 | 0 | 0 | 0 | 4 | 0 | 153 | 99 | GREEN | 0 | +0 |
| `p1_helpers` | FLAT | 0 | 0 | 0 | 1 | 0 | 0 | 1 | 182 | 75 | RED | 1 | +0 |
| `parity_ability_mod` | FLAT | 84 | 8 | 0 | 14 | 0 | 76 | 14 | 245 | 165 | RED | 14 | +0 |
| `parity_ability_mod_extra` | FLAT | 188 | 0 | 0 | 17 | 0 | 188 | 15 | 310 | 237 | RED | 17 | +0 |
| `parity_ability_watch` | FORK | 324 | 8 | 0 | 19 | 0 | 292 | 19 | 2649 | 2598 | RED | 20 | -1 |
| `parity_abilitymod_conditional` | FLAT | 763 | 5 | 0 | 8 | 0 | 758 | 8 | 898 | 681 | RED | 8 | +0 |
| `parity_area_move` | FORK | 40 | 20 | 0 | 31 | 0 | 0 | 31 | 3933 | 3810 | RED | 32 | -1 |
| `parity_auto_abilities` | COUNTERS | 87 | 0 | 0 | 1 | 0 | 87 | 1 | 141 | 190 | RED | 1 | +0 |
| `parity_blade_under` | FORK | 47 | 20 | 0 | 30 | 0 | 0 | 30 | 5127 | 7969 | RED | 30 | +0 |
| `parity_blades2` | FORK | 28 | 10 | 0 | 16 | 0 | 0 | 16 | 2436 | 2621 | RED | 16 | +0 |
| `parity_blades_live_start` | FLAT | 202 | 0 | 0 | 31 | 0 | 202 | 31 | 340 | 381 | RED | 34 | -3 |
| `parity_bp7` | FORK | 29 | 12 | 0 | 22 | 0 | 0 | 22 | 2322 | 1982 | RED | 24 | -2 |
| `parity_card_identity` | FLAT | 151 | 0 | 0 | 0 | 0 | 151 | 0 | 425 | 342 | GREEN | 0 | +0 |
| `parity_card_score` | FLAT | 325 | 6 | 0 | 21 | 0 | 319 | 17 | 904 | 1081 | RED | 21 | +0 |
| `parity_cce_gaps` | FORK | 68 | 35 | 0 | 51 | 2 | 0 | 50 | 8326 | 7938 | RED | 53 | -2 |
| `parity_char2` | FLAT | 136 | 2 | 0 | 3 | 0 | 121 | 3 | 1946 | 1718 | RED | 3 | +0 |
| `parity_characterization` | FLAT | 0 | 0 | 0 | 3 | 0 | 0 | 3 | 485 | 405 | RED | 3 | +0 |
| `parity_characterization_extra` | FORK | 54 | 18 | 0 | 25 | 0 | 0 | 25 | 2400 | 2378 | RED | 26 | -1 |
| `parity_cluster_d` | FLAT | 55 | 0 | 0 | 0 | 0 | 55 | 0 | 136 | 131 | GREEN | NEW | NEW |
| `parity_cluster_e` | FLAT | 60 | 0 | 0 | 0 | 0 | 60 | 0 | 136 | 104 | GREEN | NEW | NEW |
| `parity_compound_per_card` | FORK | 47 | 23 | 0 | 41 | 0 | 0 | 41 | 4508 | 4148 | RED | 41 | +0 |
| `parity_condition_and_effect` | COUNTERS | 343 | 0 | 0 | 48 | 0 | 343 | 48 | 240 | 192 | RED | 52 | -4 |
| `parity_conditional_gated` | FLAT | 315 | 25 | 0 | 44 | 0 | 290 | 39 | 3517 | 3415 | RED | 46 | -2 |
| `parity_conditions` | DIAG | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 154 | 146 | GREEN_UNVERIFIED | 0 | +0 |
| `parity_cost_and_effect` | COUNTERS | 14 | 0 | 0 | 1 | 0 | 14 | 1 | 135 | 124 | RED | 1 | +0 |
| `parity_cost_compound` | COUNTERS | 98 | 0 | 0 | 1 | 0 | 98 | 1 | 145 | 108 | RED | 0 | +1 |
| `parity_cost_discount` | FORK | 91 | 30 | 0 | 47 | 0 | 5 | 45 | 12435 | 12638 | RED | 47 | +0 |
| `parity_custom_misc` | FLAT | 80 | 1 | 0 | 2 | 0 | 79 | 2 | 161 | 182 | RED | 2 | +0 |
| `parity_deck_order` | FLAT | 101 | 0 | 0 | 0 | 0 | 101 | 0 | 148 | 134 | GREEN | NEW | NEW |
| `parity_deploy_move_missing` | COUNTERS | 66 | 0 | 0 | 7 | 0 | 66 | 7 | 154 | 120 | RED | 7 | +0 |
| `parity_deploy_multi` | FLAT | 22 | 11 | 0 | 21 | 0 | 0 | 21 | 1380 | 1416 | RED | 21 | +0 |
| `parity_deploy_recover` | FLAT | 305 | 0 | 0 | 24 | 0 | 305 | 24 | 299 | 247 | RED | 24 | +0 |
| `parity_draw_delta` | FLAT | 152 | 0 | 0 | 0 | 0 | 152 | 0 | 160 | 114 | GREEN | 0 | +0 |
| `parity_draw_flat` | FLAT | 550 | 0 | 0 | 57 | 0 | 550 | 54 | 389 | 436 | RED | 74 | -17 |
| `parity_draw_until_count` | FLAT | 69 | 0 | 0 | 0 | 0 | 69 | 0 | 147 | 122 | GREEN | 0 | +0 |
| `parity_energy_position_delta` | FLAT | 7 | 4 | 0 | 6 | 0 | 0 | 6 | 412 | 363 | RED | 6 | +0 |
| `parity_energy_remainder` | FORK | 7 | 3 | 0 | 4 | 0 | 0 | 4 | 443 | 359 | RED | 4 | +0 |
| `parity_enums` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 148 | 93 | GREEN_UNVERIFIED | 0 | +0 |
| `parity_executor_dispatch` | FLAT | 119 | 0 | 0 | 0 | 0 | 119 | 0 | 216 | 162 | GREEN | 0 | +0 |
| `parity_full_round` | FLAT | 165 | 15 | 0 | 29 | 0 | 0 | 29 | 2716 | 2761 | RED | 29 | +0 |
| `parity_full_round2` | FLAT | 289 | 14 | 0 | 54 | 0 | 0 | 54 | 1047 | 775 | RED | 53 | +1 |
| `parity_full_round3` | FLAT | 337 | 5 | 0 | 12 | 0 | 0 | 12 | 1435 | 1131 | RED | 12 | +0 |
| `parity_fw_cost_cardtype` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 332 | 219 | GREEN_UNVERIFIED | NEW | NEW |
| `parity_fw_effects` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 270 | 249 | GREEN_UNVERIFIED | NEW | NEW |
| `parity_gain_delta` | FORK | 16 | 2 | 0 | 2 | 0 | 0 | 2 | 1093 | 820 | RED | 2 | +0 |
| `parity_hearts_constants` | FLAT | 138 | 0 | 0 | 18 | 0 | 138 | 18 | 222 | 129 | RED | 19 | -1 |
| `parity_hearts_live_start` | FLAT | 129 | 0 | 0 | 67 | 0 | 129 | 60 | 794 | 258 | RED | 64 | +3 |
| `parity_int_gaps` | FORK | 15 | 11 | 0 | 30 | 0 | 0 | 30 | 2328 | 806 | RED | 25 | +5 |
| `parity_integration` | FLAT | 147 | 12 | 0 | 30 | 0 | 132 | 30 | 538 | 314 | RED | 31 | -1 |
| `parity_integration_extra` | FLAT | 144 | 22 | 0 | 47 | 0 | 0 | 47 | 5413 | 4170 | RED | 47 | +0 |
| `parity_integration_rounds` | FLAT | 94 | 6 | 0 | 19 | 0 | 0 | 15 | 526 | 528 | RED | 19 | +0 |
| `parity_jidou` | FLAT | 76 | 0 | 0 | 30 | 0 | 66 | 29 | 170 | 167 | RED | 26 | +4 |
| `parity_jidou_extra` | FLAT | 87 | 0 | 0 | 25 | 0 | 87 | 23 | 164 | 156 | RED | 25 | +0 |
| `parity_jidou_life` | FORK | 221 | 9 | 0 | 22 | 0 | 207 | 22 | 1000 | 706 | RED | 22 | +0 |
| `parity_jidou_move_yell` | FORK | 88 | 37 | 0 | 47 | 0 | 0 | 46 | 12374 | 13102 | RED | 49 | -2 |
| `parity_jidou_state` | FLAT | 159 | 12 | 0 | 17 | 0 | 147 | 17 | 4090 | 4143 | RED | 17 | +0 |
| `parity_jidou_watch` | FORK | 293 | 23 | 0 | 54 | 0 | 236 | 54 | 6962 | 6952 | RED | 56 | -2 |
| `parity_jidou_watch2` | FORK | 337 | 22 | 0 | 46 | 0 | 306 | 46 | 2432 | 2936 | RED | 46 | +0 |
| `parity_jidou_watch_clusters` | FORK | 405 | 15 | 0 | 47 | 0 | 370 | 47 | 2939 | 2497 | RED | 47 | +0 |
| `parity_live_total` | FLAT | 204 | 20 | 0 | 33 | 0 | 184 | 30 | 3877 | 4460 | RED | 33 | +0 |
| `parity_live_total2` | FLAT | 308 | 7 | 0 | 10 | 0 | 301 | 9 | 4335 | 3136 | RED | 10 | +0 |
| `parity_look_and_filter` | FORK | 65 | 16 | 0 | 21 | 2 | 0 | 20 | 4146 | 2885 | RED | 27 | -6 |
| `parity_look_per_card` | FORK | 48 | 20 | 2 | 30 | 14 | 0 | 30 | 2656 | 2435 | RED | 37 | -7 |
| `parity_look_reveal` | FLAT | 154 | 1 | 0 | 1 | 0 | 153 | 1 | 141 | 107 | RED | 1 | +0 |
| `parity_mill_deck` | FORK | 170 | 29 | 0 | 53 | 0 | 109 | 53 | 6549 | 6479 | RED | 53 | +0 |
| `parity_parser_e2e_extra` | FORK | 17 | 6 | 0 | 8 | 2 | 0 | 8 | 947 | 1094 | RED | 8 | +0 |
| `parity_per_card` | FORK | 21 | 5 | 0 | 15 | 0 | 0 | 15 | 1385 | 1221 | RED | 15 | +0 |
| `parity_per_card2` | FORK | 25 | 6 | 0 | 8 | 0 | 19 | 8 | 1732 | 1751 | RED | 10 | -2 |
| `parity_per_card3` | FORK | 21 | 9 | 0 | 13 | 0 | 0 | 12 | 1197 | 952 | RED | 13 | +0 |
| `parity_queue_resume` | FLAT | 145 | 2 | 0 | 3 | 0 | 143 | 3 | 168 | 110 | RED | 1 | +2 |
| `parity_realcard_walkthrough` | FLAT | 78 | 2 | 0 | 5 | 0 | 0 | 5 | 387 | 381 | RED | 5 | +0 |
| `parity_recover_hand_a` | FORK | 23 | 9 | 0 | 16 | 4 | 14 | 16 | 788 | 870 | RED | 17 | -1 |
| `parity_recover_live` | FORK | 64 | 21 | 0 | 35 | 1 | 0 | 35 | 3348 | 3416 | RED | 35 | +0 |
| `parity_recover_q79` | FORK | 24 | 8 | 0 | 4 | 6 | 16 | 4 | 879 | 655 | RED | 6 | -2 |
| `parity_recover_score_state_delta` | FORK | 165 | 18 | 0 | 28 | 0 | 147 | 28 | 1419 | 1437 | RED | 28 | +0 |
| `parity_recover_to_hand` | FLAT | 103 | 0 | 0 | 10 | 0 | 103 | 8 | 140 | 127 | RED | 10 | +0 |
| `parity_required_hearts` | FORK | 235 | 12 | 0 | 16 | 0 | 180 | 16 | 7963 | 8492 | RED | 16 | +0 |
| `parity_restriction` | FORK | 126 | 19 | 0 | 43 | 0 | 95 | 33 | 2948 | 2119 | RED | 43 | +0 |
| `parity_restriction_invalidate` | FORK | 6 | 0 | 0 | 0 | 0 | 0 | 0 | 407 | 278 | GREEN | 0 | +0 |
| `parity_reveal_extra` | FLAT | 440 | 0 | 0 | 20 | 0 | 440 | 20 | 230 | 277 | RED | 20 | +0 |
| `parity_rules` | FLAT | 160 | 0 | 0 | 11 | 0 | 157 | 10 | 172 | 140 | RED | 11 | +0 |
| `parity_rules_conditions` | FLAT | 116 | 3 | 0 | 7 | 0 | 113 | 7 | 644 | 675 | RED | 6 | +1 |
| `parity_rules_conditions2` | FLAT | 123 | 4 | 0 | 16 | 0 | 119 | 16 | 730 | 835 | RED | 0 | +16 |
| `parity_rules_extra` | FORK | 16 | 2 | 0 | 4 | 0 | 0 | 4 | 705 | 831 | RED | 4 | +0 |
| `parity_rules_phases` | FLAT | 216 | 10 | 0 | 18 | 0 | 205 | 18 | 3105 | 3380 | RED | 18 | +0 |
| `parity_score_effects` | FLAT | 107 | 0 | 0 | 1 | 0 | 107 | 1 | 182 | 168 | RED | 1 | +0 |
| `parity_score_per_card2` | FLAT | 140 | 1 | 0 | 1 | 0 | 139 | 1 | 801 | 826 | RED | 1 | +0 |
| `parity_search_reorder` | FORK | 311 | 36 | 0 | 112 | 0 | 230 | 83 | 8934 | 9220 | RED | 122 | -10 |
| `parity_sequential` | FORK | 52 | 29 | 0 | 113 | 0 | 0 | 109 | 7364 | 7494 | RED | 113 | +0 |
| `parity_small_clusters` | FORK | 887 | 40 | 0 | 64 | 0 | 847 | 64 | 17534 | 20935 | RED | 70 | -6 |
| `parity_state_effects2` | FLAT | 102 | 3 | 0 | 3 | 0 | 99 | 3 | 167 | 134 | RED | 3 | +0 |
| `parity_state_family` | COUNTERS | 39 | 1 | 0 | 1 | 0 | 38 | 1 | 165 | 129 | RED | NEW | NEW |
| `parity_state_per_card` | FORK | 82 | 33 | 0 | 45 | 0 | 0 | 44 | 10904 | 11534 | RED | 46 | -1 |
| `parity_stats_pipeline` | COUNTERS | 66 | 9 | 0 | 31 | 0 | 57 | 29 | 150 | 151 | RED | 31 | +0 |
| `parity_support` | FLAT | 67 | 1 | 0 | 1 | 0 | 0 | 1 | 662 | 852 | RED | 1 | +0 |
| `parity_t6_action_setup` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 373 | 274 | GREEN_UNVERIFIED | NEW | NEW |
| `parity_t7_condition_decoder` | DIAG | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 149 | 104 | GREEN | NEW | NEW |
| `parity_target_player` | FORK | 46 | 27 | 0 | 48 | 8 | 0 | 48 | 2634 | 2417 | RED | 54 | -6 |
| `parity_trigger_paths` | FORK | 321 | 15 | 0 | 23 | 0 | 282 | 23 | 2866 | 2835 | RED | 23 | +0 |
| `parity_under_member` | FORK | 83 | 30 | 0 | 57 | 0 | 0 | 57 | 11999 | 12465 | RED | 60 | -3 |
| `parity_wait_activation` | FLAT | 251 | 0 | 0 | 39 | 0 | 251 | 36 | 766 | 273 | RED | 41 | -2 |
| `parity_wave1a` | FORK | 21 | 13 | 0 | 23 | 2 | 0 | 23 | 3445 | 1234 | RED | 23 | +0 |
| `parity_wave1b` | FORK | 31 | 11 | 0 | 21 | 0 | 0 | 21 | 2819 | 2084 | RED | 39 | -18 |
| `parity_wave1c` | FORK | 20 | 10 | 0 | 31 | 3 | 0 | 24 | 1580 | 1192 | RED | 31 | +0 |
| `parity_x_class` | FORK | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 351 | 287 | GREEN | NEW | NEW |
| `parity_yell` | FORK | 15 | 14 | 0 | 39 | 0 | 0 | 34 | 1163 | 978 | RED | 39 | +0 |
| `parity_zone_source` | FLAT | 470 | 27 | 0 | 42 | 0 | 443 | 32 | 6902 | 7030 | RED | 51 | -9 |
| `parity_zones_position` | FLAT | 139 | 0 | 0 | 37 | 0 | 139 | 33 | 221 | 145 | RED | 38 | -1 |
| `performance_phase_rules` | FLAT | 36 | 0 | 0 | 2 | 0 | 36 | 2 | 223 | 147 | RED | 2 | +0 |
| `phase_machine_rules` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 156 | 136 | GREEN_UNVERIFIED | 0 | +0 |
| `ported_generated` | FLAT | 80 | 2 | 0 | 3 | 0 | 78 | 3 | 289 | 150 | RED | 3 | +0 |
| `position_and_source` | FORK | 17 | 0 | 0 | 0 | 0 | 0 | 0 | 600 | 734 | GREEN | 0 | +0 |
| `restriction_mechanics` | FLAT | 18 | 0 | 0 | 0 | 0 | 18 | 0 | 190 | 157 | GREEN | 0 | +0 |
| `state_effects` | FLAT | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 165 | 161 | GREEN_UNVERIFIED | 0 | +0 |
| `surplus_heart` | FLAT | 13 | 1 | 0 | 1 | 0 | 12 | 1 | 182 | 159 | RED | 1 | +0 |
| `survey_probe` | FORK | 4 | 1 | 0 | 1 | 0 | 0 | 1 | 344 | 260 | RED | 5 | -4 |
| `target_selection` | FLAT | 31 | 0 | 0 | 3 | 0 | 31 | 3 | 151 | 110 | RED | 3 | +0 |
| `tp_activation_offer` | FLAT | 3 | 0 | 0 | 3 | 0 | 3 | 3 | 164 | 104 | RED | 3 | +0 |
| `tp_joint_card_live_start` | FLAT | 33 | 0 | 0 | 21 | 0 | 33 | 20 | 159 | 140 | RED | 21 | +0 |
| `tp_turn_limited_activation` | FLAT | 15 | 0 | 0 | 0 | 0 | 15 | 0 | 181 | 128 | GREEN | 0 | +0 |
| `tp_zone_change_gate` | FLAT | 8 | 0 | 0 | 13 | 0 | 8 | 13 | 141 | 126 | RED | 13 | +0 |
| `trigger_paths` | COUNTERS | 121 | 0 | 0 | 19 | 0 | 121 | 19 | 156 | 147 | RED | 19 | +0 |
| `vm_decode` | FLAT | 43 | 0 | 0 | 0 | 0 | 43 | 0 | 206 | 158 | GREEN | 0 | +0 |
| `zone_filter_parity` | FLAT | 45 | 0 | 0 | 0 | 0 | 45 | 0 | 148 | 101 | GREEN | 0 | +0 |
| `zzprobe` | DIAG | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 157 | 143 | GREEN_UNVERIFIED | 0 | +0 |

| **TOTALS (155 suites)** | | 20029 | **997** | **2** | **2466** | 44 | 16894 | 2349 | | | **114 RED / 29 GREEN / 12 unverified / 0 silent** | **2 530** | **-64** |
## 13. WHAT I DID NOT DO

* Did not modify any `.c`, `.h`, `Makefile`, `.sh`, `.ps1`, `.py` or test file — **anywhere**.
* Did not run `make run-all-tests`, did not weaken, ignore or skip any test, did not run any
  generator, did not bypass `tools/guard_data_blob.sh`'s refusal in the live tree.
* Did not perform any git write. No stash pop/apply/drop, no `checkout --`, no `restore`,
  no `reset --hard`, no commit. All git use was read-only (`rev-parse`).
* Did not read any checked-in `.log` as a result. All 142 stale `tests/*.log` were excluded
  from the copy and every number here comes from a fresh capture.
* Did not use `-fsyntax-only` for any warning claim. Every warning number in §5 comes from a
  real `make` / `gcc -c` compile.
* Did not run a suite in the live tree. Everything ran in
  `C:\Users\trios\AppData\Local\Temp\kilo\base2\engine_c_wip`.
* Did not run `board_post`. This report is the only channel.

## 14. REPRODUCING THIS

```
snapshot     C:\Users\trios\AppData\Local\Temp\kilo\base2\engine_c_wip
cards        C:\Users\trios\AppData\Local\Temp\kilo\base2\cards\build        (seeded with the canonical blobs)
manifest     C:\Users\trios\AppData\Local\Temp\kilo\base2\MANIFEST.sha256   (265 entries)
baseline     C:\Users\trios\AppData\Local\Temp\kilo\baseline\engine_c_wip    (240 entries, still matching)
raw runs     C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\runs\run{1,2}_<suite>.{out,err}
build logs   C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\{clean,all1,web,data}.{out,err}
suite builds C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\compile\<suite>.{out,err}
run indices  C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\runrun{1,2}_index.json
warnings     C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\{warnsum1.txt,waudit.out,waudittest.out}
delta        C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\{diff.txt,diff_norm.txt,diff_norm.json}
analysis     C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\{suite_table.md,suite_table.json,ranking.json,faillines.txt,attribution.json}
drivers      C:\Users\trios\AppData\Local\Temp\kilo\base2\{manifest,build_suites,run_suites,analyze,shapes2,compare,diffbase2,regress,attrib,mktable2,mktop25}.py
```
If `MANIFEST.sha256` no longer matches the tree you are looking at, this is a different
measurement.
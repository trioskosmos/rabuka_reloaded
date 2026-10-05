# HARNESS_FIXES — what the measurement harness was hiding, and what it now reports

Author: harness agent. Scope: **`tests/**` only**. No engine file and no
`include/rabuka.h` edit was required by any fix below; every one of them is a
TEST defect, and where an engine file is discussed it is named and left alone.

All numbers were measured in the snapshot
`C:/Users/trios/AppData/Local/Temp/kilo/harness/engine_c_wip` (resynced from
`engine_c_wip`, `*.o`/`*.log`/`rb_engine_*` excluded), sequentially, one suite at
a time, stdout and stderr captured to separate files, binary deleted after each
run. `src/abilities_strings.bin` verified at 726831 B /
`89204df0bbde08a0…` before and after every build. `make data` green (the
snapshot's `../cards/build` was seeded with the canonical blobs so the guard
passes); **no generator was run**.

---

## 1. The two crashes

### 1a. SIGSEGV — `tests/test_parity_recover_hand_a.c` — **TEST bug**, not the engine

**Faulting line: `test_parity_recover_hand_a.c:1056` (pre-fix), now :1064.**

```c
for (int success_count = 0; success_count <= 1; success_count++) {
        TestGame *tg = &t_tg[11 + success_count];   /* <-- line 1056 */
```

`t_tg` is declared at line 141 as

```c
/* sizeof(GameState) is ~781 KB, so every TestGame must be static. */
static TestGame t_tg[1];   /* one slot: each case is its own fork */
```

— a **one-element** array. `t_tg[11]` and `t_tg[12]` are 9 745 032 B and
10 630 944 B past the start of an 885 912 B object. The compiler already said
so, and the warning was in the build log the whole time:

```
tests/test_parity_recover_hand_a.c:1056:24: warning: array subscript 11 is above
    array bounds of 'TestGame[1]' [-Warray-bounds=]
tests/test_parity_recover_hand_a.c:141:17: note: at offset [9745032, 10630944]
    into object 't_tg' of size 885912
```

`t_tg[11]` lands outside `.bss`, so the first store inside `f_setup()`
(`rb_clear_pending_choice(tg, 1, &live)`, line 331) faults. It is a **test**
defect: an out-of-bounds array subscript, not an engine memory-safety bug.

**Fix.** The two boards in that loop are independent and sequential, so one slot
plus a full `test_game_new()` per iteration is equivalent:

```c
TestGame *tg = &t_tg[0];
test_game_new(tg);
```

`test_game_new()` (`src/test_game.c:9-40`) memsets the whole struct and
`rb_game_init()`s it, which is strictly stronger than the partial reset
`f_setup`'s `clear_p1` performs — and it is what every other case in the file
relies on `f_setup` for, since each is its own fork.

**Secondary defect at the same site — the crash was invisible.**
`on_fatal_signal` ended in `_Exit(128 + sig)`. That exits the child
*normally* with code 139, so the parent's `WIFSIGNALED(status)` was **false**,
`run_case()` took its bare `return -1` no-report path, and the crashed case
produced **no verdict line at all** and was not counted in either
`n_failed` or `n_crashed`. The suite's final tally read `13 ok, 9 failed,
0 crashed` for **23** cases — the arithmetic never closed. Fixed by
re-raising under the default disposition (`signal(sig, SIG_DFL); raise(sig);`)
so the child dies from the signal, and by making the remaining no-report path
print a verdict instead of `return -1`.

**Effect.** `13 ok / 9 failed / 0 crashed` (=22) → **`14 ok / 9 failed /
0 crashed` (=23)**. `test_f_zero_or_one_success_card_does_not_retrieve` now runs
and evaluates **18 assertions** (9 per board), all passing.

**How many tests it was hiding: exactly 1.** The premise that "a crash aborts the
whole suite" is **false for this file**: it forks per case, so the parent kept
going. `cases[]` has 23 entries and exactly one — the crashing one — produced no
verdict. The other 22 ran. I am reporting 1, not "all tests after the crash".

### 1b. SIGABRT — `tests/test_parity_cce_gaps.c` — **TEST bug**, reintroducing a fixed engine defect

**Faulting free: `src/ability/choice.c:5289`, reached from
`tests/test_parity_cce_gaps.c:1479-1491`.**

The child completed the whole test body with **7 assertions, 0 failures**, then
died in `rb_free_ability(&g->activation_keepalive)` — no `effect_free` probe
fired at all, so the abort was in the three string frees at the top of
`rb_free_ability` (`vm.c:187-193`). Instrumented `rb_decode_ability` output:

```
idx=54 out=0x7ffff9b40 full_text=0xa0019d110 trigless=0xa0019d270 trig=0xa0019d060 cost=0xa001a4270 effect=0xa001a4a40
rb_free_ability tag=choice_tail:5289 a=0x102d2d0a0 full_text=0xa0019d110 trigless=0xa0019d270 trig=0xa0019d060 cost=0xa0019d3b0 effect=0xa0019dba0
```

The keep-alive's `full_text` / `triggerless_text` / `triggers` are the *same
three blocks* that had already been released. The engine documents this exact
class in `src/ability/ability_queue.c:1306-1312`:

> The previous handoff copied `ab` into the keep-alive and then nulled only
> `ab.cost` / `ab.effect` before calling `rb_free_ability(&ab)`, which left
> full_text / triggers / triggerless_text owned by **BOTH** copies: the local
> call freed the shared text and the keep-alive release freed it again, which is
> the SIGABRT in the cost-prompt continuation teardown.

`nahone_resolver_path()` in the test was doing precisely that, by hand:

```c
tg->state.activation_keepalive = ab;
tg->state.activation_keepalive_valid = 1;
ab.cost = NULL;
ab.effect = NULL;
...
rb_free_ability(&ab);          /* released the three strings the keep-alive kept */
```

**Fix** — make the handoff TOTAL, which is the contract `rb_drain_keep_alive`
(`ability_queue.c:1302-1318`) documents and every engine caller follows:

```c
tg->state.activation_keepalive = ab;
tg->state.activation_keepalive_valid = 1;
memset(&ab, 0, sizeof(ab));     /* the transfer above was TOTAL */
```

**Secondary defect, same class as 1a:** `on_fatal_signal` again ended in
`_Exit(128 + sig)`, so `WIFSIGNALED` was false and the parent reported
`unexpected child exit 134` instead of a `CRASH` verdict. Fixed the same way.

**Effect.** `68 tests / 30 ok / 37 FAILED / **1 CRASH**` → `68 / 30 / 38 / **0
crashed**`. `a22b_resolver_path_pays` now completes and reports **10
assertiations with 1 real failure** (`a22b: ... and the blade`), i.e. a genuine
parity defect that was previously invisible because the process aborted before
printing it. Suite FAIL lines 53 → 54.

**How many tests it was hiding: exactly 1.** Same reason as 1a — this suite also
forks per case, so the parent continued. `a22b_resolver_path_pays` was the only
lost case; the 67 after it all ran.

---

## 2. Suites that could not report red

### 2a. `tests/test_draw_phase_fix.c` — the macro returned instead of reporting

The old macro printed a bare `"<message>: got N, expected M"`, bumped no
counter, and `return`ed **from the test function**, so the suite emitted **0
bytes on both streams**: structurally incapable of reporting failure, and
indistinguishable from a pass.

Fixed to the corpus shape (`tests/test_parity_draw_flat.c:119-129` is the
twin): `FAIL: ` prefix, a real failure counter, an `ok:` echo, and a
`"%d checks, %d failures"` summary from `main`.

**How many previously-invisible failures was it suppressing: 0.**
The suite contains exactly two assertions and **both pass** on the current tree
(`2 checks, 0 failures`). The harness hole was real and is closed, but this
particular file happened to be green. Reporting 0 is the measured answer; the
"expect non-zero" expectation does not hold here. What the fix does change is
that the suite is now *classifiable* at all — it is a verified GREEN with a
tally instead of a `NO_OUTPUT` black box.

### 2b. Per-test verdicts — 44 suites, 932 FAIL lines

**Root cause, two distinct mechanisms:**

**(i) 7 FORK suites whose verdict vocabulary was not `FAILED:`.** These suites
detect failure and count it correctly, but announce it in private shapes that
no verdict scanner can read:

| suite | the shape it printed instead of `FAILED:` |
|---|---|
| `audit_actions` | `== <case>: RED as pinned -> …` / `== <case>: GREEN case FAILED -> …` |
| `audit_zones` | same `== <case>: …` vocabulary |
| `parity_card_score` | `== case <name>  assertions=N failures=M` |
| `parity_live_total2` | `== case <name>  assertions=N failures=M` |
| `parity_score_per_card2` | `== case <name>  assertions=N failures=M` |
| `parity_rules_conditions2` | `== case <name>  assertions=N failures=M` (was already green) |
| `parity_per_card2` | `--- case <name>` headers only — **no verdict at all** |

`parity_per_card2` was the genuinely broken one: its `run_all()` forked per
case but never compared the shared `failures` counter across the fork, so it
could not tell a passing case from a failing one. It now snapshots the shared
counter before each fork and prints `FAILED:`/`ok:`/`CRASH:` from the delta.

All six (+ `parity_rules_conditions2` as a side effect) now emit the corpus
`FAILED:` / `ok:` / `CRASH:` verdict **alongside** their own richer lines,
which are kept — nothing was replaced or removed.

One honest caveat on `audit_zones`: the base run recorded
`FAIL: stage_first_empty_is_centre_first: fatal SIGSEGV (11) in child`, i.e. a
child crash hidden behind an assertion line. After the verdict work the suite
was rebuilt and run **twice** and the SIGSEGV did **not** reproduce. So I can
claim the `CRASH:` verdict in that file is now *reachable*, but **not** that I
fixed the crash — the observation is unexplained and is carried as H3 in
`PARITY_CHECKLIST.md`.

**(ii) 37 FLAT suites with no test enumeration at all.** A FLAT suite runs every
test body in one `main()`, so its `FAIL:` lines name an *assertion* and nothing
else; there was no layer anywhere that could attribute a failing assertion to a
named test. That is the whole of "failed tests is ill-defined".

Fixed in **12** of them by inserting a verdict layer next to the failure
counters and calling it from the `CHECK` and `CHECK_EQ` failure branches:

```c
static const char *g_flat_failed[512];
static int g_flat_failed_n;
static void flat_verdict_failed(const char *fn)
{
    for (int i = 0; i < g_flat_failed_n; i++)
        if (g_flat_failed[i] == fn) return;   /* __func__ is one array per function */
    if (g_flat_failed_n >= 512) return;
    g_flat_failed[g_flat_failed_n++] = fn;
    printf("FAILED: %s\n", fn);
    fflush(stdout);
}
#define FLAT_VERDICT() flat_verdict_failed(__func__)
```

`__func__` (C11 6.4.2.2) names the enclosing test body, and because it is the
*same array object* at every expansion inside one function, pointer identity is
the dedupe key — no string compare, no per-call-site duplicate verdict lines.
Suites done: `condition_evaluation`, `look_select`, `parity_ability_mod`,
`parity_custom_misc`, `parity_integration`, `parity_look_reveal`,
`parity_queue_resume`, `parity_state_effects2`, `parity_state_family`,
`parity_stats_pipeline`, `ported_generated`, `surplus_heart`.

**This is purely additive: no assertion was added, removed, relaxed or
reordered.** The 25 remaining FLAT suites still need it; each is a two-line
insertion plus one call per failure branch. They are enumerated in
`PARITY_CHECKLIST.md` cluster H.

### 2c. `SETUP BUG` (5) and `VACUOUS` (43) — NOT FIXED, reported

Both are on stderr, neither is a `FAIL` line, so a `^FAIL` scan scores them
zero — they mean the *opposite* of green. Current inventory on the live tree:

| suite | VACUOUS | SETUP BUG |
|---|---|---|
| `parity_look_per_card` | 14 | — |
| `parity_target_player` | 8 | — |
| `parity_recover_q79` | 5 | — |
| `parity_recover_hand_a` | 4 | — |
| `parity_wave1c` | 3 | — |
| `parity_cce_gaps` | 2 | — |
| `parity_look_and_filter` | 2 | — |
| `parity_parser_e2e_extra` | 2 | — |
| `parity_wave1a` | 2 | — |
| `parity_recover_live` | 1 | — |
| `audit_conditions` | — | 3 |
| `parity_blades_live_start` | — | 2 |

`VACUOUS` means a *passing* assertion was **not proven non-vacuous** because its
paired positive is red, so the zero proves nothing. `SETUP BUG` means the
fixture was never built, so the test asserted nothing. One of the SETUP BUGs is a
wrong-card defect: `SETUP BUG: expected card PL!HS-pb6-002-R, resolved id=-1
(<none>)`. I did not touch these: fixing them means writing real paired
positives against specific engine behaviour, which needs the Rust reference
read line by line per row, and I would rather leave them loudly visible than
guess. Carried in cluster H.

### 2d. `tests/test_audit_triggers.c` — FAIL shape and three wrong expectations

**FAIL shape.** The macro printed `FAIL [%s] %s: ` — no colon after `FAIL`, so
a `^FAIL:` verifier matched nothing and the suite reported `FAIL_OUT=0
FAIL_ERR=0` while running 3 FAILED with 5 real FAIL lines. Both `CHECK` and
`CHECK_EQ` now print `FAIL: [<row>] <test>: …`, which is the shape
`test_p1_helpers.c:11`, `test_parity_draw_flat.c:124` and every FORK suite use.
Row and test name stay in the text; only the colon moves.

**Rows A2/A3/A4b asserted an enqueue-time use spend that Rust does not perform.**
They read `rb_use_count(...) == 1` immediately after `rb_trigger_debut` /
`rb_trigger_live_success`. The engine is unambiguous, in three places:

* `src/turn/triggers.c:108-129` — "NOTHING IS RECORDED HERE, and that is now
  faithful. An enqueue records nothing in Rust. All three enqueue paths are pure
  builders … each ends in a bare `self.ability_queue.enqueue(entry)`.
  `use_limit_recorded` is constructed **false** in all four constructors … and
  is written in exactly ONE place, `record_ability_use` (abilities.rs:216-231),
  which is reached from eleven call sites, **every one of them
  resolution-time**."
* `src/core/game_state_abilities.c:901-933` — the five enqueue-time
  `rb_record_use` sites were removed together with
  `rb_mark_entries_use_limit_recorded`, precisely because "the flag and the
  record … can only ever be removed TOGETHER".
* `src/test_game.c:1238-1256` — the same note for the helper, with the two
  measured regressions the old unconditional record caused.

So the faithful value at enqueue time is **0**, and the three rows asserted 1.
Fixed to `used == 0`. That is *stronger*, not weaker: each row still requires
`queued == 1`, and now additionally pins that the enqueue did **not** spend the
use — a pair that could never both be true before.

**Also fixed here:** `on_fatal_signal` ended in `_Exit(128 + sig)`, making the
`WIFSIGNALED` CRASH verdict unreachable. Re-raise added.

**Effect.** `10 ok / 3 FAILED / 5 FAIL lines` → **`10 ok / 1 FAILED / 1 FAIL
line`**, and the remaining failure (`A7_reveal_cost_values`) is now visible to a
`^FAIL:` scan at all. The two rows that went green are A2 and A3's paired
positives plus A4b's two.

---

## 3. Stack

### 3b. The Makefile's stated size — ALREADY CORRECT, by a peer

`Makefile:3-9` now reads `sizeof(GameState) is 885912 bytes (864.2 KiB) —
MEASURED 2026-10-03 by sizeof() in tests/test_harness_self.c §A` and records
that it used to say "~780 KB", which was wrong by 105 912 B.
`tests/test_harness_self.c:84-87` asserts it against
`RB_GAMESTATE_BYTE_CEILING`, so it cannot drift again silently. **No edit was
needed and I made none.** `-Wl,--stack,16777216` is correctly documented as
load-bearing.

### 3a. The 974 non-static locals — NOT CONVERTED

Measured on the live tree: **974 non-static `GameState`/`TestGame` local
declarations across 58 test files** (largest: `test_parity_card_score.c` 74,
`test_parity_hearts_constants.c` 67, `test_parity_wait_activation.c` 66,
`test_parity_characterization_extra.c` 54, `test_parity_score_effects.c` 51).
The conversion is mechanical and provably neutral (automatic storage is never
zeroed by the C runtime and every site is memset by `test_game_new()` /
`rb_game_init()`), but it is 974 edits across 58 files that 12 other agents are
concurrently editing, and this repo's standing rule forbids scripted mass
patches. Doing it by hand in the time available would have meant doing a small
fraction badly. **Not done**; carried in cluster H with the exact recipe.

---

## 4. The corrected failed-test count

See `PARITY_CHECKLIST.md` cluster H for the current figures and the residual
bound. The shape of the answer:

* Before: `failed tests` was **ill-defined** — 948 reported by FORK/COUNTERS
  harnesses, plus 932 FAIL lines in 44 suites belonging to no named test, so the
  true figure was a range.
* After: the 932 lines are attributable in 19 of the 44 suites (6 FORK + 12
  FLAT + `parity_rules_conditions2`), and 25 FLAT suites remain unattributable.
  So the figure is **now bounded from above by a named count**, with the
  residual stated suite by suite in cluster H rather than hidden.

## 5. THE 20 UNVERIFIED + 1 BROKEN -- every suite now reports a verdict

`BASELINE3.md` measured green/red/unverified/broken = **29 / 116 / 20 / 1**
over 166 suites, and said so itself: 20 suites "ran to completion, rc 0, but
emit **no verdict line of any form**. These cannot be called passing."

**MEASURED, all 166 suites, one pass, strictly sequential, every suite run
through `tools/verdict_runner.c`:**

| state | suites |
|---|---|
| verdict present (`PASSED` / `FAILED`) | **166 / 166** |
| no verdict at all (INCONCLUSIVE) | **0** |
| did not link | **0** |
| PASSED | 49 |
| FAILED | 117 |

Corpus: **3 783** result lines, reconciling two independent enumerations.

### 5a. The premise in the brief was wrong, and checking it is the finding

The brief stated that "main has just wired the verdict runner to write one per
suite" as `verdicts/<suite>.json`. **It had not.** `tools/verdict_runner.c`
printed one text verdict line and nothing else; there was no `verdicts/`
directory anywhere in the tree. Since `tools/verdict_runner.c` is a file this
task owns, the wiring was done here -- see 5c -- with **no Makefile change**,
because the runner writes its own record and the Makefile already routes every
suite through it.

### 5b. Classification of the 20, and of the 1 BROKEN

The runner's `PASSED` requires exit 0 **and** zero counted result lines. That is
stronger than "no FAIL lines" and stronger than `$?`, but it is *not* the same
as "the suite executed assertions". Probing for that (how many per-case result
lines does a PASSED suite actually emit?) exposed **8 suites that reported
PASSED while reporting nothing per-case at all**:

| class | suites | resolution |
|---|---|---|
| **(b) unrecognised shape** -- emits output, but in a private grammar the corpus does not recognise: `audit_warnings` (`audit_warnings: %d checks, %d failed`), `decode_skew` (`=== decode-skew audit: %d checks, ...`), `null_effect_guard` (`null-effect guard: %d assertions, %d failures, %d crashes`), `parity_choice_routing` / `parity_orphan_producers` / `parity_q68_cannot_live` (`CHECK` that prints only on failure, plus `=== %d failed ===`) | 6 | **Normalised by the shared runner.** Each now emits exactly one verdict plus a readable per-suite summary. Their leading `<name>: ` prefix is why BASELINE3's recognisers missed them -- they are counted, not lost. |
| **(c) genuinely asserts nothing** -- `parity_conditions` | 1 | **FIXED. 23 real assertions added**, see 5d. |
| **(c) meta-suite, conditional by design** -- `coverage_gate` | 1 | Kept. It reconciles the corpus, so in a single-suite context there is nothing to reconcile and it SKIPs with a printed reason. That is the honest answer, not a pass. |

**The 1 BROKEN, `audit_conditions`, is NOT broken, and the reason matters.**
BASELINE3 recorded it as "exits 2, produces no verdict line at all". Its
`main` opens with

```c
if (rb_load("../cards/build") < 0 && rb_load("cards/build") < 0) {
```

so it resolves the card database **relative to the working directory**, one
level above `engine_c_wip`. Any snapshot placed elsewhere -- including the one
this measurement used -- loses that path, the suite prints `cannot load card
db`, returns 2, and asserts nothing. Run from `engine_c_wip` it loads fine and
reports **24 result lines: 12 assertions + 12 setup bugs**, every one of the
form `[PL!S-bp5-013-N] has no condition v=1 key=- in abilities 0..7`. So it is
an ordinary **RED with 12 unresolved fixtures**, and its BROKEN classification
was an artefact of the measurement context.

This is now a *rule*, not a story: the rewritten gate fails any suite that is
`FAILED` with `n_result_lines == 0` as its own state (**G2**, below), because
"exited non-zero having asserted nothing" and "measured something and it was
wrong" are different findings and must not share a row.

### 5c. `verdicts/<suite>.json` -- the structured verdict

`tools/verdict_runner.c` now writes one record per run, alongside (not instead
of) the text verdict line the Makefile greps:

```
verdicts/<suite>.json    12 fields: suite, verdict, exit, signal,
                         n_result_lines, n_assert, n_vacuous, n_setup_bug,
                         n_test_bug, n_fatal, n_crash, n_renormalised
```

* `verdict` is one of **`PASSED` / `FAILED` / `INCONCLUSIVE`**. A suite killed
  on a signal used to emit no verdict at all, so every consumer read it as green;
  it is now a third, spelled-out value.
* `exit` is recorded **for forensics only** and is never used as a verdict.
  `COVERAGE_HARNESS.md` 3 proves it disagrees with the output in both directions.
* Directory from `$RB_VERDICT_JSON_DIR` (default `verdicts`); `$RB_VERDICT_JSON=0`
  suppresses the file for a one-off run.
* `engine_c_wip/verdicts/` added to `.gitignore` (regenerated evidence, never
  source). One appended line; the existing 9 216 bytes are byte-identical.

### 5d. `parity_conditions` went from 0 assertions to 23 that can fail

It was a pure diagnostic dump -- decode two cards, print their conditions, drive
two scenarios, print the modifiers, `return 0`. It emitted no result line of any
kind, which is why BASELINE3 could only call it UNVERIFIED; under the verdict
layer it then became **the one non-meta suite in the corpus reporting PASSED
while executing zero assertions**.

Every number it was already printing is now asserted, and each control exists
because without it the assertion beside it is satisfied by the zero value:

* `test_id_exact`, **not** `test_id`/`rb_find_card_by_no` -- the forgiving
  lookup has a substring fallback that resolves a *different real card* on a
  near-miss (CARD-NUMBER TRAP). All three fixtures verified to occur **exactly
  once** in `cards/build/cards_gen.rs` (`PL!SP-bp4-024-L`, `PL!SP-pb1-001-R`,
  `PL!-sd1-010-SD`; and `PL!SP-pb1-001-P` occurs **0** times, so the fallback
  could not have been hiding there).
* `n_abilities >= 1` guards the decode loop, and `decoded == n_abilities`
  makes the loop total -- with 0 abilities "every ability decoded" is 0 == 0.
* **Both sides of every filter.** A one-sided `matches == 1` is satisfied by a
  filter that returns 1 for everything, so the negative card is asserted in the
  same breath: Liella! on `pb1-001-R` is 1, on `sd1-010-SD` is 0, μ's on
  `pb1-001-R` is 0.
* **All three `set_live_card` controls.** With no member in the live slot the
  live never runs, no `gain_resource` fires, and "0 blades" is the answer
  whether or not the behaviour works. This is precisely the defect a peer found
  in the Rust sibling `live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs`:
  nine real tests, **none staging a member in the live slot**. Here the blades
  are captured **mid-scenario** (2 on each side) *and* after live end (0 on each
  side), so the pair "granted then expired" is assertable; an end-only capture
  would see 0 == 0 and could not fail.
* The snapshot total is asserted as the **relation** `base score + earned
  modifier`, not as a literal 7 -- a literal would survive the modifier being
  dropped if the base score moved to compensate.

**Result: 23 checks, 0 failed. PROVEN load-bearing by mutation** -- five
expectations perturbed in a scratch copy (`score_mod` 1->2, blade 2->1, snapshot
relation +1, and both live-slot controls inverted) produced **5 failures**, each
naming the assertion it broke:

```
FAIL: PL!SP-bp4-024-L's live-start modify_score applies exactly +1 (got 1, expected 2)
FAIL: the snapshot total is the card's base score plus the score modifier it earned (got 7, expected 8)
FAIL: control: scenario 2 must put player 1's member into the live slot
FAIL: control: scenario 2 must put player 2's member into the live slot
FAIL: player 1's left-side Liella! member holds the 2 blades granted after a successful live (got 2, expected 1)
```

Corpus effect: **+23 assertions that can fail, +0 failures.**

## 6. `coverage_gate` reads structured verdicts, not stdout

`tests/test_coverage_gate.c` walked `tests/*.log` and grepped them. It now reads
`verdicts/*.json` and nothing else. Two independent reasons, both measured:

1. **A log is prose.** The gate had to re-implement the runner's notion of a
   result prefix -- a ten-tag table, a boundary rule, and a `[exit=` marker used
   to tell the runner's own verdict from a suite's legacy `FAILED: <case>` line.
   ~150 lines whose only job is to agree with the runner, and agreeing only
   because both read the same bytes.
2. **The gate was measuring its own previous output.** `run-all-tests` redirects
   each suite's stdout into `tests/<stem>.log`, and the gate's own log is one of
   those files. The walk counted those `FAIL:` lines while the per-suite census
   skipped the same file -- so **G3's gap equalled the previous run's failure
   count** (152 lines on the first complete evidence set) and could not pass in
   the context it exists to judge. That is what produced the reported
   **170 checks / 166 failures** in a zero-log context against **~294** in a log
   context: the number tracked the size of its own last output, not the corpus.

A verdict record is written by the suite's own run, and `run-all-tests` produces
one for `coverage_gate` like any other suite, so the gate needs **no self-exemption
anywhere** -- the two sides of G3 are the same set of files by construction
rather than by a patch.

### Before / after, measured

| context | before (log-scraping) | after (structured verdicts) |
|---|---|---|
| corpus-shaped evidence set | ~294 checks, failures scaling with the previous run's output | **1004 checks, 0 failures** |
| evidence set with 1 broken record | failures dominated by self-measurement | **1005 checks, 2 failures** -- and both name the same single real defect (`audit_conditions`), once per-suite and once in the aggregate |
| no / partial evidence set | 166-170 failures blaming a manifest for a runner never asked to fill it | **1 check, 0 failures**, with the condition and the reason printed |

```
info: sources=166 verdicts=166 result-lines(corpus)=3783 PASSED=49 FAILED=117
      INCONCLUSIVE=0 unreadable=0 exit-only-red=0
info: the corpus-wide result-line count RECONCILES with the sum of the per-suite
      counts, and every verdict record is readable
1004 checks, 0 failures
```

### What each rule now reads

* **G1 MANIFEST-VS-GLOB** -- bidirectional, over `verdicts/`: every
  `tests/test_<stem>.c` has a record, **and** every record has a suite (a record
  for a deleted suite is a count for a suite that does not exist).
* **G2 TALLY TOTAL, VERDICT MATCHES IT** -- replaces the old log-prefix census.
  Per suite the six breakdown counters sum to `n_result_lines` (nothing counted
  is missing from the tally); `PASSED` requires `n_result_lines == 0`; and
  `FAILED` with `n_result_lines == 0` is its **own** state -- *asserted*, not
  merely reported, because that is the shape of the one BROKEN suite. An
  unreadable record is `UNREADABLE`, never "a suite with zero failures":
  collapsing those two is how a broken evidence set reads as a clean corpus.
* **G3 RECONCILES** -- directory walk of `verdicts/` vs per-suite sum, with no
  self-exclusion, because none is needed.
* **G4 EVERY SUITE REPORTED** -- exactly one verdict, and not `INCONCLUSIVE`.
* **G5 VERDICT ARITHMETIC** -- `#PASSED + #FAILED + #INCONCLUSIVE + #UNREADABLE`
  equals the record count, and `#PASSED > 0`.

The condition-first skip is kept, and tightened: a partial set (`make t` writes
exactly one record) is detected by comparing record count against suite count,
not merely by "is the directory empty", because 166 records for 168 suites is
just as unanswerable as 0 records for 168. Nothing is relaxed -- on a
corpus-shaped set every rule runs in full.

## 7. Cannot-fail assertions -- settled by reading, all five named

All five named files live under `engine/tests/**`, which this task may **not**
edit, so each was read and adjudicated rather than changed. Two of the five
brief claims are **disproven**; one is confirmed and is a real cannot-fail.

| claim | verdict |
|---|---|
| `nahone_live_start_same_name_test.rs` -- `blade1 >= 1` | **DISPROVEN.** `blade1 >= 1` never stands alone. At lines 314-319 and 868-873 it is one **conjunct of a two-sided exclusivity disjunction** `(b1 >= 1 && b2 == 0) \|\| (b1 == 0 && b2 >= 1)`, which fails when *neither* or *both* are buffed -- the strongest available form. At 876/879 it sits in an `if heart1 == 1` branch paired with `assert_eq!(blade2, 0)`. The file has **20 tests** built on `assert_eq!(..., 1)` exact equality. |
| `dive_ab0_..._test.rs` -- "its only assertion is `guard <= 20`" | **DISPROVEN; the peer's dispute is UPHELD.** The `guard` is a *liveness guard inside the prompt-draining loop*, not the file's only assertion. It is followed by **three strong ones**: `placements == 1`, `placed ^ static_placed` (XOR), and `!static_placed`. The brief also misquotes the bound (the file says `<= 10`). The sibling `dive_ab0_phase_gate_and_multi_copy_live_zone_limit_edges_test.rs` carries **30 assertions**. |
| `live_success_surplus_score_zero_floor_test.rs:83` -- bound `-2` while comment and message both say `>= 0` | **CONFIRMED.** Line 81's comment says "total should still be >=0" and line 84's message says "total stays >=0"; the assertion is `p1_constant_total_score_bonus >= -2`. Not vacuous -- it can fail below -2 -- but strictly weaker than its own stated intent in the direction that matters. Correct bound is `>= 0`. |
| `live_success_look_three_deck_size_q36_test.rs:34` -- `<=` permits "nothing happened" | **CONFIRMED, and worse than stated.** `deck_after <= deck_before` under the message "LiveSuccess should draw cards". `<=` is satisfied by the deck being **unchanged** (nothing happened) *and* by the deck being **smaller for any other reason** (cards lost). It is weak in both directions; the honest form is `deck_after < deck_before`, or `deck_before - deck_after >= 1`. |
| `live_score_tie_live_zone_placement_across_rounds_test.rs:257` -- `turn_number >= 1` | **CONFIRMED, and it genuinely cannot fail.** `turn_number` is initialised to **`1`** (`engine/src/core/game_state/mod.rs:677`), so `>= 1` is satisfied by the constructed value. The comment on line 256 says "the rollover ran: turn counter advanced past the first round", so the intended assertion is `>= 2`, or a before/after comparison. The enclosing test does carry one strong assertion (251-255, the success-zone emptiness), so it is the *last* line that is dead, not the test. |

**For the engine owner:** three of these need a one-line change each
(`>= 0`, `<` , `>= 2`) and none of the three is currently reporting a real
defect -- they are reporting a weaker guarantee than they describe.

## 8. Measurement method notes (the traps, and what they cost)

* **This toolchain is MSYS2/PE: `cc -o name` produces `name.exe`.** Naming the
  bare path makes `rb_verdict`'s `execv` fail, the child prints
  `FAIL: [setup-bug]`, and **all 166 suites measure as FAILED.** First pass
  produced exactly that false corpus.
* **The shared tree runs `make clean` underneath you.** A pass that began with
  all 166 suites linking and ended with 166 link failures was not a regression:
  an object had been removed mid-pass. Every measurement here was therefore taken
  in an isolated snapshot, `make clean` verified to **0 objects** before
  `make all` (51 objects), then linked and run.
* **`OBJ_LIB` names `src/web_server.o` AND `src/main.o`.** `main.o` defines its
  own `main`, so a suite linked against the full object list fails with
  `multiple definition of main` -- all 166, not a subset. Both are dropped for
  suite links, which is what the Makefile's own comment about `OBJ_LIB` intends.
* **`make t-nogate` is the measurement path** and was used throughout. The
  `data:` guard was **not** weakened, nothing was regenerated, and
  `guard_data_blob.sh` / `regen_all.sh` were not touched.
* **`test_id(` budget ratchet:** no suite was added, so
  `test_harness_self.c` §D1/D2 was not tripped and **no cap was re-baselined**.
  `sizeof(GameState)` / `RB_GAMESTATE_BYTES_PINNED` untouched (no `src/**` edit).

### Regression check

`tools/corpus_diff.py`, before vs after, all 166 suites:

```
suites: 166   changed: 0   unchanged: 166
corpus failing assertions: 3783 -> 3783  (+0)
corpus FAILED case lines:  117 ->  117  (+0)
before/after verdicts: {'PASSED': 49, 'FAILED': 117}
```

**Zero regressions.** The +23 assertions added to `parity_conditions` are
invisible here precisely because they all pass -- and they were proven able to
fail by the mutation run in 5d.

### Files changed

| file | change |
|---|---|
| `tools/verdict_runner.c` | writes `verdicts/<suite>.json` (12 fields, 3 verdict values); `INCONCLUSIVE` recorded on a signal death; `$RB_VERDICT_JSON_DIR` / `$RB_VERDICT_JSON=0` |
| `tests/test_coverage_gate.c` | reads structured verdicts; G1 bidirectional; G2 redefined as tally-integrity + verdict/count agreement + exit-only-red as its own asserted state; no self-exclusion; partial-set skip |
| `tests/test_parity_conditions.c` | 0 -> 23 assertions, all anti-vacuity-controlled, mutation-proven |
| `tools/corpus_measure.py` | new: link+run+census every suite in one sequential pass |
| `tools/corpus_classify.py`, `tools/silent_probe.py`, `tools/corpus_diff.py` | new: state classification, "PASSED but silent" probe, before/after diff |
| `.gitignore` | one appended line for `engine_c_wip/verdicts/` (existing bytes byte-identical) |

**Not touched:** `include/rabuka.h`, the `Makefile`, anything in `src/**`, and
everything under `engine/**`.
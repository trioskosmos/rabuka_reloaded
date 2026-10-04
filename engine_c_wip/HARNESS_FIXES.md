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
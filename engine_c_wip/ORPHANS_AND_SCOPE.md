# ORPHANS_AND_SCOPE — cluster T8

**Owner:** cluster T8 (orphans, dead build graph, scope decisions).
**Date:** 2026-10-03.
**Status:** COMPLETE for Q1-Q4. Q5 lists what was found and deliberately NOT fixed.
**This file was flushed incrementally** — a predecessor session of
this cluster died twice on infrastructure errors and left `tests/test_harness_self.c`,
`tests/test_choice_renderer.c`, `tests/test_describe_golden.c`, `tests/fixtures/describe_golden.txt`
and the `Makefile` / `src/test_game.c` / `include/test_game.h` hunks on disk but no
decision document. Those partials were READ and continued, not restarted.

All measurements below were taken in an isolated snapshot at
`C:/Users/trios/AppData/Local/Temp/kilo/orph3/engine_c_wip`, built from scratch
(`robocopy` excluding every `*.o`, every `rb_engine*`, every `tests/*.log`; then
`make -j1 all`, exit 0). `src/abilities_strings.bin` was verified at 726831 B /
`89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344` before the build
and after every run; `make data` reported `already installed` both times, i.e. no
generator ran. No git write of any kind was performed.

---

## Q1 — `src/ability/choice_renderer.c`: dead code, or a broken build graph?

### DECISION: **RESOLVED — it is built. Nothing to do. Do not re-open.**

`Makefile:25` lists `src/ability/choice_renderer.c` in `SRC` and `Makefile:28`
lists `src/ability/choice_renderer.o` in `OBJ_LIB`. That is the whole fix. Proven
by building, not by reading the Makefile:

```
$ grep -n choice_renderer Makefile
25:... src/ability/describe.c src/ability/choice_renderer.c src/game/deck_parser.c ...
28:... src/ability/describe.o src/ability/choice_renderer.o src/game/deck_parser.o ...

$ make -j1 all                       # snapshot, exit 0
cc ... -c -o src/ability/choice_renderer.o src/ability/choice_renderer.c
cc ... -o rb_engine ... src/ability/choice_renderer.o ...
```

So **1964 of 1964 lines are compiled and linked**, and it is exercised:

```
choice_renderer     exit=0  out_lines=75  err_lines=0  FAIL=0  longest=139
```

(`make t T=choice_renderer`, streams captured separately via
`Start-Process -RedirectStandardOutput/-RedirectStandardError`, longest line 139 B
— far above the 119-char PowerShell truncation threshold, so nothing was cut.)

### WHICH CHANGE RESOLVED IT

The T5 finding ("`Makefile:20-70` and `platforms/*/build_*.sh` did not reference
it") was accurate **for the revision T5 measured**. It was resolved by an
**uncommitted working-tree edit to `engine_c_wip/Makefile`**, not by a commit.
Evidence: `git status --short` reports `M engine_c_wip/Makefile` (modified,
uncommitted), and the added explanatory block is `Makefile:13-24`:

```
# src/ability/choice_renderer.c (1964 lines, the port of engine/src/
# choice_renderer.rs) was in NO build target until 2026-10-03: not here, not in
# platforms/*/build_*.sh, not in platforms/Makefile. 1821 of its lines were
# compiled into nothing ...
# Exercised by tests/test_choice_renderer.c.
```

`git diff --stat engine_c_wip/Makefile` = **72 lines changed**. `HEAD = f152a50f`
still does not list the file. **This fix is uncommitted and therefore invisible to
anyone who checks out the tree** — it is the most fragile resolution in this whole
report and it is called out in the final answer as a live risk.

The decision recorded by that edit is deliberate and worth restating so it is not
undone: the file stays in `SRC`/`OBJ_LIB` **even though nothing in the C tree
consumes it**. `platforms/*/build_*.sh` still does *not* list it, and that is
correct — those targets ship their own UI in `platforms/*/src`. A grep for
`rb_rend_` across `src/`, `tests/` and `tools/` returns exactly two files:
`src/ability/choice_renderer.c` (372 hits) and `tests/test_choice_renderer.c`
(127 hits). The test is the consumer. That is a legitimate consumer for a port
whose production consumer is a UI backend this tree does not have.

**Consequence for `include/rabuka.h`:** nothing to report. Because the file is
*kept*, no `BLOCKED-ON` arises. `include/rabuka.h:3263-3289` already declares the
public surface (`rb_rend_render_card_choice_grid` and friends) and those
declarations are now backed by definitions that actually link.

---

## Q1b — `src/core/alloc_counter.c`: the template. **Fix it, do not quarantine it.**

The brief supplied the template as "`alloc_counter.c` is NOT in `SRC` or `OBJ_LIB`
and it FAILS TO COMPILE under every `-std` (`ATOMIC_VAR_INIT` was removed in C17)".
**Half of that premise is measurably wrong, and the half that is wrong is the half
that would have justified a quarantine.** Measured with the toolchain this tree
builds with (gcc 15.3.0, MSYS2):

| `-std=` | result |
|---|---|
| `c99` | exit 0, no diagnostics |
| `c11` (the Makefile's setting, `Makefile:2`) | exit 0, no diagnostics |
| `c17` | exit 0, no diagnostics |
| `gnu11` | exit 0, no diagnostics |
| `gnu17` | exit 0, no diagnostics |
| `c2x` (C23) | **exit 1, 18 errors** |

Command, once per row:
`gcc -std=<std> -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -c -o $TMP/ac.o src/core/alloc_counter.c`

**`ATOMIC_VAR_INIT` was removed in C23, not C17.** C17 *deprecated* it; glibc's
`<stdatomic.h>` keeps defining it for every mode up to and including `c17`, and
GCC 15.3.0 accepts it there. It genuinely disappears in `c2x`, where the file then
fails with `implicit declaration of function 'ATOMIC_VAR_INIT'` at
`src/core/alloc_counter.c:97` plus 17 `initializer element is not constant`
errors at lines 98-101, 105-108, 111-112, 118, 487, 758-759.

**Therefore the premise "it fails to compile, so quarantine it" is unfounded for
the configuration this tree actually builds in.** It is not a landmine; it is an
ordinarily-compilable file that happens to be one `-std=c2x` bump away from
stopping. Quarantining it would have destroyed 775 lines of real work on the
strength of a claim that does not reproduce.

**The real defect is the build-graph one, and it is the same defect T5 found in
`choice_renderer.c`: the file is simply not in `SRC`/`OBJ_LIB`.** It has a real
Rust twin — `engine/src/alloc_counter.rs`, 18168 B / ~600 lines, the C being
775 lines / 33425 B. Under the project's own rule ("port `engine/src/...` into
`engine_c_wip/src/...` file-by-file, faithfully", `PROGRESS.md:5`), an unbuilt
faithful port is an unfinished port, not dead code.

**Recommendation, not applied (see "found without fixing" below): `alloc_counter.c`
should be FIXED and WIRED, not quarantined** — add `src/core/alloc_counter.c` to
`SRC` and `src/core/alloc_counter.o` to `OBJ_LIB`, add its `rb_allocctr_*`
declarations to `include/rabuka.h` (currently absent — verified: zero matches for
`allocctr|alloc_counter` in `include/rabuka.h`), and replace the 18 `ATOMIC_VAR_INIT`
initialisers with plain `= 0` / `= false` so the file also survives C23. I did not
make this change: `src/core/**` and `include/rabuka.h` are explicitly outside the
files this agent owns, and adding 775 lines of allocator instrumentation to every
link target in a tree whose parity baseline is 2530 FAIL assertions is a change
that needs an owner's sign-off, not a drive-by.

**This is still strong evidence for the Q1 answer**, in the opposite direction to
the one the brief anticipated: the `alloc_counter.c` template does *not* show that
an unbuilt file here is a broken file. It shows that an unbuilt file here is
usually just an unwired file. The Q1 answer is therefore "resolved, and the fix is
correct", not "quarantine".

---

## Q2 — `engine/src/game/display.rs`: **OUT. Deliberate scope decision, now
recorded in `PROGRESS.md` so it stops being re-litigated.**

Size, exact (`wc -l`): Rust `engine/src/game/display.rs` = **2014 lines** /
86326 B. Its only in-scope sibling consumer, Rust
`engine/src/game/web_server.rs`, is 3861 lines / 138678 B.

### The five measurements that decide it

**1. It is a READ-ONLY PROJECTION. It cannot affect engine behaviour.**
`game_state_to_display(game_state: &GameState)` and every other entry point take
`&GameState`. A grep for `&mut GameState`, `&mut game_state` and `mut gs` across
all 2014 lines returns **nothing**. The one `&mut` in the file is
`filter_display_for_player(&mut GameStateDisplay, ...)` — the *display*, not the
state. Consequence: no parity test can be made to disagree with `display.rs`,
because `display.rs` cannot make the engine do anything. It is not on the parity
critical path and cannot become one.

**2. Its consumers are three, and none of them is the engine.**
A grep for `display::` / `to_display` across all of `engine/src` and
`engine/tests` returns exactly:

| consumer | what it does with it |
|---|---|
| `game/web_server.rs` (6× `game_state_to_display`) | serialises it as the HTTP JSON payload |
| `bin/harness.rs:86` | `serde_json::to_string_pretty(&display)` — a debug dump |
| `main.rs:27-28, :87` | a re-export plus one debug print |

**Zero** engine modules. **Zero** of the ~1500 Rust test files assert on it except
one (see 4).

**3. `game_setup.rs`'s 57 "display" hits are a red herring — they are a boolean.**
Every one is `action_display_enabled()`, a flag from `core::runtime_flags`, not
the `display` module. I checked all 57 individually. Anyone who reads the grep
count and concludes "the setup screen needs the display module" is wrong.

**4. `describe.c` does NOT cover it, and does not need to.**
`describe.c` (3050 lines, 25 exported entry points at `include/rabuka.h:3234-3258`)
is the **ability-text** projection: it renders one ability's effect/cost/label
into a sentence. `display.rs` is the **board-snapshot** projection: it renders a
whole `GameState` into nested `PlayerDisplay`/`ZoneDisplay`/`StageDisplay`/
`CardDisplay` structs for a UI. They are disjoint projections. What `describe.c`
*does* cover is already parity-proven: `tests/fixtures/describe_golden.txt`
(212159 B) is a genuine Rust oracle produced by the committed
`engine/src/bin/describe_dump.rs`, and the suite is green —
`describe_golden exit=0 out_lines=3 err_lines=0 FAIL=0`.

**5. The C already owns the wire surface, and no test can see it.**
`src/web_server.c`, 724 lines, emits 70 distinct JSON keys. It is linked only by
the `web` target (`Makefile:63-66`); it is **not** in `OBJ_LIB` and **no** test
target links it. So the C web surface is entirely outside the parity harness by
construction, and porting `display.rs` would move nothing measurable.

### The one genuine gap this analysis surfaced — stated, not fixed

Rust's `filter_display_for_player` (`game/web_server.rs:1157` onward, ~100 lines)
is a **hidden-information filter**: it blanks the opponent's hand, hides
unperformed live cards (with the `FirstAttackerPerformance` /
`is_first_attacker` case analysis at :1202-1213), and strips
`effect_attribution` / `ability_applications` for face-down cards so a hidden card
cannot leak its bonus sources.

**The C has no equivalent whatsoever.** `grep -n "filter|fog|hidden|opponent|
waiting_for|rps"` over `src/web_server.c` returns exactly one hit:
`web_server.c:209`, `buffer_put(b, ",\"ui_config\":{\"perspective_player\":0,\"current_lang\":\"jp\"}}")`
— a **hard-coded** perspective of 0. The C web server has a single global view:
every client sees both hands.

That is a real information leak in a target with zero tests, and it is the honest
answer to "is anything from `display.rs` needed". But the fix is **~100 lines of
filtering inside `web_server.c`**, not 2014 lines of display structs. Recording it
here so the next person does not port the whole module to get it.

### DECISION

**`engine/src/game/display.rs` is OUT of scope for the C engine.** Not
`BLOCKED-ON: rabuka.h` — nothing in it is needed, so no declaration is owed. It
was **not** started, per the instruction not to begin without saying so.

If a C web UI is ever wanted, the correct order is: port
`filter_display_for_player` first (self-contained, ~100 lines, closes a real leak),
and only then decide whether the `CardDisplay`/`GameStateDisplay` structs are
wanted as a schema.

---

## Q3 — the bot tree (~21 600 lines in `engine/src/bot`, ~7900 in the bot bins):
**OUT OF SCOPE. Not a parity oracle. Confirmed, and now recorded in
`PROGRESS.md`.**

### (a) Does the C port have a bot? **No.**

`grep -rn "rb_bot|bot_|strategy_v|ismcts|neural|rollout_policy"` across
`engine_c_wip/src/**.c` returns **two hits, both comments, neither code**:
* `src/engine.c:1569` — `* (engine/src/bin/bot_p1.rs is where a policy lives, and it has no C twin).`
* `src/game/deck_builder.c:154` — `* Their Rust callers are engine/src/bin (bad_game_report, bot_arena,`

Measured size (`wc -l`, 26 files, 932667 B total): `strategy_v7.rs` 3690 ·
`v8_model.rs` 2745 ·`v8_live.rs` 2454 · `v7_ismcts.rs` 1660 · `v7_main.rs` 1571 ·
`v8_main.rs` 1468 · `observation.rs` 854 · `strategy_v3.rs` 789 ·
`rollout.rs` 597 · `encoding.rs` 645 · `registry.rs` 536 · `neural.rs` 530 ·
`strategy_v4.rs` 575 · `strategy_v2.rs` 458 · `strategy_v6.rs` 372 · `strategy_v5.rs` 437 ·
`v7_live.rs` 352 · `conductor.rs` 257 · `strategy.rs` 264 · `strategy_common.rs` 243 ·
`determinization.rs` 218 · `ismcts.rs` 187 · `weights.rs` 74 · `evaluation.rs` 11 ·
`mod.rs` 133. **≈21 561 lines.**
Bot bins: `bot_arena.rs` 3052 · `live_calibrate.rs` 894 · `human_vs_v7.rs` 861 ·
`sim_bench.rs` 978 · `ppo_collect.rs` 631 · `ppo_learn.rs` 490 ·
`ppo_train.rs` 437 · `bot_replay.rs` 549 = **7892 lines** in the eight named,
plus ~3800 in the remaining 18 bins.

`PROGRESS.md:133-136` already declared `engine/src/bot/**` and
`engine/src/bin/**` out of scope. This section is the **evidence** for that
declaration, which the declaration itself did not have. (Its `~11.8k` and
`~6.4k` figures are stale — the real numbers are 21 561 and ~11 700.)

### (b) Do they depend on engine internals the C does not yet replicate? **Yes —
several, and they are the blocking kind.**

`use crate::` across `engine/src/bot/*.rs` resolves to:

`crate::rng::Lcg` · `crate::core::stats_pipeline` · `crate::core::constants::CountCast` ·
`crate::game_setup::{Action, ActionParameters, ActionType}` ·
`crate::turn::TurnEngine` · `crate::card::{BaseHeart, CardDatabase, CardType, HeartColor, HeartMap}` ·
`crate::ability::types::Choice` · `crate::game_state::{GameResult, GameState, Phase, TurnPhase}` ·
`crate::player::Player`

`crate::rng::Lcg` is the decisive one: the bot's determinism, its rollout seeds
and its `policy_seed` are all a specific LCG the C port does not have. Porting a
bot without that LCG does not produce a comparable bot, it produces a different
one — and "different" is indistinguishable from "wrong" when you have no oracle.
`TurnEngine` and `stats_pipeline` are engine-internal surfaces still being filled
in. **So the honest answer for (b) is: blocked on engine parity, and therefore
out of scope until parity.**

### (c) **IS ANY BOT A PARITY ORACLE? NO. This is the load-bearing finding.**

Four independent checks, all negative:

1. **No Rust test touches the bot.** `grep -rl "rabuka_engine::bot|use crate::bot|bot::"`
   over `engine/tests/**` (the whole ~135 000-line corpus, ~1500 files) returns
   **0 files**. The oracle that C is measured against (`engine/tests/run_all.rs`
   `scenario_oracle`, the `CHECK <n> <pass|FAIL> <actual> <expected>` format the
   C's `tests/replay.c` reproduces) imports **only** `helpers::TestGame` and
   `std::collections::HashMap`. It does not construct, import or consult a policy.

2. **The parity fixtures are hand-written scenarios, not bot output.**
   `engine_c_wip/tests/fixtures/` holds exactly six files: `debut.json` (84 B),
   `live.json` (88 B), `describe_golden.txt` (212159 B — from `describe_dump.rs`),
   `scenario_eri_w1.txt` (329 B), `scenario_seq_cost.txt` (352 B),
   `scenario_diff_w1.txt` (292 B). **None is bot-generated.** And the one golden
   that *is* a committed Rust artefact (`describe_golden.txt`) comes from
   `engine/src/bin/describe_dump.rs` — a dumper, not a policy.

3. **No in-repo expectation artefact mentions a bot.** `grep -rl` over every
   `*.json`/`*.txt`/`*.csv`/`*.md` in the repository (excluding `.git`,
   `target`, `node_modules`) for `bot_arena|bot_replay|strategy_v7|strategy_v8|
   sim_bench` returns **only `docs/*.md` and `.kilo/worktrees/*/docs/*.md`** —
   20 prose files. There is no machine-readable bot output anywhere that C could
   be diffed against.

4. **The one real edge is in `replay.rs`, and it is not on the measured path.**
   `engine/src/replay.rs:1` is `use crate::bot::PublicObservation;`, and its
   `observe(gs)` (`:57-83`) embeds
   `"observations": [PublicObservation::from_state(gs, 0), ...from_state(gs, 1)]`
   in the snapshot JSON. **So `bot/observation.rs` (854 lines) IS reachable from
   the Rust replay format.** But:
   * `observe()` is called from exactly two places: inside `replay.rs` itself, and
     **`engine/src/bin/sim_bench.rs`** — a bin, out of scope.
   * The C side has no twin of that snapshot format at all: `grep -rn
     "observation|PublicObservation"` over `engine_c_wip/src` + `tests` +
     `tools/*.c` finds the word only inside unrelated identifiers in 12 parity
     suites (`condition.c` and 11 `test_parity_*.c` files, e.g. an
     "observation" in a comment). `tests/replay.c` is the *scenario* replayer and
     emits `CHECK` lines, not observations.
   * Therefore `PublicObservation` is a **format** dependency of a Rust-only
     profiling/replay tool, not an oracle dependency of the parity harness.

**Answer to (c): no bot is used as the oracle for parity tests. Not one. The
oracle is `scenario_oracle` (hand-written scenarios replayed through
`helpers::TestGame`) plus `describe_golden.txt` (from a dumper).**

### DECISION

**OUT OF SCOPE. Not on the critical path. Not an oracle. Do not port, and do not
treat the absence as a parity gap.** Recorded in `PROGRESS.md`.

The one caveat worth carrying: if this project ever wants **differential bot
testing** (C bot vs Rust bot, same seed, same games, compare traces), then
`bot/observation.rs` + `crate::rng::Lcg` become load-bearing and this decision has
to be revisited. That is a future project, not this one.

---

# Q4 — `src/test_game.c`: the harness audit

`src/test_game.c` is **1391 lines** (`wc -l`) in the working tree, not the 1269 the
brief quotes — the difference is the uncommitted baton-touch / active-seat /
double-baton hunks described below, which are already on disk and which this audit
read and kept.

## Q4.0 — the headline number

```
sizeof(GameState) = 1113736 bytes   (1087.6 KiB)
sizeof(TestGame)  = 1113736 bytes
sizeof(RbPlayer)  =   20824 bytes
sizeof(RbBag)     =    2052 bytes
```

Measured by `tests/test_harness_self.c` §A running in the snapshot, and printed on
stdout so it is evidence rather than a claim:

```
sizeof: GameState=1113736 TestGame=1113736 RbPlayer=20824 RbBag=2052
ok: A1 TestGame wraps exactly one GameState (sizes equal)
ok: A2 sizeof(GameState) keeps RB_MIN_SAFE_LOCALS of them inside the 16 MB linked stack
ok: A3 the multiplication the ceiling is derived from actually holds
```

### ⛔ THE MAKEFILE HEADER IS WRONG AND I DID NOT EDIT IT

`Makefile:3-8` currently says:

```
# sizeof(GameState) is 885912 bytes (864.2 KiB) — MEASURED 2026-10-03 by
# sizeof() in tests/test_harness_self.c §A, not estimated ...
```

That figure is **stale by 227 824 bytes**. The same comment block even records
that the previous value (~780 KB) "was wrong by 105912 B" — so this header has
been wrong twice, and is wrong a third time now, while claiming to be measured.
The correct value is **1113736**, and the `RB_GAMESTATE_BYTE_CEILING` that §A
derives it against is `16777216 / 15 = 1118481`, leaving **4745 bytes** of headroom
— about three ints. The next field anyone appends to `GameState` will breach it,
and the correct response is to convert stack locals to `static`, not to raise the
number. **Left for you to apply**, as instructed; `Makefile` is shared and not in
this agent's owned set.

## Q4.1 — every helper, audited against its Rust twin

Rust twin: `engine/tests/helpers/mod.rs` (584 lines) + `choices.rs` (19190 B) +
`assertions.rs`. Method: all 40 `pub fn` in the Rust helper corpus listed, then the
C's 118 function definitions in `src/test_game.c` listed, then paired.

### DIVERGENCE 1 — `test_pending_choice_count` returned a CONSTANT. **FIXED.**

```c
int test_pending_choice_count(TestGame *tg){ return rb_has_pending_choice(&tg->state) ? 1 : 0; }
```

Rust `pending_choice_count` (`engine/tests/helpers/choices.rs:421-428`):

```rust
match self.pending_choice_view() {
    Live(Choice::SelectCard { count, .. }) => *count,
    Live(_)                               => 0,
    Json(pc) => pc["count"].as_u64().map_or(0, |c| c as usize),
    None                                  => 0,
}
```

The answer is **how many cards the prompt asks for**, not "is one standing". The C
returned 1 for *every* prompt that existed, so it was right only for a
`SelectCard { count: 1 }` and silently wrong for every other SelectCard.

This is the same shape as the `test_id` aliasing trap and just as consequential: a
suite asking "how many picks do I owe" was told 1 whatever the engine said. Fixed
in `src/test_game.c` to read `RbChoice.count` for `RB_CHOICE_SELECT_CARD` and 0
otherwise. The Rust `Json` arm has no analogue because the C models every pending
prompt as a flat `RbChoice` — documented, not silently dropped.

**A/B MEASURED, with the baseline proved by a file-hash sweep.** The baseline was
*this snapshot with only `test_pending_choice_count`'s body reverted*, never git
HEAD. A hash sweep of every `.c`/`.h`/`Makefile`/`.sh`/`.py`/`.txt` in both trees
reports **exactly one differing file: `src/test_game.c`**. Both trees were built
from scratch with all `.o` deleted, and `src/test_game.o` mtime was confirmed
*later* than `src/test_game.c` in the baseline build, so neither run measured a
stale object.

Six suites call the helper. Result:

| suite | base FAIL | fixed FAIL |
|---|---|---|
| p1_helpers | 1 | 1 |
| parity_score_per_card2 | 2 | 2 |
| parity_cost_and_effect | 1 | 1 |
| parity_live_total2 | 17 | 17 |
| parity_look_per_card | 74 | 74 |
| parity_characterization_extra | 45 | 45 |

`p1_helpers`, `parity_score_per_card2`, `parity_cost_and_effect`,
`parity_live_total2` and `parity_look_per_card` are **byte-identical on both
streams after normalising heap addresses** (verified file-by-file, not by eye). The
`[RESUME_CAPTURE]` lines do differ in their `0x…` values — one hex digit — because
the reverted/fixed `test_game.c` have different sizes and therefore different heap
layouts; every non-pointer field (`cur=`, `n=`, `state=`, `pending=`) matches.

`parity_characterization_extra` is the one that moved, and it moved **toward the
truth**:

| §K check | expected | base got | fixed got |
|---|---|---|---|
| `pb1007_cost_1_with_2_success` | 1 | 1 (right by luck) | **3** |
| `pb1007_cost_2_with_1_success` | 2 | 1 | **3** |
| `pb1007_cost_3_with_0_success` | 3 | 1 | **3** ✅ |

The constant-1 body accidentally passed the first check and failed the third. The
fixed body passes the third and fails the first two — and the value it reports,
**3**, is the engine's genuine `RbChoice.count` for that cost prompt, so the two
residual reds are now real engine/expectation questions instead of harness noise.
Net FAIL count unchanged at 45; the information content strictly improved. **No
suite got redder.**

### DIVERGENCE 2 — Rust's `fill_decks` had NO C twin, and the C's nearest name is a trap. **FIXED (added).**

```
Rust  fill_decks(game, filler)   clears BOTH main decks, then pushes 30 filler into each.
C     test_filler_hand(tg)       RETURNS a filler card id. Mutates NOTHING.
```

Nothing else in the C was named like `fill_decks`, so a transpiled
`fill_decks(game, filler)` had nowhere to land, and a porter reading
`test_filler_hand` could reasonably have believed the deck had been filled when it
had not been touched. Added `test_fill_decks(tg, filler)` — including the **clear**,
which is the load-bearing half: a deck the fixture meant to be empty otherwise
keeps whatever the setup put in it.

### DIVERGENCE 3 — Rust's `fill_energy_deck` had NO C twin. **FIXED (added).**

`test_add_to_energy_deck` existed (the per-card push) but there was no
`fill_energy_deck(game, player, count)`, which resolves `"LL-E-001-SD"` once and
pushes `count` copies into seat `player`'s **energy deck** — the draw pile. Added,
with the trap documented in the header: it does **not** raise `energy_active`, so a
suite wanting active energy must still call `test_give_energy_for`.

### DIVERGENCE 4 — Rust's `stage_occupant` had NO C twin. **FIXED (added).**

Rust's own doc for it (`mod.rs:437-440`) says it "lets a test assert its own setup,
which is what would have caught §8.3's vacuous negative". The C inlined the same
comparison inside `test_play_to_stage`'s stderr diagnostic, so the *check* existed
but a test could not *call* it: **137 tests** had the baton-touch hazard and none
of them had a way to ask. Added `test_stage_occupant(tg, card_id, area)` returning
the occupant that is not the card being played, or -1 (Rust's `None`).

### DIVERGENCE 5 — Rust's `id_ref` had NO C twin. **FIXED (added), with the residual divergence documented.**

`id_ref` (`mod.rs:245-253`) is a **peek**: "what would the next `id()` hand out for
this card_no?" without advancing the counter. Added. It necessarily diverges in one
way, and the header says so: the C's `test_id` has **no per-card counter at all**
(it is `rb_find_card_by_no`, the shared template, on every call — §C below), so in
the C `id_ref` and `id` always agree, while in Rust they differ by one pool
position. The C cannot do better without reimplementing Rust's pool, which is the
aliasing fix itself and out of scope here.

### HELPERS CONFIRMED FAITHFUL (no change needed)

| C helper | Rust twin | verdict |
|---|---|---|
| `test_give_energy` / `_for` | `give_energy` / `give_energy_for` | faithful — and notably both use the **aliasing** `id("LL-E-001-SD")`, so all N energy cards are one id in both languages |
| `test_insert_deck_top` | `put_on_deck_top` (`cards.insert(0, x)`) | faithful, including the saturating bag |
| `test_play_to_stage` | `play_to_stage` | faithful — C cannot panic, so the guard is a **loud stderr line** (`[TEST_BATON_TOUCH]`) that deliberately does **not** begin with `FAIL`, so a legitimately-batoning suite stays green and is still told |
| `test_try_play_to_stage` | `try_play_to_stage` (no guard) | faithful — routed straight to `_for`, not through the panicking twin |
| `test_play_to_stage_baton` | `play_to_stage_baton` | faithful, **added by the uncommitted working-tree hunks**, not by this audit |
| `test_play_to_stage_double_baton` | `play_to_stage_double_baton` | faithful, likewise added. Not cosmetic: it is the only path that records two touches, which is what `PL!-pb2-000-DUO`'s `min_count = 2` gate needs |
| `test_play_to_stage_for` / `test_activate_ability_for` | `try_play_to_stage_for` / `activate_ability_for` | faithful **and already corrected in the working tree**: both now call `test_set_active_seat` first, mirroring Rust's `set_active_side` (8 call sites were reading the wrong attacking seat). Pinned by §H |
| `test_activate_ability_for` | `activate_ability_for` | faithful — the C already falls through to `rb_activate_card(g, pl, card_id)` when the card is not in hand, which is what lets a **stage**-resident card activate. I checked for this divergence and it is not there |
| `test_select_indices` | `select_indices` | faithful (`n == 0` is a DECLINE via `rb_resume_with_choice(g, -1)`, not a no-op) |
| `test_pending_choice_type` | `pending_choice_view` | faithful, including `""` (not `"Unknown"`) when quiet |
| `test_add_to_discard` | `add_to_discard` | faithful — `RbBag discard` is documented as the waitroom (`rabuka.h:770`) |
| `test_set_live_card` | `set_live_card` | faithful (appends; the `slot` argument is deliberately ignored because `tools/port_one.py` always emits 0) |
| `test_advance_to_phase_strict` | `advance_to_phase` | **documented divergence**: Rust drains mid-walk with `select_indices(&[0])` and hands the arrival prompt to the caller; the C's `_strict` never answers anything. `test_advance_to_phase_rust` reproduces Rust's drain. Both documented in `test_game.h` |
| `test_get_heart_modifier` | `get_heart_modifier` | faithful passthrough — the old `color == 5 → RB_HEART_ORANGE` remap is gone (it made every heart05 read return the heart06 slot) |
| `rb_card_no_eq` | n/a (C-only) | faithful, and refuses empty/NULL/negative so `""` cannot match every card |

### MISSING TWINS — REPORTED, NOT BUILT

Rust helpers with no C spelling that this audit did **not** add, because each needs a
design decision rather than a transcription:

* `activate_ability_index` / `try_activate_ability_index` (`mod.rs:478-499`) — needs
  the C's ability-indexed action entry point to be confirmed as
  `rb_execute_main_phase_action`'s indexed sibling before a helper may claim it.
* `drain_choices_strict(allowed, answer)` (`choices.rs:358`) — the anti-soft-guard
  primitive. An empty `allowed` list must make any prompt a failure. Porting it
  carelessly is exactly how a "vacuous" drain gets reintroduced.
* `select_option(option_index)` (`choices.rs:297`) — dispatches per choice kind
  (`SelectHeartColor` needs `select_option`, `SelectPosition` needs
  `select_generated`), so a one-line wrapper would be **wrong**.
* `push_energy_zone_change(game, player)` and `scan_autos_both(game)`.

None of these is load-bearing for a suite that exists today (0 call sites each) —
that is why they are listed rather than guessed at.

## Q4.2 — the harness self-test

`tests/test_harness_self.c` already existed on disk (untracked) from the
predecessor's first session: sections §A-§J. It was **RED on exactly one check**
when I inherited it, and it now covers the new ground too.

### The inherited red: D1, `test_id(` call-site ratchet

```
FAIL: D1 test_id( call sites in tests/ stay <= 3011 (now 3013)
```

Not a real hazard — a **baseline that had drifted**. The bound was measured earlier
the same day; four new suites landed since, and one of them
(`tests/test_parity_state_family.c`, 4 `test_id(` lines) accounts for the +2. The
bound was rebaselined to the measured **3013** with the delta attributed in the
file, because raising a ratchet to the true current value with the reason written
down is the documented procedure for that check — not a weakening of it. `D2`
(files, 102) did **not** move, which is the check working as intended: the growth
was inside existing files, not new ones.

### Sections added by this audit

* **§K — count-shaped helpers (6 checks).** A helper that answers a *constant* is
  not a helper; that is how the old `? 1 : 0` body hid. §K stages
  `SelectCard{count:1}`, `{3}`, `{5}` and a `SelectAutoAbility`, and requires 1, 3,
  5 and **0** respectively. The 0 is the load-bearing one: a non-SelectCard prompt
  owes no picks.
* **§L — the missing twins (11 checks).** Pins `test_filler_hand` as
  return-only-and-mutates-nothing (the trap), `test_fill_decks` clearing both seats
  to 30, `test_fill_energy_deck` writing the deck and **not** active energy,
  `test_stage_occupant`'s three arms (empty → -1, self-exclusion, other query), and
  `test_id_ref`'s documented agreement with `test_id`.

### Final state — measured

```
scanned tests/: 1432 non-static state locals in 75 files; 3013 test_id( calls in 102 files
ok: A1/A2/A3   ok: B1/B2   ok: D1/D2   ok: C1..C8   ok: E1..E8   ok: F1/F2
ok: G1..G22    ok: H1..H7  ok: I1..I3   ok: J1..J9   ok: K1..K6   ok: L0..L10
GREEN: 0 checks failed
```

```
harness_self   exit=0  out=90  err=4  FAIL=0  longest=309
```

90 stdout lines, 4 stderr lines, zero `^FAIL` on either stream (counted with
`-CaseSensitive` on the **pattern**, so a suite's own `failures : 54`-shaped summary
line cannot be mistaken for a failure). It compiles with **zero warnings** from
`tests/test_harness_self.c` (an earlier revision returned `const RbChoice *` from a
`stage_choice` helper and tripped `-Wdiscarded-qualifier`; fixed).

### Suites named in the brief that DO NOT EXIST — verified before running

Per the brief's own warning, each was checked with `Test-Path` before use:

| name in circulation | exists? |
|---|---|
| `test_deck_parser` | yes |
| `test_deck_builder` | yes |
| `test_describe_golden` | yes |
| **`describe_golden`** | **NO** — the real name is `test_describe_golden` |
| **`test_game_harness`** | **NO** |
| `test_harness_self` | yes (created by the predecessor; this cluster's deliverable) |
| **`parity_card_text`** | **NO** |
| **`parity_dynamic_count`** | **NO** |
| **`audit_fields`** | **NO** |
| **`audit_cards`** | **NO** |
| `test_mechanics` | yes (the brief's `mechanics` is a make **target**, not a file) |
| **`parity_sequential`** | **NO** — the scenario fixtures are `scenario_eri_w1.txt`, `scenario_seq_cost.txt`, `scenario_diff_w1.txt` |
| **`parity_wave1a`** | **NO** |
| **`parity_wave1b`** | **NO** as such — `tests/test_parity_wave1b.c` exists |
| **`parity_wave1c`** | **NO** |
| `test_artifacts_consistent` | yes |

**What was actually run** (snapshot, strictly sequential, `stdout` and `stderr`
captured to separate files via `Start-Process -RedirectStandardOutput/-RedirectStandardError`;
`make run-all-tests` never invoked):

```
choice_renderer         exit=0  out=75  err=0   FAIL=0   longest=139
deck_parser             exit=0  out=64  err=0   FAIL=0   longest=74
deck_builder            exit=0  out=63  err=0   FAIL=0   longest=77
describe_golden         exit=0  out=3   err=0   FAIL=0   longest=91
artifacts_consistent    exit=0  out=6   err=0   FAIL=0   longest=113
mechanics               exit=0  out=31  err=111 FAIL=0   longest=104
harness_self            exit=0  out=90  err=4   FAIL=0   longest=309
p1_helpers              exit=1  out=0   err=9   FAIL=1   longest=83
parity_score_per_card2  exit=1  out=226 err=89  FAIL=2   longest=221
parity_characterization_extra exit=19 out=127 err=3225 FAIL=45 longest=309
parity_look_per_card    exit=23  out=120 err=341 FAIL=74  longest=156
parity_cost_and_effect  exit=1536 out=14 err=13 FAIL=1   longest=87
parity_live_total2      exit=1  out=527 err=134 FAIL=17  longest=280
```

The longest captured line is 309 bytes, far above the 119-character
`2>&1 | Out-File` truncation threshold, so no line was cut. **Exit code is not a
signal** and was not used as one: `parity_cost_and_effect` exits **1536** and
`parity_characterization_extra` exits **19**, neither of which is 0 or 1, and both
carry FAIL lines. `p1_helpers` emits **0 bytes on stdout** — its whole report is on
stderr, which is exactly the "a suite producing no stdout is not thereby passing"
case.

`p1_helpers`' single FAIL is pre-existing and **not** from this cluster's changes:
it is byte-identical between the baseline and the fixed tree.

---

# Q5 — FOUND WITHOUT FIXING, stated precisely

1. **`Makefile:3-8` carries the wrong `sizeof(GameState)`** (885912; the truth is
   **1113736**). Not edited — reserved for you, and the file is shared. The comment
   also asserts the number is "MEASURED by §A of test_harness_self.c", which makes
   the staleness look like a measurement. **This is the highest-priority follow-up**,
   because the 16 MB stack flag's headroom calculation rests on it and §A's own
   ceiling has only 4745 bytes left.

2. **The `choice_renderer` build-graph fix is UNCOMMITTED.** `Makefile` is `M` in
   `git status`; `HEAD = f152a50f` still does not list `src/ability/choice_renderer.c`.
   Anyone who checks out the tree, or diffs against HEAD, sees a file that is not
   built. If that edit is ever lost, 1963 lines silently vanish from every binary
   and no compiler complains. **This is the second-highest priority.**

3. **`src/core/alloc_counter.c` is unwired and one `-std` bump from broken.** Not
   fixed: `src/core/**` and `include/rabuka.h` are outside this agent's owned set.
   Recommendation and evidence in Q1b; it needs an owner's sign-off because it adds
   775 lines of allocator instrumentation to every link target.

4. **`engine_c_wip/src/web_server.c` leaks the opponent's hand.** No hidden-information
   filter at all; `web_server.c:209` hard-codes `"perspective_player":0`. Rust's
   `filter_display_for_player` (`game/web_server.rs:1157+`, ~100 lines) is the fix.
   Not fixed: `src/web_server.c` is not in this agent's owned set, and the file is
   linked by no test, so there is no measurement to prove the fix with.

5. **`B1`/`B2` are AT their ceiling, not below it.** 1432 non-static state locals in
   75 files, against bounds of 1432 and 75. Any new suite that declares one
   `GameState tg;` locally turns the harness self-test red. That is the ratchet
   working, but it means the next agent to add a suite will meet a red harness and
   may not know why.

6. **`D1`'s baseline had already drifted once** (3011 → 3013 in a single day). Two
   ratchets in one file, both measured rather than reasoned, and one of them moved
   within hours of being written. Anyone adding a suite should expect to re-baseline
   and should read the file's comment first.

7. **`parity_cost_and_effect` exits 1536 and `parity_characterization_extra` exits
   19**, and `p1_helpers` writes nothing to stdout. All three are consistent with the
   traps in the brief and all three are still true after this cluster's changes.

---

# WHAT WAS CHANGED (three files, all inside this agent's ownership)

| file | change |
|---|---|
| `src/test_game.c` | `test_pending_choice_count` made faithful; `test_id_ref`, `test_fill_decks`, `test_fill_energy_deck`, `test_stage_occupant` added |
| `include/test_game.h` | declarations for the four new helpers, with the divergences and the `fill_decks`/`test_filler_hand` trap written on them |
| `tests/test_harness_self.c` | §K (6 checks) and §L (11 checks) added; `D1` rebaselined 3011 → 3013 with the delta attributed; the `-Wdiscarded-qualifier` warning removed |
| `ORPHANS_AND_SCOPE.md` | this file |
| `PROGRESS.md` | the Q2 and Q3 scope decisions, so they are not re-litigated |
| `PARITY_CHECKLIST.md` | cluster T8 |

No git write of any kind was performed. `stash@{0} session-wip`
(`c458ab6f85ce5559041defd25165789b62cd8f21`) was never popped, applied or dropped.
The live tree was only read, plus the three owned files above; the live tree holds
**0** `*.o` and **0** `rb_engine*` artifacts. The snapshot's `.o` files were purged
on completion.
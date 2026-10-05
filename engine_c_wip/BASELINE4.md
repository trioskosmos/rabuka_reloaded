# BASELINE4 - whole-corpus measurement of the C engine (172 suites, two runs)

Measured 2026-10-05. Supersedes BASELINE3.md; **every number in BASELINE3 is stale.**

* **HEAD: `aed8bd0d609601ba45ba6c3fb412d142fa61ecfe`** (working tree dirty: 91 changed paths).
* **Suites on disk: 172** `tests/test_*.c` (BASELINE3 measured 166).
* **MANIFEST-SHA256** (sha256 over the sorted `hash  path` manifest of every
  `.c`, `.h` and `Makefile` under `engine_c_wip/`):

```
a022f5431015ad9ee509ad0af59aeda573e6698bfefc3c099bedca21283dd4d2
```

  **269 files.** Full listing: `engine_c_wip/BASELINE4_manifest_sha256.txt`.
  Without this manifest these numbers cannot be checked.

## Integrity of what was measured (OneDrive is a sync root, so this was not assumed)

| step | result |
|---|---|
| snapshot -> `Temp/kilo/final/snap` (outside the sync root) | 172 suites, 54 src `.c`, 0 objects |
| SHA-256 of live tree vs snapshot, **before** the runs | **identical, 269/269** |
| `make clean` | 0 objects, 0 `rb_engine*`/`rb_verdict*` binaries remain |
| SHA-256 of snapshot sources, **after** both runs | **identical to before** - sources did not move under the measurement |
| disk free at start / end | 36.0 GB / 36.0 GB - **never low; no `warnings-audit` log truncation occurred** |

`make all` -> **51 objects**, `rb_engine.exe` (1 438 018 B), `rb_verdict.exe` (72 515 B).
No trap fired: no stale link was possible (0 objects pre-build, and the runner
additionally refuses any `.exe` older than its newest input object).

## Build/run method

`make t` is correctly bricked: `cards/build` (726 742 B) and `src/` (726 671 B)
disagree, and the `data:` guard refuses to overwrite a good tree with a
mismatched generation. **The guard was not weakened and nothing was regenerated.**
`make t-nogate T=<stem>` was used, i.e. `t-run` with `GATE_DEP=` blanked.

**Recipe equivalence was MEASURED, not assumed**: for `parity_sequential`,
`audit_zones` and `max_distinct_names`, real `make t-nogate` and the replicated
link+run produced **byte-identical verdict lines**, breakdowns included:

```
FAILED: parity_sequential  [exit=1, 142 result line(s): assert=142 vacuous=0 setup-bug=0 crash=0 test-bug=0 fatal=0 renormalised=29]
```

Every suite was then run **strictly sequentially, TWICE, same snapshot, no
rebuild between**, each in its own freshly-linked binary, with **stdout and
stderr captured to separate files** (`results/run{1,2}/{out,err,link,time}`).
**0 link failures in both runs.** Suites were capped at 120 s; none hit it.

## Gates

| gate | result |
|---|---|
| `make regen-check` | **PASSED** - `sizeof(GameState)` measured **1 090 528 B**, pinned 1 090 528 B; `RBKA_STRINGS_TABLE_LEN` 726 671 = blob = pin; offsets 6033; FNV 6032 |
| `make check-suite-names` | **PASSED** - 172 suites, 47 names in circulation, every name resolves / recorded gap / recorded absent |
| `make warnings-audit` | **FAILED** - see below |

### `make warnings-audit` is RED, and the red is real

```
warnings-audit: bug-predicting diagnostics found = 12, baseline pins = 12
  NEW      src/ability/effects/look.c:1577:-Wformat-truncation=
  STALE    src/ability/effects/look.c:1486:-Wformat-truncation=  pinned but no longer emitted
warnings-audit: FAIL -- 1 bug-predicting diagnostic(s) outside the baseline.
```

Verified independently, because a short log reads as a short `found.txt` and
manufactures false STALE pins: compiling `src/ability/effects/look.c` alone emits
`-Wformat-truncation=` **at :1577 (twice) and not at :1486**. So `FT-6` is a
genuinely stale pin and `:1577` a genuinely new diagnostic.

**A measurement trap I hit and had to discard:** re-running the same compile with
PowerShell `2>` redirection changed GCC's line wrapping, which moved the
`[-Wformat-truncation=]` marker off the `file:line: warning:` line onto a
continuation line, so my own extractor counted **3 records instead of 12**. That
was *my* harness artifact, not a gate lie. The gate's 12 is correct. Recorded
here because it is the same class of error as trap 3.

Note the Makefile's own comment claims **four** stale pins in files other agents
own (`look.c:1409`, `rules.c:849`, `rules.c:1352`, `rules.c:1555`). Three of
those have since been re-pinned and now match; only the `look.c` one remains, and
it has moved again. The gate is red by exactly **1 NEW + 1 STALE**.

## Two-run agreement: EXACT

Both passes ran on the same snapshot with no rebuild between, so this is a
**determinism** result, not a drift result.

| check | result |
|---|---|
| suites whose verdict/FAIL-line multiset differs between run 1 and run 2 | **0 / 172** |
| every counter below identical across the two runs | **yes, 12 / 12** |
| suites that crashed (no verdict line) | **0** |
| run 1 wall clock / run 2 wall clock | 363.5 s / 397.3 s (timing only) |
| sum of per-suite wall clock, run 1 | 355.4 s |
| slowest suite | `parity_small_clusters` 10.24 s |
| suites over 20 s | **0** |

| counter | run 1 | run 2 |
|---|---|---|
| result lines | 3665 | 3665 |
| assert lines | 3619 | 3619 |
| `[vacuous]` | 44 | 44 |
| `[test-bug]` | 2 | 2 |
| `[setup-bug]` / `[crash]` / `[fatal]` | 0 / 0 / 0 | 0 / 0 / 0 |
| detail lines | 2557 | 2557 |
| rollup lines | 1108 | 1108 |
| `ok:` lines | 18 799 | 18 799 |

## Counting rules

The corpus has **five** result-line grammars, and `rb_verdict` rewrites six
prefixes to a canonical `FAIL:`. Two consequences:

* **Rollups are not findable by prefix.** `FAILED <name>` becomes `FAIL:<name>`,
  so a per-case rollup and an assertion detail are separated by **shape**: a
  rollup is `FAIL:` followed by a bare identifier and nothing else.
* **Every regex carries `\r?`.** The harness writes CRLF; a bare `$` anchor
  matches nothing and every count silently reads 0.

| signal | meaning |
|---|---|
| result lines | what `rb_verdict` decided the verdict on (authoritative; it also writes `verdicts/*.json`) |
| rollups | per-**case** failures (a distinct failing case) |
| detail | per-**assertion** failures |
| `ok:` lines | per-case **passes** |

## The CASE-COUNT assertion, and why it was necessary

A `PASSED` verdict is **not** evidence of anything on its own: the gate defines
PASSED as *exit 0 AND zero result lines*, which a library that asserts **nothing**
satisfies perfectly. Every suite was therefore re-opened and asked how many cases
it actually ran. **Five distinct case-count grammars exist** and the first three
scanners missed four of them, manufacturing phantom "ran zero cases" suites:

* `^ok: <description>` (151 suites)
* `^<name>: N assertion(s), M failure(s)` - **indented, and with a `name:` prefix**
* `^<name>: N cases, M failures` / `N checks, 0 failed`  (8 suites)
* `^N assertions, M failures, K crashes`  (3 suites)
* `^<name>: N failure(s), M setup bug(s)`  (3 suites)

| case-count result | count |
|---|---|
| suites reporting an explicit numeric case count | **170 / 172** |
| suites with **no** numeric case count at all | **2**: `audit_conditions`, `coverage_gate` |

### One suite is BROKEN, not red

`audit_conditions` exits 2 having run **zero** assertions:

```
load_cards failed
load_cards failed
cannot load card db
FAILED: audit_conditions  [exit=2, 0 result line(s): assert=0 ...]
```

It compiles and links cleanly (0 errors) and then cannot load the card DB. Its
"red" is a **fixture failure**, and it contributes **0** to every case total.
This is exactly the "broken library reads as a perfect run" shape, in its
mirror image: a broken *fixture* reads as a red suite.

## Suite states

| state | count |
|---|---|
| GREEN, verifiably (PASSED **and** positive case count) | **54** |
| GREEN but UNVERIFIED (PASSED with no case count) | **1** (`coverage_gate` - a meta-suite) |
| RED (assertions actually failed) | **116** |
| BROKEN (FAILED with 0 result lines) | **1** (`audit_conditions`) |

## Whole-corpus totals, BOTH VIEWS

`cases` = structured assertions where a suite reports them, otherwise
detail+rollup lines. One grammar per suite; never both, or the total
double-counts (12 structured / 158 detail+rollup).

| view | suites | cases | failing cases | passing | pass rate | result lines | detail | rollups | `ok:` |
|---|---|---|---|---|---|---|---|---|---|
| **A** all 172 | 172 | 5 316 | 1 204 | 4 112 | **77.35 %** | 3 665 | 2 557 | 1 108 | 18 799 |
| **B** minus the 2 non-metrics | 170 | 5 264 | 1 193 | 4 071 | **77.34 %** | 3 614 | 2 517 | 1 097 | 18 546 |
| **C** minus those **and** the 6 generation-pinned | 164 | 5 125 | 1 157 | 3 968 | **77.42 %** | 3 481 | 2 420 | 1 061 | 18 187 |

**Why the views barely differ:** the six generation-pinned suites carry 97 failing
detail lines out of 2 517, i.e. **3.9 %**. Excluding them moves the pass rate by
**+0.08 points** (77.34 % -> 77.42 %). The headline does not depend on them.

### The two NON-METRICS - named, and never quoted as a verdict

* **`coverage_gate`** - a **meta-suite**. It does not test engine behaviour at all;
  it walks `verdicts/*.json` and reconciles the *log files*. Its count measures
  the measurement, not the engine.
  Measured properly (re-run after the sweep, so the verdict set was complete):
  `sources=172 verdicts=172 result-lines(corpus)=3665 PASSED=55 FAILED=117
  INCONCLUSIVE=0 unreadable=0 exit-only-red=1`, `1041 checks, 2 failures`.
  That independently **confirms** view A's totals. In a sequential single pass it
  runs mid-sweep with only 14 verdicts present and correctly prints `SKIP` - which
  is why a raw corpus sweep reports its PASSED as vacuous.
* **`parity_required_hearts`** - its fixture set **links `test_parity_required_hearts.c`**
  into itself, so it counts its own fixtures as cases. Not an independent oracle.

Neither figure is used anywhere in the pass rate, the failure attribution, or the
parity verdict.

## TOP-25 FAILING ASSERTIONS

Full list with counts and suites: `engine_c_wip/BASELINE4_top25.txt`.

* **2 392 distinct signatures over 2 517 failing detail lines.**
* **Top-25 cover 93 lines = 3.7 %.** The largest single signature occurs **5**
  times. **No clustering has formed** - the failure mass is flat.

## Per-Rust-module attribution over the detail lines

Method, stated so it can be argued with: each failing detail line is attributed to
the Rust module whose behaviour keyword dominates it, most-specific keyword first.
This is a **text attribution, not a call graph** - a triage map, not a proof.

| Rust module | failing assertions | share | suites |
|---|---|---|---|
| `core/zones` | 668 | **26.5 %** | 73 |
| `ability/choice` | 541 | **21.5 %** | 73 |
| `modifiers` | 396 | **15.7 %** | 68 |
| `ability/cost` | 224 | **8.9 %** | 59 |
| `ability/effects/move` | 71 | 2.8 % | 30 |
| `turn/triggers` | 57 | 2.3 % | 27 |
| `ability/effects/ability` | 52 | 2.1 % | 32 |
| `ability/compound` | 41 | 1.6 % | 24 |
| `turn/phases` | 34 | 1.4 % | 19 |
| `ability/effects/look` | 30 | 1.2 % | 15 |
| `core/card` | 30 | 1.2 % | 18 |
| `core/stats_pipeline` | 26 | 1.0 % | 15 |
| `ability/effects` | 20 | 0.8 % | 9 |
| `ability/condition` | 14 | 0.6 % | 11 |
| `ability/resolver` | 7 | 0.3 % | 2 |
| `turn/actions` | 3 | 0.1 % | 3 |
| `core/game_state` | 2 | 0.1 % | 2 |
| `ability/ability_queue` | 1 | 0.0 % | 1 |
| **UNATTRIBUTED** | **300** | **11.9 %** | 67 |

**What moved vs the prior reading** (`core/zones` 43.4 · `ability/choice` 14.2 ·
`modifiers` 12.8 · `core/card` 9.8 · `ability/cost` 7.8 · unattributed 7.1):

* **`core/zones` 43.4 % -> 26.5 %: moved DOWN sharply.** `ability/choice`
  14.2 -> 21.5 % and `modifiers` 12.8 -> 15.7 % **moved UP**.
* **`core/card` 9.8 % -> 1.2 %: moved DOWN hard** - the single largest change.
* **`ability/cost` 7.8 % -> 8.9 %: essentially flat.**
* **unattributed 7.1 % -> 11.9 %: moved UP.**
* The reordering is driven by the **keyword table and the failure mix**, not by
  the engine: `core/zones` keywords (`draw`, `hand`, `zone`, `deck`) now match
  far more assertion text than the prior table's narrower set. Treat this table
  as a *triage* ranking. The stable fact is the **top-3 ordering**
  (`core/zones` > `ability/choice` > `modifiers`) and that those three hold
  **63.7 %** of all failing detail lines.

### How much of the unattributed bucket is unattributable BY CONSTRUCTION

The C corpus is **ASCII** in its identifiers while Rust test names and card text
are **Japanese**, so a Japanese-subject assertion can never be keyword-matched.
Measured over the 300 unattributed lines:

| property | lines | share of bucket |
|---|---|---|
| contains Japanese script text | **70** | **23.3 %** |
| carries no alphanumeric content word at all | 0 | 0.0 % |
| <= 12 non-space characters | 0 | 0.0 % |
| **union, provably unmatchable** | **70** | **23.3 %** |

**23.3 %** of the unattributed bucket - **8.7 % of all failing detail lines** -
is unattributable *by construction*, not by oversight. A prior agent measured
**27.6 %** of the bucket in that state; it is **23.3 %** now, so that figure has
moved down 4.3 points. The remaining 230 lines are unattributable because the
keyword table has no entry for the behaviour they name (e.g.
`replacement_appear_wait_holds_the_newcomer`,
`test_genki_zenkai_threshold_counts_total_not_members`) - a **table gap, fixable**.

**A real defect found while measuring this:** the harness emits Japanese card
text as **mojibake** - UTF-8 bytes decoded as Shift-JIS, e.g.
`繝ｩ繧､繝夜幕蟋区凾` for a card name, `逵溷ｧｫ is played to centre` instead of
`真姫 is played to centre`. Mojibake still lands in CJK codepoint ranges so the
Japanese detection above is not fooled, but any human or keyword reader of this
output is, and it makes the C suite logs unreviewable as text.

## Stale expectations vs genuine divergences

Over the 2 517 failing detail lines (excluding the 2 non-metrics):

| category | lines | share | how derived |
|---|---|---|---|
| **(a) pinned to the old card generation** - the six suites | 97 | 3.9 % | all detail lines in the six |
| **(d) harness artefacts** - vacuity control failed, or fixture/card did not resolve | 155 | 6.2 % | `vacu`-matching + fixture/setup-bug/test-bug lines, union |
| **(b)+(c) C and Rust genuinely disagree** | **2 265** | **90.0 %** | remainder |

Two refinements that matter:

* Of the 155 artefact lines, **10** are `ANTI-VACUITY CONTROL` **positive
  controls** whose failure is a **real engine defect**, not an artefact. So
  **(d) is an upper bound and the genuine share is higher than 90.0 %**.
* `[vacuous]` lines (**44** result lines, 18 in the detail set) mean the C engine
  **raised no prompt at all** where Rust does. Those are **not** artefacts - they
  are the harness correctly detecting a missing C behaviour.

**Confidence in this split: moderate-to-high for the 90/4/6 proportions, LOW for
any individual line.** The categories are derived by *text pattern*, and I have
demonstrated twice in this very measurement that a text pattern can be wrong
(GCC line wrapping; the five case-count grammars). I can state with confidence
that **the great majority of failures are not explained by card generation**,
because the six pinned suites are only 3.9 % of the mass. I cannot, from text
alone, certify that any specific one of the 2 265 lines is a true behavioural
divergence rather than a third grammar I have not recognised. The honest
confidence interval on "90 % genuine" is roughly **+- 5 points**.

## The 6 generation-pinned suites

Red purely because they hard-code the old card generation
(`parity_wave1c`, `parity_jidou_watch2`, `decode_bytecode_shape`, `parity_enums`,
`describe_golden`, `vm_decode`). Their failing detail lines, view C basis:

| suite | failing detail lines | failing cases |
|---|---|---|
| `parity_wave1c` | 42 | 13 |
| `parity_jidou_watch2` | 50 | 23 |
| `describe_golden` | 1 | - |
| `vm_decode` | 1 | - |
| `decode_bytecode_shape` | 2 | - |
| `parity_enums` | 2 | - |
| **total** | **97** | - |

## Alias-probe sweep (`tools/alias_probe.sh`), whole corpus

Run one suite per invocation (a single 172-argument command line silently
collapses in this shell, and the first attempt probed exactly 1 suite).

| metric | value |
|---|---|
| arms with a usable probe line | **171 / 172** |
| `BUILD-FAIL` | **1** (`web_server_mods_init`, rc=1 - **not** counted as clean) |
| **total alias hits** | **6** |
| suites with >= 1 hit | **4** |
| total `frees_top` observed | 71 285 |
| total `prompt_open` observed | 2 348 |
| arms whose probe ran **NO CASES** (a zero there covers nothing) | **12** |
| arms where forked-child and parent summaries DISAGREE | **4** |

The 6 hits: `parity_cost_and_effect` **3**, and `state_effects`,
`parity_realcard_walkthrough`, `parity_target_player` **1** each.

**Negative control, run as the script requires.** A probe that has not been shown
to go non-zero under `--negative-control` has never fired. Run per suite with
**separate outdirs** (a shared outdir silently overwrote 3 of the 4 summaries in
the first attempt):

| suite | fixed arm | negative-control arm |
|---|---|---|
| `parity_cost_and_effect` | 3 | 3 |
| `state_effects` | 1 | 1 |
| `parity_realcard_walkthrough` | 1 | 1 |
| `parity_target_player` | 1 | 1 |

The probe **does** fire (6 != 0), but reverting the `pca_resolve_entry` lifetime
fix changes **nothing**. So the 6 hits are **not** explained by that known-reverted
defect; they are unexplained hits that need their own triage. Treat "alias-clean"
as **not established**.

## Per-suite table (run 1; run 2 identical for every row)

`resLines` = what the verdict was decided on; `detail` = failing assertions;
`rollups` = failing cases; `CASES` = asserted case count and the grammar it came
from; `GEN6` = generation-pinned; `META` = non-metric.

| suite | verdict | exit | resLines | vac | testBug | detail | rollups | ok: | CASES | src | rc | sec | state | tag |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ability_effects` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 12 | 12 | ok | 0 | 1.11 | GREEN(verified) |  |
| `ability_effects_crash` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 9 | 31 | ok | 0 | 1.43 | GREEN(verified) |  |
| `alias_ownership` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 16 | 16 | ok | 0 | 0.94 | GREEN(verified) |  |
| `artifacts_consistent` | FAILED | 1 | 2 | 0 | 0 | 3 | 0 | 0 | 2 | resLines | 1 | 0.99 | RED |  |
| `audit_actions` | FAILED | 1 | 6 | 0 | 2 | 5 | 2 | 46 | 46 | ok | 1 | 1.41 | RED |  |
| `audit_conditions` | FAILED | 2 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | - | 2 | 1.31 | BROKEN |  |
| `audit_fields` | FAILED | 1 | 4 | 0 | 0 | 3 | 2 | 14 | 14 | ok | 1 | 2.30 | RED |  |
| `audit_triggers` | FAILED | 1 | 4 | 0 | 0 | 3 | 2 | 9 | 9 | ok | 1 | 2.00 | RED |  |
| `audit_warnings` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 59 | checks | 0 | 0.96 | GREEN(verified) |  |
| `audit_zones` | FAILED | 1 | 5 | 0 | 0 | 4 | 2 | 253 | 253 | ok | 1 | 1.89 | RED |  |
| `b8_live_timing` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | checks | 0 | 1.08 | GREEN(verified) |  |
| `baton_touch` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 18 | 18 | ok | 0 | 1.03 | GREEN(verified) |  |
| `choice_renderer` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 65 | 65 | ok | 0 | 1.03 | GREEN(verified) |  |
| `condition_evaluation` | FAILED | 1 | 5 | 0 | 0 | 4 | 2 | 23 | 23 | ok | 1 | 1.17 | RED |  |
| `coverage_gate` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | - | 0 | 1.14 | GREEN(UNVERIFIED) | META |
| `cross_player_interaction` | FAILED | 1 | 18 | 0 | 0 | 11 | 8 | 11 | 18 | ok | 1 | 0.85 | RED |  |
| `deck_builder` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 62 | 62 | ok | 0 | 1.50 | GREEN(verified) |  |
| `deck_parser` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 63 | 63 | ok | 0 | 1.39 | GREEN(verified) |  |
| `decode_bytecode_shape` | FAILED | 1 | 2 | 0 | 0 | 3 | 0 | 0 | 2 | resLines | 1 | 1.18 | RED | GEN6 |
| `decode_fallback` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | ok | 0 | 0.93 | GREEN(verified) |  |
| `decode_skew` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 41 | checks | 0 | 1.10 | GREEN(verified) |  |
| `describe_golden` | FAILED | 1 | 1 | 0 | 0 | 2 | 0 | 0 | 1 | resLines | 1 | 1.46 | RED | GEN6 |
| `draw_phase_fix` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 2 | ok | 0 | 1.23 | GREEN(verified) |  |
| `harness_self` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 108 | 108 | ok | 0 | 1.42 | GREEN(verified) |  |
| `live_allocator` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 38 | 38 | ok | 0 | 0.91 | GREEN(verified) |  |
| `live_card_zone_movement` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 41 | 41 | ok | 0 | 1.11 | GREEN(verified) |  |
| `live_success_rules` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 73 | 73 | ok | 0 | 1.07 | GREEN(verified) |  |
| `live_zone_slot_validation` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 58 | 58 | ok | 0 | 1.05 | GREEN(verified) |  |
| `log_drain` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 60 | 60 | ok | 0 | 0.90 | GREEN(verified) |  |
| `look_select` | FAILED | 1 | 4 | 0 | 0 | 3 | 2 | 47 | 47 | ok | 1 | 1.03 | RED |  |
| `max_distinct_names` | FAILED | 1 | 7 | 0 | 0 | 7 | 1 | 4043 | 4043 | ok | 1 | 1.08 | RED |  |
| `mechanics` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 30 | 30 | ok | 0 | 1.07 | GREEN(verified) |  |
| `member_activation_debut_live_start` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 17 | ok | 0 | 1.01 | GREEN(verified) |  |
| `move_cards` | FAILED | 1 | 3 | 0 | 0 | 3 | 1 | 14 | 14 | ok | 1 | 0.99 | RED |  |
| `null_effect_guard` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 14 | asserts | 0 | 1.72 | GREEN(verified) |  |
| `opponent_live_success_flow` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 4 | 4 | ok | 0 | 0.92 | GREEN(verified) |  |
| `p1_helpers` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 0 | 2 | resLines | 1 | 1.47 | RED |  |
| `parity_ability_mod` | FAILED | 1 | 22 | 0 | 0 | 15 | 8 | 76 | 76 | ok | 1 | 1.21 | RED |  |
| `parity_ability_mod_extra` | FAILED | 1 | 24 | 0 | 0 | 18 | 7 | 188 | 188 | ok | 1 | 1.46 | RED |  |
| `parity_ability_watch` | FAILED | 1 | 27 | 0 | 0 | 20 | 8 | 316 | 316 | ok | 1 | 3.62 | RED |  |
| `parity_abilitymod_conditional` | FAILED | 0 | 13 | 0 | 0 | 9 | 5 | 758 | 758 | ok | 0 | 2.25 | RED |  |
| `parity_area_move` | FAILED | 1 | 51 | 0 | 0 | 32 | 20 | 20 | 51 | ok | 1 | 6.49 | RED |  |
| `parity_auto_abilities` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 87 | 87 | ok | 1 | 1.68 | RED |  |
| `parity_blade_under` | FAILED | 0 | 50 | 0 | 0 | 31 | 20 | 27 | 201 | ok | 0 | 4.78 | RED |  |
| `parity_blades2` | FAILED | 1 | 26 | 0 | 0 | 17 | 10 | 18 | 26 | ok | 1 | 2.62 | RED |  |
| `parity_blades_live_start` | FAILED | 1 | 61 | 0 | 0 | 40 | 22 | 198 | 198 | ok | 1 | 1.93 | RED |  |
| `parity_bp7` | FAILED | 1 | 47 | 0 | 0 | 35 | 13 | 16 | 47 | ok | 1 | 2.92 | RED |  |
| `parity_card_identity` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 151 | 151 | ok | 0 | 1.53 | GREEN(verified) |  |
| `parity_card_score` | FAILED | 1 | 24 | 0 | 0 | 20 | 5 | 323 | 323 | ok | 1 | 2.73 | RED |  |
| `parity_cce_gaps` | FAILED | 33 | 82 | 0 | 0 | 50 | 33 | 35 | 82 | ok | 33 | 6.51 | RED |  |
| `parity_char2` | FAILED | 1 | 5 | 0 | 0 | 6 | 0 | 128 | 128 | ok | 1 | 2.28 | RED |  |
| `parity_characterization` | FAILED | 1 | 5 | 0 | 0 | 4 | 2 | 0 | 5 | resLines | 1 | 0.93 | RED |  |
| `parity_characterization_extra` | FAILED | 18 | 43 | 0 | 0 | 26 | 18 | 36 | 43 | ok | 18 | 3.36 | RED |  |
| `parity_choice_routing` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 39 | checks | 0 | 1.08 | GREEN(verified) |  |
| `parity_cluster_d` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 55 | 55 | ok | 0 | 1.09 | GREEN(verified) |  |
| `parity_cluster_e` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 60 | 60 | ok | 0 | 0.99 | GREEN(verified) |  |
| `parity_compound_per_card` | FAILED | 0 | 64 | 0 | 0 | 42 | 23 | 24 | 64 | ok | 0 | 4.36 | RED |  |
| `parity_condition_and_effect` | FAILED | 1 | 69 | 0 | 0 | 48 | 22 | 344 | 344 | ok | 1 | 1.87 | RED |  |
| `parity_conditional_gated` | FAILED | 0 | 69 | 0 | 0 | 45 | 25 | 290 | 290 | ok | 0 | 3.17 | RED |  |
| `parity_conditions` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 23 | checks | 0 | 1.08 | GREEN(verified) |  |
| `parity_cost_and_effect` | FAILED | 1 | 46 | 0 | 0 | 33 | 14 | 135 | 135 | ok | 1 | 1.57 | RED |  |
| `parity_cost_compound` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 98 | 98 | ok | 1 | 1.07 | RED |  |
| `parity_cost_discount` | FAILED | 1 | 87 | 10 | 0 | 58 | 30 | 61 | 385 | ok | 1 | 7.65 | RED |  |
| `parity_custom_misc` | FAILED | 1 | 3 | 0 | 0 | 3 | 1 | 79 | 81 | ok | 1 | 1.26 | RED |  |
| `parity_dead_twins` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 47 | 47 | ok | 0 | 1.71 | GREEN(verified) |  |
| `parity_deck_order` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 159 | 159 | ok | 0 | 1.56 | GREEN(verified) |  |
| `parity_deploy_move_missing` | FAILED | 1 | 10 | 0 | 0 | 8 | 3 | 66 | 66 | ok | 1 | 1.19 | RED |  |
| `parity_deploy_multi` | FAILED | 1 | 32 | 0 | 0 | 22 | 11 | 11 | 212 | ok | 1 | 2.45 | RED |  |
| `parity_deploy_recover` | FAILED | 1 | 36 | 0 | 0 | 25 | 12 | 305 | 305 | ok | 1 | 1.99 | RED |  |
| `parity_draw_delta` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 152 | 426 | ok | 0 | 1.03 | GREEN(verified) |  |
| `parity_draw_flat` | FAILED | 1 | 67 | 0 | 0 | 52 | 16 | 556 | 556 | ok | 1 | 2.63 | RED |  |
| `parity_draw_until_count` | FAILED | 1 | 4 | 0 | 0 | 5 | 0 | 65 | 65 | ok | 1 | 1.53 | RED |  |
| `parity_energy_position_delta` | FAILED | 1 | 10 | 0 | 0 | 7 | 4 | 3 | 10 | ok | 1 | 1.65 | RED |  |
| `parity_energy_remainder` | FAILED | 0 | 7 | 0 | 0 | 5 | 3 | 4 | 57 | ok | 0 | 1.29 | RED |  |
| `parity_enums` | FAILED | 1 | 2 | 0 | 0 | 3 | 0 | 0 | 7199 | asserts | 1 | 1.15 | RED | GEN6 |
| `parity_executor_dispatch` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 119 | 119 | ok | 0 | 1.58 | GREEN(verified) |  |
| `parity_foreign_cause` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 38 | 38 | ok | 0 | 1.19 | GREEN(verified) |  |
| `parity_full_round` | FAILED | 1 | 44 | 0 | 0 | 30 | 15 | 19 | 44 | ok | 1 | 2.63 | RED |  |
| `parity_full_round2` | FAILED | 1 | 68 | 0 | 0 | 55 | 14 | 2 | 68 | ok | 1 | 2.83 | RED |  |
| `parity_full_round3` | FAILED | 1 | 17 | 0 | 0 | 13 | 5 | 16 | 17 | ok | 1 | 2.49 | RED |  |
| `parity_fw_cost_cardtype` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 33 | struct | 0 | 1.12 | GREEN(verified) |  |
| `parity_fw_effects` | FAILED | 0 | 2 | 0 | 0 | 2 | 1 | 0 | 25 | struct | 0 | 1.14 | RED |  |
| `parity_fx_fixers` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 34 | 34 | ok | 0 | 0.99 | GREEN(verified) |  |
| `parity_gain_delta` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 15 | 15 | ok | 1 | 1.62 | RED |  |
| `parity_group_names` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 32 | 32 | ok | 0 | 1.19 | GREEN(verified) |  |
| `parity_hearts_constants` | FAILED | 1 | 23 | 0 | 0 | 18 | 6 | 139 | 372 | ok | 1 | 1.34 | RED |  |
| `parity_hearts_live_start` | FAILED | 1 | 81 | 0 | 0 | 65 | 17 | 132 | 196 | ok | 1 | 1.92 | RED |  |
| `parity_int_gaps` | FAILED | 1 | 41 | 0 | 0 | 31 | 11 | 4 | 135 | ok | 1 | 1.76 | RED |  |
| `parity_integration` | FAILED | 1 | 39 | 0 | 0 | 29 | 11 | 135 | 171 | ok | 1 | 1.72 | RED |  |
| `parity_integration_extra` | FAILED | 1 | 64 | 0 | 0 | 44 | 21 | 26 | 64 | ok | 1 | 3.46 | RED |  |
| `parity_integration_rounds` | FAILED | 1 | 25 | 0 | 0 | 20 | 6 | 4 | 25 | ok | 1 | 1.46 | RED |  |
| `parity_jidou` | FAILED | 1 | 47 | 0 | 0 | 31 | 17 | 66 | 66 | ok | 1 | 1.99 | RED |  |
| `parity_jidou_extra` | FAILED | 1 | 30 | 0 | 0 | 22 | 9 | 91 | 91 | ok | 1 | 2.12 | RED |  |
| `parity_jidou_life` | FAILED | 1 | 26 | 0 | 0 | 19 | 8 | 217 | 217 | ok | 1 | 1.73 | RED |  |
| `parity_jidou_move_yell` | FAILED | 1 | 86 | 0 | 0 | 51 | 36 | 52 | 324 | ok | 1 | 7.33 | RED |  |
| `parity_jidou_state` | FAILED | 1 | 21 | 0 | 0 | 14 | 8 | 151 | 151 | ok | 1 | 3.05 | RED |  |
| `parity_jidou_watch` | FAILED | 1 | 73 | 0 | 0 | 53 | 21 | 283 | 283 | ok | 1 | 5.16 | RED |  |
| `parity_jidou_watch2` | FAILED | 1 | 72 | 0 | 0 | 50 | 23 | 311 | 311 | ok | 1 | 2.62 | RED | GEN6 |
| `parity_jidou_watch_clusters` | FAILED | 1 | 61 | 0 | 0 | 47 | 15 | 390 | 390 | ok | 1 | 3.04 | RED |  |
| `parity_live_total` | FAILED | 1 | 50 | 0 | 0 | 33 | 18 | 185 | 185 | ok | 1 | 2.91 | RED |  |
| `parity_live_total2` | FAILED | 1 | 12 | 0 | 0 | 8 | 5 | 306 | 306 | ok | 1 | 2.59 | RED |  |
| `parity_look_and_filter` | FAILED | 15 | 37 | 0 | 0 | 23 | 15 | 50 | 50 | ok | 15 | 3.82 | RED |  |
| `parity_look_per_card` | FAILED | 21 | 65 | 0 | 0 | 45 | 21 | 28 | 65 | ok | 21 | 3.44 | RED |  |
| `parity_look_reveal` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 153 | 153 | ok | 1 | 1.68 | RED |  |
| `parity_mill_deck` | FAILED | 1 | 81 | 0 | 0 | 82 | 0 | 140 | 140 | ok | 1 | 5.12 | RED |  |
| `parity_movement_moves` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 12 | 12 | ok | 0 | 0.89 | GREEN(verified) |  |
| `parity_need_heart_live_flow` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 23 | 23 | ok | 0 | 0.93 | GREEN(verified) |  |
| `parity_orphan_producers` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 38 | checks | 0 | 0.89 | GREEN(verified) |  |
| `parity_parser_e2e_extra` | FAILED | 6 | 16 | 0 | 0 | 11 | 6 | 11 | 16 | ok | 6 | 2.03 | RED |  |
| `parity_per_card` | FAILED | 5 | 20 | 0 | 0 | 16 | 5 | 16 | 20 | ok | 5 | 1.92 | RED |  |
| `parity_per_card2` | FAILED | 1 | 17 | 0 | 0 | 11 | 7 | 18 | 18 | ok | 1 | 2.67 | RED |  |
| `parity_per_card3` | FAILED | 9 | 22 | 0 | 0 | 14 | 9 | 12 | 22 | ok | 9 | 1.99 | RED |  |
| `parity_q68_cannot_live` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 37 | checks | 0 | 1.21 | GREEN(verified) |  |
| `parity_queue_resume` | FAILED | 1 | 5 | 0 | 0 | 4 | 2 | 143 | 143 | ok | 1 | 1.65 | RED |  |
| `parity_realcard_walkthrough` | FAILED | 1 | 7 | 0 | 0 | 6 | 2 | 6 | 7 | ok | 1 | 2.12 | RED |  |
| `parity_recover_hand_a` | FAILED | 1 | 26 | 0 | 0 | 27 | 0 | 15 | 26 | ok | 1 | 1.99 | RED |  |
| `parity_recover_live` | FAILED | 19 | 52 | 0 | 0 | 34 | 19 | 45 | 52 | ok | 19 | 3.99 | RED |  |
| `parity_recover_q79` | FAILED | 1 | 18 | 0 | 0 | 19 | 0 | 16 | 18 | ok | 1 | 2.02 | RED |  |
| `parity_recover_score_state_delta` | FAILED | 0 | 46 | 0 | 0 | 29 | 18 | 147 | 147 | ok | 0 | 1.88 | RED |  |
| `parity_recover_to_hand` | FAILED | 1 | 15 | 0 | 0 | 11 | 5 | 103 | 103 | ok | 1 | 2.13 | RED |  |
| `parity_required_hearts` | FAILED | 1 | 51 | 26 | 0 | 41 | 11 | 253 | 253 | ok | 1 | 7.02 | RED | META |
| `parity_resolver` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 99 | 99 | ok | 1 | 1.16 | RED |  |
| `parity_restriction` | FAILED | 1 | 61 | 0 | 0 | 62 | 0 | 107 | 107 | ok | 1 | 2.68 | RED |  |
| `parity_restriction_invalidate` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 6 | 6 | ok | 0 | 1.62 | GREEN(verified) |  |
| `parity_reveal_extra` | FAILED | 1 | 23 | 0 | 0 | 16 | 8 | 445 | 445 | ok | 1 | 1.99 | RED |  |
| `parity_reveal_provenance` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 25 | 25 | ok | 0 | 0.88 | GREEN(verified) |  |
| `parity_rules` | FAILED | 1 | 21 | 0 | 0 | 12 | 10 | 157 | 157 | ok | 1 | 1.31 | RED |  |
| `parity_rules_conditions` | FAILED | 0 | 6 | 0 | 0 | 5 | 2 | 122 | 122 | ok | 0 | 1.53 | RED |  |
| `parity_rules_conditions2` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 147 | 147 | ok | 0 | 1.90 | GREEN(verified) |  |
| `parity_rules_extra` | FAILED | 2 | 6 | 0 | 0 | 5 | 2 | 14 | 14 | ok | 2 | 1.47 | RED |  |
| `parity_rules_phases` | FAILED | 0 | 28 | 0 | 0 | 19 | 10 | 205 | 205 | ok | 0 | 2.95 | RED |  |
| `parity_score_effects` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 107 | 108 | ok | 1 | 1.14 | RED |  |
| `parity_score_per_card2` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 139 | 139 | ok | 1 | 1.53 | RED |  |
| `parity_search_reorder` | FAILED | 1 | 97 | 0 | 0 | 98 | 0 | 286 | 286 | ok | 1 | 6.00 | RED |  |
| `parity_sequential` | FAILED | 1 | 142 | 0 | 0 | 114 | 29 | 23 | 142 | ok | 1 | 7.17 | RED |  |
| `parity_small_clusters` | FAILED | 0 | 101 | 0 | 0 | 63 | 39 | 849 | 849 | ok | 0 | 10.24 | RED |  |
| `parity_state_effects2` | FAILED | 1 | 6 | 0 | 0 | 4 | 3 | 99 | 99 | ok | 1 | 2.04 | RED |  |
| `parity_state_family` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 38 | 38 | ok | 1 | 1.04 | RED |  |
| `parity_state_per_card` | FAILED | 33 | 86 | 8 | 0 | 54 | 33 | 49 | 86 | ok | 33 | 6.78 | RED |  |
| `parity_stats_pipeline` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 92 | 92 | ok | 0 | 1.09 | GREEN(verified) |  |
| `parity_support` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 12 | 12 | ok | 1 | 1.38 | RED |  |
| `parity_t6_action_setup` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 48 | struct | 0 | 1.17 | GREEN(verified) |  |
| `parity_t7_condition_decoder` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | ok | 0 | 0.95 | GREEN(verified) |  |
| `parity_target_player` | FAILED | 24 | 77 | 0 | 0 | 54 | 24 | 22 | 77 | ok | 24 | 3.21 | RED |  |
| `parity_trigger_paths` | FAILED | 1 | 53 | 0 | 0 | 33 | 21 | 301 | 301 | ok | 1 | 2.51 | RED |  |
| `parity_under_member` | FAILED | 0 | 81 | 0 | 0 | 55 | 27 | 56 | 431 | ok | 0 | 7.27 | RED |  |
| `parity_wait_activation` | FAILED | 1 | 66 | 0 | 0 | 40 | 27 | 251 | 251 | ok | 1 | 1.98 | RED |  |
| `parity_wave1a` | FAILED | 1 | 38 | 0 | 0 | 26 | 13 | 8 | 38 | ok | 1 | 2.41 | RED |  |
| `parity_wave1b` | FAILED | 1 | 29 | 0 | 0 | 20 | 10 | 21 | 29 | ok | 1 | 2.91 | RED |  |
| `parity_wave1c` | FAILED | 1 | 55 | 0 | 0 | 43 | 13 | 7 | 55 | ok | 1 | 2.07 | RED | GEN6 |
| `parity_wire_token_bytes` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 35 | 35 | ok | 0 | 0.83 | GREEN(verified) |  |
| `parity_x_class` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 5 | 5 | ok | 0 | 1.06 | GREEN(verified) |  |
| `parity_yell` | FAILED | 1 | 53 | 0 | 0 | 40 | 14 | 1 | 106 | ok | 1 | 1.69 | RED |  |
| `parity_zone_source` | FAILED | 0 | 73 | 0 | 0 | 45 | 29 | 441 | 441 | ok | 0 | 5.08 | RED |  |
| `parity_zones_position` | FAILED | 1 | 48 | 0 | 0 | 37 | 12 | 140 | 176 | ok | 1 | 1.23 | RED |  |
| `performance_phase_rules` | FAILED | 1 | 3 | 0 | 0 | 3 | 1 | 36 | 36 | ok | 1 | 1.05 | RED |  |
| `phase_machine_rules` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 34 | checks | 0 | 0.99 | GREEN(verified) |  |
| `ported_generated` | FAILED | 1 | 5 | 0 | 0 | 4 | 2 | 78 | 78 | ok | 1 | 1.14 | RED |  |
| `position_and_source` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 17 | 44 | ok | 0 | 1.55 | GREEN(verified) |  |
| `restriction_mechanics` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 18 | 18 | ok | 0 | 0.87 | GREEN(verified) |  |
| `state_effects` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 35 | 35 | ok | 0 | 1.56 | GREEN(verified) |  |
| `surplus_heart` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 12 | 12 | ok | 1 | 1.04 | RED |  |
| `survey_probe` | FAILED | 1 | 2 | 0 | 0 | 2 | 1 | 3 | 3 | ok | 1 | 1.30 | RED |  |
| `target_selection` | FAILED | 1 | 5 | 0 | 0 | 4 | 2 | 31 | 31 | ok | 1 | 0.96 | RED |  |
| `tp_activation_offer` | FAILED | 1 | 5 | 0 | 0 | 4 | 2 | 3 | 5 | ok | 1 | 0.89 | RED |  |
| `tp_joint_card_live_start` | FAILED | 1 | 29 | 0 | 0 | 22 | 8 | 33 | 33 | ok | 1 | 1.04 | RED |  |
| `tp_turn_limited_activation` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 15 | 15 | ok | 0 | 0.95 | GREEN(verified) |  |
| `tp_zone_change_gate` | FAILED | 1 | 18 | 0 | 0 | 14 | 5 | 8 | 18 | ok | 1 | 0.99 | RED |  |
| `trigger_paths` | FAILED | 1 | 18 | 0 | 0 | 13 | 6 | 128 | 128 | ok | 1 | 1.85 | RED |  |
| `vm_decode` | FAILED | 1 | 1 | 0 | 0 | 2 | 0 | 41 | 41 | ok | 1 | 1.36 | RED | GEN6 |
| `web_server_mods_init` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 32 | asserts | 0 | 1.13 | GREEN(verified) |  |
| `zone_filter_parity` | PASSED | 0 | 0 | 0 | 0 | 0 | 0 | 45 | 45 | ok | 0 | 1.31 | GREEN(verified) |  |

## What is NOT measured here

* The **Rust** side was not executed. `cargo` was never run, by instruction.
  Every Rust statement in `PARITY_VERDICT.md` is a **source reading**, not a
  measurement.
* The **Rust tree moved during this session.** OneDrive materialised two
  tracked `mod.rs` files between two consecutive checks minutes apart
  (`tests/test_modules/effects/choice/target_player/mod.rs`,
  `tests/test_modules/rules/targeting/mod.rs`). The **C** snapshot was
  hash-verified stable throughout, so the numbers above are safe; any Rust
  file-count claim is a snapshot of a moving target.

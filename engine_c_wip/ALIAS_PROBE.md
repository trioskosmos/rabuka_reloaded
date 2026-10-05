# ALIAS_PROBE.md — the alias-class probe for the seven parked continuation slots

Date: 2026-10-05. Scope: `engine_c_wip/`. No Rust was read except for the
citations already in the tree's own notes. **No sanitizer ran** — gcc 15.3.0
(MSYS2/PE) in this tree has no asan, no ubsan, no valgrind and no gdb.

## What is here

| artifact | role |
|---|---|
| `tools/alias_probe.c` | the probe. Wraps `rb_effect_free`, `rb_free_ability`, `rb_has_pending_choice` at LINK time and tests all seven slots against every freed subtree while a prompt is open |
| `tools/alias_probe.sh` | the runner. Builds, runs, counts, and refuses to report a verdict it did not earn |
| `tests/test_alias_ownership.c` | the ownership-topology regression test (16 assertions) |
| `src/ability/effects/look.c` | two READ-ONLY accessors, `rb_look_slot_pending_select` / `rb_look_slot_reprompt_followup`, so the two `static` slots became measurable |
| `src/core/game_state_abilities.c` | one `#ifdef RB_ALIAS_PROBE_NEGATIVE_CONTROL` block, never defined by the build, that restores the pre-fix body for the control |

Run it:

```
bash tools/alias_probe.sh <outdir> <suite> [ ... ]              # measure
bash tools/alias_probe.sh --negative-control <outdir> <suite>   # prove it fires
```

## Why a link-time wrap and not a source edit

`--wrap` redirects every **undefined** reference, which is every call from
another translation unit. It touches no engine source, so the probe measures the
tree as it is. The two intra-TU recursions (`alloc.c`'s own
`rb_effect_free(child[i])` and `vm.c`'s static `effect_free`) are deliberately
*not* redirected — the probe walks the whole subtree itself before delegating, so
a partial interception would under-report rather than over-report.
`rb_free_ability`'s release path is `vm.c`'s **static** `effect_free`, which no
wrapper can name; intercepting `rb_free_ability` and enumerating `a->cost` and
`a->effect` covers exactly the node set that is about to be released.

The GameState is not a parameter of either free function, so the probe also
wraps `rb_has_pending_choice(g)` — the most-called predicate in the prompt
protocol (~190 sites in 13 files) — purely to cache the live `g`. The cached
pointer is validated by reading `g->queue.has_pending` directly, so the
"is a prompt open" half never recurses back through the wrapper.

## Why this design and not the address-set design

The alternative — record every freed address in a set, then test the slots
against the set — reports a hit whenever a slot holds an address that was freed
at *some* point in the run, including after `malloc` recycled that address for a
live node. This design asks a question with no time dimension: **at the instant
this tree is released, does one of the seven slots point INTO it?** A recycled
address cannot answer that. There are no false positives by construction.

## ⛔ Four measurement traps this runner closes, all of which fired for real

1. **`make` reports a link failure as `Error 1`, not `error:`.** The runner never
   calls `make`. It links with `cc`, checks the exit status, checks the `.exe`
   exists, and prints `BUILD-FAIL ... (NOT counted as clean)`. **This fired**: a
   peer's `make` was mid-write, 18 links failed, and a naive sweep would have
   reported a clean zero.
2. **A suite that ran 0 tests prints zero `FAIL`s.** Every line carries
   `ok=`, `FAIL=`, `alias_hits=`, `probe=LINKED|MISSING` and
   `ran=yes|NO-CASES`. A suite with `ok=0 FAIL=0` is visibly not a pass.
3. **A sweep that measured nothing is not a clean sweep.** `VERDICT ZERO` is
   refused when `probed == 0`; `VERDICT PARTIAL` when any suite was unusable.
   This is trap 2 one level up, and **it fired** in the same run as trap 1.
4. **The reporter can run in the wrong process.** `parity_cce_gaps:3551` and
   `parity_required_hearts` fork a child per case, and a child leaving through
   `_exit()` never reaches a destructor — so the parent's exit counter reads
   **0 hits while the log is full of them**. The authoritative number is
   therefore the count of `ALIAS_HIT` lines; the exit line is only a lower bound,
   and the runner prints `SUMMARY-DISAGREES(...)` whenever the two differ rather
   than reporting the flattering one.

`^FAIL` is counted case-sensitively, stdout and stderr together. **Exit code is
never used to decide green** — in the negative-control arm the process dies part
way through and its rc is worthless; both arms are judged on the suite's own
`ALL ALIAS OWNERSHIP CHECKS PASSED` marker.

There is also a **staleness gate**: the runner refuses to run if any `.o` is
older than its `.c` or than `include/rabuka.h`. This fired twice, correctly, on
peers' edits.

## The negative control

```
bash tools/alias_probe.sh --negative-control <outdir> parity_cce_gaps parity_int_gaps
```

Reverses the `pca_resolve_entry` keep-alive transfer in
`src/core/game_state_abilities.c` (`-DRB_ALIAS_PROBE_NEGATIVE_CONTROL`,
recompiling that TU fresh) and restores the pre-2026-10-04 body: free the
ability on the pass that opened the prompt.

**Measured: 22 hits** (parity_cce_gaps 21, parity_int_gaps 1) versus **0** with
the fix in place over the same suites. Exit 1, as required.

## Results on the landed tree (2026-10-05)

18 suites, `suites_probed=18 unusable=0`, **total_alias_hits = 0**. Every suite's
`FAIL` count matches the baseline in `UAF_LIFETIME_MEASUREMENTS.md` exactly
(parity_cce_gaps 82, parity_queue_resume 5, parity_under_member 81,
parity_look_per_card 65, parity_look_and_filter 37, parity_int_gaps 41,
parity_wave1b 29, parity_wave1a 38, parity_wave1c 55, parity_full_round 44,
parity_full_round2 68, parity_full_round3 17, parity_jidou_watch 73,
parity_jidou_watch_clusters 61, parity_jidou_move_yell 86, parity_live_total
50, parity_recover_live 52, parity_zones_position 48).

## ⛔ The conversion that was measured, rejected, and reverted

`g_look_pending_select` was converted from an alias to an owning
`rb_effect_deep_clone`. It **did not work, and the probe is why that is a fact
rather than an opinion.**

`look_emit_pool_choice`'s body is, in this order:

```
795:  g->queue.resume_eff = (AbilityEffect *)sel;
797:  look_park_select((AbilityEffect *)sel);
```

and on the multi-pick re-prompt path `sel` **is** the value `look_park_select`
is replacing. So a park that clones and then releases the old value releases the
node `resume_eff` published one line earlier.

Same-snapshot A/B — `tools/alias_probe.sh --prechange` versus the converted arm,
**one tree state, no rebuild between** (this matters: the tree cannot be
snapshotted across a rebuild, and a sweep taken 40 minutes earlier is not
comparable to one taken now):

| arm | alias hits | FAIL look_and_filter / look_per_card / full_round |
|---|---|---|
| pre-change (alias) | **0** | 37 / 65 / 44 |
| converted (own + release) | **32**, all `queue.resume_eff`, `slot_value == freed_root` | **41 / 73 / 44** |

The owning clone did not create a defect. It **destroyed** the node `resume_eff`
was aliasing — turning a two-slot alias into a live use-after-free — and cost 10
previously-passing assertions. The conversion was reverted; `look.c` carries the
measurement and the reason at `look_park_select`.

`tests/test_alias_ownership.c` is the artifact that attempt produced. It pins the
sharing property instead, so the next attempt starts from the measured fact:

* a fresh offer: slot and `resume_eff` are one node;
* **the re-park: after `look_emit_pool_choice` runs twice, slot and `resume_eff`
  are again one node** — this is the identity that breaks the conversion;
* `look_and_select`: `g_look_reprompt_followup` and `resume_after_look` are one
  node, assigned from the same expression on adjacent lines.

## What this means for the remaining six slots

The slots are **not independent owners**, so the conversion has to be joint.
`resume_eff`, `resume_after_look` and the look.c statics must become owning
copies in one pass, with `resume_eff` taking ownership of a clone that the
others either share or clone from. Converting one at a time is precisely the
"third state with neither the safety of the original nor the correctness of the
target" — and it is now a measured failure, not a hypothesis.

`target_selection_eff` is worth naming separately: **it has no writer anywhere in
the tree.** The only assignments are `= NULL` (choice.c:5538) and a local read
(choice.c:5537). `misc.c:527`'s comment names a producer that does not exist. It
is a dead field and cannot alias anything; the probe tests it anyway, so if a
writer ever appears the audit will say so.
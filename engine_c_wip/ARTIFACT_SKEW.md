# ARTIFACT_SKEW — the Rust and C generations have diverged, and the C is behind

Measured at **HEAD `5ce311e6`** (dirty working tree, as found). `cargo` was **never** run; every number is
recomputed from bytes on disk. The one supported pipeline, `engine_c_wip/tools/regen_all.sh`, was run in an
**isolated snapshot only** (`%TEMP%/kilo/skew_snap`, since deleted), never in the live tree.

## 1. Canonical generation — **Rust (`cards/build/`), by 160 bytes**

`cards/abilities.json` is the source of truth. The pipeline reproduces the live Rust bytes **exactly**
(`32fa1a69…` out == in), emits `TABLE_LEN=726671`, `bytecode_blob.c` 98 782 B (`49d796fd…`), `gen_data.c`
132 608 B (`a28c1456…`), then `verify_artifacts.py` exits 0. **Layer 2 needs no regeneration; only layer 3 is
behind.** `RBKA_NUM_ABILITIES` is 985 on both sides — it was never the tell.
| artefact | bytes | sha256 | verdict |
|---|---|---|---|
| `cards/build/abilities_strings.bin` | **726 671** | `32fa1a69c0ba9bc2f69a7dfd046395f6b021840fbc484ce70f4a40865734d652` | **CANONICAL** |
| `cards/build/abilities.bin` | 98 790 | `1e0720bf2989273abab38ed25d7498aa841b6270e1ba7cba054f8009786cea15` | **CANONICAL** |
| `engine_c_wip/src/abilities_strings.bin` | 726 831 | `89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344` | **STALE** |
| `engine_c_wip/src/core/generated/gen_data.c` | 134 073 | `9bf115ffdbcc41a6931f4fd2227230e2d04cdc4a9ed673fb2326f657d178f309` | **STALE** |
| `engine_c_wip/src/core/generated/bytecode_blob.c` | 273 059 | `a9d0fdbab49d4afadf91ee401c34c20be4e6b4046dad50bd779b145efe244366` | **STALE** |

`tools/verify_artifacts.py` is **RED on the live tree today** (exit 1): 726 671 vs a recorded 726 831,
5 533 of 6 032 string digests mismatched, whole-table FNV `3985478542` vs `876896371`, and `gen_data.c`'s
`STRINGS_OFFSETS`/`OFFSET_DELTAS` disagreeing with `abilities_gen.rs` at *equal length*. Nobody ran it.

## 2. The C is self-consistent — and that is the problem

Both generations pass every internal check, so **no C-side test can see this**: `RBKA_BYTECODE_LEN ==
sum(OFFSET_DELTAS)` (98 737 ✔ / 98 782 ✔); `RBKA_STRINGS_TABLE_LEN` == blob size (726 831 ✔ / 726 671 ✔);
whole-table FNV-1a recomputed from disk matches (`876896371` ✔ / `3985478542` ✔); `RBKA_NUM_STRING_OFFSETS`
== n+1; offset sentinel == table len. **`tests/test_artifacts_consistent.c` cannot catch this** — line 84
loads `rb_load("src")` **first**, and `src/abilities_strings.bin` holds the *stale* 726 831 blob matching the
*stale* `gen_data.c`, so it validates a closed, self-consistent, **two-generations-old island** and reports
`all digests match`. `verify_artifacts.py` checks `cards/build/` and *does* catch it; the gap is that nothing
runs it, and `make data` refuses to install the correct blob (§6).
⚠️ **None of the eight named suites could be run here.** The only compiler on this machine is devkitPro's
MSYS2 gcc, target `x86_64-pc-cygwin`, with no libc sysroot — `stdint.h` is unresolvable, so `make` dies at
`src/ability/enums.o`. The 142 `tests/*.log` files are dated 2026-10-02 and belong to the old generation.
**No suite result is claimed anywhere in this file.**

## 3. The 15 abilities whose LENGTH differs

969 of 985 slices differ in **bytes**; only these 15 differ in **length**. Bytes are C → Rust; print
variants are the trailing group of the identifier. All 26 identifiers are **exact `cards.json` keys**
(1 650/1 650 ability identifiers are), so the `rb_find_card_by_no` substring-fallback trap does not apply.
```
785 PL!N-bp5-013-N              N     74 -> 84    773 PL!N-bp4-018-N          N      144 -> 146
159 PL!SP-bp7-005-{P,P＋,R＋,SEC}      89 -> 85    325 PL!-pb2-012-{P＋,R}      P＋/R   159 -> 149
934 PL!SP-bp7-016-N              N     89 -> 85    643 PL!-pb2-021-N           N      156 -> 158
302 PL!-pb1-015-{P＋,R}        P＋/R   95 -> 97    801 PL!N-bp7-022-N          N      184 -> 188
412 PL!N-bp7-001-{P,R}         P/R   103 -> 99    812 PL!N-pb1-037-L          L      198 -> 204
912 PL!SP-bp4-016-N             N     88 -> 105    805 PL!N-bp7-028-L          L      238 -> 236
306 PL!-pb1-028-{L,SRL}        L/SRL 124 -> 126    151 PL!SP-bp5-004-{AR,P,R＋,SEC}   138 -> 134
327 PL!-pb2-014-{P＋,R}        P＋/R  113 -> 141  <-- the big one
```

## 4. Cross-engine check — layer 1 LANDED, layer 2 REPORTED

**Landed:** `cards/build/generation_manifest.json` gained a top-level `c_artifacts` block recording all three
committed C artefacts (bytes + full sha256) plus `bytecode_len 98737`, `strings_table_len 726831`,
`strings_table_fnv1a32 876896371`, `generation: "stale"`. Valid JSON; `verify_artifacts.py` still runs. ⚠️
**`cards/build_lib.py:115-136` rebuilds that manifest from scratch and does not know about `c_artifacts`, so
the next `compile_abilities.py` run DELETES the key** — a reader must treat an absent `c_artifacts` as
*cannot cross-check*, never *agrees*. Surviving it needs a passthrough in `build_lib.py`: adjacent to my
sandbox, so reported, not done.
**⚠️ NOT LANDED: `cards/build.rs` DOES NOT EXIST.** The crate's only build script is `engine/build.rs`, which
already reads `../cards/build/generation_manifest.json`, already holds `sha256_hex`, and already emits
`cargo:warning=` for this class of staleness. Per the brief's own instruction ("if the check belongs in
`engine/build.rs` instead, report it rather than doing it") I did not edit it — and **`cargo` is off limits, so
I could not have compiled it anyway.** The reviewed patch is in
**`ARTIFACT_SKEW_buildrs_patch.md`** (insert after `let _ = stale;`, `engine/build.rs:325`).
**PROOF IT FIRES** — the identical comparison, run against real bytes. (It proves the *comparison* fires and
the numbers disagree; it does **not** prove the Rust compiles — `cargo` was never run.)
| case | warnings | required | |
|---|---|---|---|
| live tree as found | **2** — `RBKA_BYTECODE_LEN 98737 != 98782`; `TABLE_LEN 726831 != 726671` | > 0 | **PASS** |
| after `regen_all.sh` | **0** | 0 | **PASS** |
| C `TABLE_LEN` 726671→726672 (1 byte) | **1** | > 0 | **PASS** |
| C last `RBKA_BYTECODE` byte dropped (98 782→98 781) | **1** | > 0 | **PASS** |
| C `NUM_STRING_OFFSETS` 6033→6032 | **1** | > 0 | **PASS** |

## 5. What must be re-pointed in the SAME change as any regen

**29 Rust files** = 28 `*_test.rs` + 1 shared helper, each naming a card from the 15. Paths relative to
`engine/tests/test_modules/`; slices in parentheses. This corrects `RUST_ORACLE_REPAIR.md:147-148` ("30
distinct files (29 test files + 1 helper)") — it double-counts by one; **29 is the total.**
```
characterization/card_type_filter_test.rs (805) · effects/auto/pb2_self_wait_or_cost_paid_activates_and_gains_blade_pl_pb2_021_n_test.rs (302,643) · effects/conditional/state_gated/deployed_member_blade_heart_wait_test.rs (773)
effects/cost/additional_cost_choice/pb2_kidou_additional_cost_discard_two_or_wait_two_printemps_pl_pb2_012_p_plus_test.rs (325) · effects/cost/play_cost/pb2_optional_two_distinct_printemps_wait_costs_two_less_pl_pb2_012_p_plus_test.rs (325) · effects/energy/under_member/energy_and_member_under_test.rs (643,785)
effects/gain/blades/constants/pb2_score_sum_per_five_blade_pl_pb2_030_n_test.rs (306) · effects/gain/blades/per_card/bp7_parser_gap_cards_test.rs (412) · effects/gain/hearts/constants/success_zone_subunit_constant_heart_test.rs (306) · effects/gain/hearts/live_start/live_start_discard_shuffle_group_members_heart01_test.rs (805)
effects/move_cards/pb2_reveal_lily_white_live_and_swap_success_zone_pl_pb2_014_p_plus_test.rs (327) · effects/recover/per_card/live_start_activate_three_printemps_wait_members_q178_q179_test.rs (306) · effects/state/wait_activation/center_optional_unit_wait_opponent_wait_immunity_test.rs (302) · effects/state/wait_activation/maki_optional_self_wait_cost_opponent_own_wait_test.rs (302)
integration/per_card/q203_q204_q218_eternalize_cara_chika_rulings_test.rs (812) · jidou/energy_watch/debut_and_energy_return_watchers_stay_on_stage_test.rs (159) · jidou/energy_watch/debut_or_own_energy_return_place_waited_energy_test.rs (159) · jidou/energy_watch/energy_placed_by_own_effect_gain_blade_test.rs (159,934)
jidou/energy_watch/energy_placement_blade_excludes_area_move_and_under_member_test.rs (159) · jidou/energy_watch/energy_placement_watchers_across_debut_and_position_change_test.rs (912) · jidou/energy_watch/own_effect_energy_placed_blade_upper_bound_test.rs (159) · jidou/energy_watch/own_effect_energy_placed_each_time_gain_heart06_test.rs (151,912)
jidou/energy_watch/real_card_effect_self_move_or_energy_placed_draw_heart02_test.rs (151) · jidou/state_watch/live_phase_group_wait_discard_reactivate_test.rs (801) · jidou/state_watch/own_effect_wait_cheap_opponent_draw_one_q177_test.rs (302) · rules/trigger_paths/jidou_paired_ability_combination_test.rs (159)
rules/trigger_paths/sibling_abilities_on_one_card_interact_test.rs (159) · rules/trigger_paths/trigger_scope_test.rs (159,912) · support/baton_swap_auto_helpers.rs (412)
```
**20 C files** (`engine_c_wip/tests/`) touch the same 15: `test_audit_conditions.c` (801) ·
`test_audit_zones.c` (159) · `test_decode_bytecode_shape.c` (325,805) · `test_describe_golden.c` (325,805)
· `test_parity_bp7.c` (412) · `test_parity_conditional_gated.c` (773) · `test_parity_deploy_recover.c` (306)
· `test_parity_hearts_constants.c` (306) · `test_parity_hearts_live_start.c` (805) ·
`test_parity_integration_extra.c` (812) · `test_parity_jidou.c` (412) · `test_parity_jidou_extra.c` (159,934)
· `test_parity_jidou_state.c` (302,801) · `test_parity_jidou_watch2.c` (4 ids) ·
`test_parity_orphan_producers.c` · `test_parity_resolver.c` · `test_parity_trigger_paths.c` (2 ids) ·
`test_parity_under_member.c` · `test_parity_wait_activation.c` · `test_parity_wave1a.c`. At the wider
969-slice scope it is **846 Rust** and **155 C** files — so re-pointing must be scoped to the 15, or
expectations regenerated wholesale.

## 6. `guard_data_blob.sh`'s pin is STALE — proven, and it fails safe

`Makefile:129-130` pins `STRINGS_BYTES := 726831` / `STRINGS_SHA256 := 89204df0bbde08a0` while `STRINGS_BIN`
points at the canonical `../cards/build/abilities_strings.bin`:
```
guard --install ../cards/build/abilities_strings.bin … 726831 89204df0bbde08a0
  → "the source blob is 726671 bytes, not the canonical 726831"   exit 1   REFUSED
guard --install ../cards/build/abilities_strings.bin … 726671 32fa1a69c0ba9bc2
  → "installed"                                                       exit 0   ACCEPTED
```
So `make data` — and therefore `make t` and `make run-all-tests`, which all depend on `data` (`Makefile:426`,
`:442`) — **currently hard-fails, refusing the correct blob.** It fails *safe*: the guard cannot install a
wrong blob, it merely blocks the right one. **The pin must become 726 671 / `32fa1a69c0ba9bc2`; do not
change it ahead of the layer-3 regen** (hazard below). `tools/sweep_daemon.ps1` and `tools/queue_runner.ps1`
exist (in `tools/`, not `engine_c_wip/tools/`) and pin **no** blob size or digest; `Makefile:123`'s comment
(`726831 … / 98737 …`) is stale too.
⚠️ **`--install` DESTROYS the good blob.** Correcting the pin alone makes `make data` overwrite the 726 831
blob with the 726 671 one *while `gen_data.c` still holds 726 831 offsets* — the exact 2026-09-30 byte
splice, manufactured by the fix. Order must be **`gen_data.c` + `bytecode_blob.c` +
`src/abilities_strings.bin` + the pin, one commit.** (I hit this during a probe and restored the blob; it is
back at `89204df0…`, git-clean.) The other hazard: `c_artifacts` will not survive the next
`compile_abilities.py` (§4).

## 7. Required order — do not reorder

1. `bash engine_c_wip/tools/regen_all.sh` in one ordered run (layer 2 is already canonical, so a no-op there;
   layer 3 catches up) → `verify_artifacts.py` exit 0.
2. In the **same commit**: regenerated `gen_data.c` + `bytecode_blob.c` + the `Makefile:129-130` pin at
   726 671 / `32fa1a69c0ba9bc2`, + the 29 Rust and 20 C expectations re-pointed.
3. Only then enable `bytecode_deep_compare_test` (`RUST_ORACLE_REPAIR.md` §2).
**Stale prose that will now mislead.** `src/ability/describe.c:46-51` asserts "Rust reads the STALE 726649-byte
string blob while the C reads the canonical 726831-byte one. Do NOT 'fix' this file for ability 325."
**Exactly backwards**, and 726 649 is now a third value. `RUST_ORACLE_REPAIR.md` §6 (`engine/build.rs:325 let
_ = stale;`) remains open and is the right home for layer 2.

# §8 — RESOLVED 2026-10-04, measured at HEAD `5ce311e6` (dirty tree, as found)

§1–§7 above are the **pre-fix** record, left unedited as the audit trail. §7's required order was executed; `cargo` was
**never** run; every number below is recomputed from bytes on disk or from a measured build.

## 8.1 Decision: REGENERATED — and the deciding fact was not the skew

The brief left "regenerate or leave stale" open. The tiebreaker: **`make data` was hard-FAILING at `5ce311e6`**, and
`make t` (`Makefile:426`) and `make run-all-tests` (`:442`) both depend on `data`. The pin was 726 831/`89204df0`
against a canonical 726 671, so the guard refused the *correct* blob and the **entire C corpus was unrunnable** —
0 `.o` files, 142 stale logs from 2026-10-02. "Leave it stale" was not the low-risk option; it meant leaving a bricked
tree plus a lying port. So layer 3 was regenerated in ONE ordered run, with the pin moved in the same change.

Also refuted by measurement: the peer's claim that this machine has no usable compiler. The only compiler is devkitPro's
MSYS2 gcc 15.3.0 (`x86_64-pc-cygwin`) and it compiles `stdint.h` and the whole tree cleanly — `make all` succeeds with
only the 3 known pinned `-Wrestrict` diagnostics (`data.c:572/600/607`). **All 165 suites were run before and after.**

## 8.2 What changed, and what provably did not

| artefact | before | after | verdict |
|---|---|---|---|
| `cards/build/abilities_strings.bin` | 726 671 `32fa1a69…` | 726 671 `32fa1a69…` | **byte-identical** — layer 2 was already canonical |
| `cards/build/abilities.bin` | 98 790 `1e0720bf…` | 98 790 `1e0720bf…` | byte-identical |
| `cards/build/abilities_gen.rs` | 88 762 `4d29e947…` | 88 762 `4d29e947…` | byte-identical |
| `cards/build/generation_manifest.json` | 2 569 `e12aea10…` | 2 569 `e12aea10…` | byte-identical (incl. `c_artifacts`) |
| `engine_c_wip/src/abilities_strings.bin` | 726 831 `89204df0…` | **726 671 `32fa1a69…`** | installed by the guard, REPAIR branch, loudly |
| `gen_data.c` | 134 073 `9bf115ff…` | **132 608 `a28c1456…`** | regenerated |
| `bytecode_blob.c` | 273 059 `a9d0fdbab49d4afa…` | **267 001 `49d796fd…`** | regenerated; `gen_data.h` 495 `f670b2b3…` → **484 `ffd72c3b…`** |

Layer 2 reproducing byte-identically is the strongest available evidence the regen did not silently perturb Rust.
`verify_artifacts.py` now exits **0** (was 1: six problems, 5 533/6 032 digests mismatched). `RBKA_NUM_ABILITIES` 985,
`RBKA_NUM_STRING_OFFSETS` **6033**, interned 6 032, pairs 2 116 — all unchanged. Whole-table FNV-1a 876 896 371 → **3 985 478 542**.

**`sizeof(GameState)` did NOT move: 1 090 264 B, delta +0**, measured with a throwaway `sizeof` TU, not estimated.
`RBKA_NUM_STRING_OFFSETS` was expected to change and did not, and `STRINGS_OFFSETS` is an `extern` pointer in
`rabuka.h:408`, not a count-sized array. Therefore `RB_GAMESTATE_BYTES_PINNED` and `Makefile:3` are **untouched** and
§A4 stays green. **Order was load-bearing and went regen → pin → `make data`**: the guard then took its REPAIR branch
("it held 726 831 … being replaced by the canonical 726 671") — the safe direction, because layer 3 had already moved.

## 8.3 `c_artifacts` — preserved (landmine 3 defused)

`build_lib.py` now merges instead of rebuilding: `PRESERVED_MANIFEST_KEYS = ("c_artifacts",)` is read from the existing
manifest and carried forward; `output` is still replaced wholesale. **Both** writers share the path — `compile_abilities.py`
*and* `compile_cards.py` — so the fix is in the shared helper, which covers both. Proven three ways: (a) a 7-assertion unit
test in a temp dir — survives both writers, content intact, `output` still refreshed, schema still switches; (b) the real
`regen_all.sh` printed `manifest: preserved 'c_artifacts' from the previous manifest`; (c) the resulting manifest hashes
`e12aea100594d0ec…` — **byte-identical to before the regen**, possible only if the key survived *and* the fresh `output`
reproduced exactly. `c_artifacts` was then re-measured to the new generation (`generation: "canonical"`, 726 671 /
98 782 / FNV 3 985 478 542, plus the four current file digests).

## 8.4 The harness blind spot is closed, and it is proven to fire

`tests/test_artifacts_consistent.c` grew a **section 6** that leaves the island. §1–5 compare `gen_data.c` against
whatever `rb_load("src")` opened — the two halves of that check move together, so it can never detect this class.
§6 compares against the two things the *other* pipeline generates and commits independently:
`cards/build/abilities_strings.bin` by **length and whole-file FNV-1a**, and `generation_manifest.json`'s
`c_artifacts` block against the values actually compiled into the binary. **"Not recorded" is a FAIL, never a pass** —
an absent `c_artifacts` is how the 2026-10-04 skew shipped. `ck()` now prints `FAIL: ` at **column 0**: the old
indented `  FAIL  ` was invisible to every `^FAIL` count, because `verdict_runner.c` only rewrites a prefix that
begins the line.

| condition | old suite | new suite |
|---|---|---|
| skewed tree (as found) | **12 checks, 0 failed, "all digests match", exit 0** | **23 checks, 3 failed, exit 1** |
| `c_artifacts` block deleted | — | **1 FAIL, exit 1** |
| `c_artifacts.strings_table_len` → 726 831 | — | **2 FAIL, exit 1** |
| **one byte** flipped in the canonical blob (length unchanged) | — | **1 FAIL, exit 1** (digest only) |
| clean tree after regen | — | 23 checks, 0 failed, exit 0 |

The first row is the defect, measured on one tree with one build: the old suite printed "all digests match" while
`verify_artifacts.py` called the same tree skewed. The fourth row is the one that matters most — a single flipped byte
with total length preserved is invisible to every length check, which is precisely the 2026-10-01 failure mode.

## 8.5 `describe.c` corrected — and its own caveat turned out to be stale

The old text said "Rust reads the STALE 726649-byte string blob while the C reads the canonical 726831-byte one."
**Exactly backwards**, and 726 649 was a third value on neither disk. Measured: Rust read the canonical **726 671**
(`32fa1a69…`); the C read the stale **726 831** (`89204df0…`). Rewritten to state that, to cite both sizes and both
sha256 prefixes, and to record that the two are now byte-identical.

The correction invalidated the comment's own instruction, which is worth stating plainly: **"Do NOT fix this file for
ability 325" is WITHDRAWN** — 325 renders identically now (its slice went 159→149 B). `test_describe_golden.c` is RED
on exactly one ability, **327** (`PL!-pb2-014-P＋`/`-R`, 113→141 B, the largest length change in the corpus): the C
renders two `success_live_zone` actions, the golden records one. The golden is a fixture of Rust's **pre-regen**
output, so this is a stale fixture — but that is an **inference**, because re-deriving it needs the Rust dumper and
`cargo` was not run. **327 is left OPEN, not claimed fixed and not claimed a defect.** Suite: 1 394 checks, 1 failure.

## 8.6 The cross-engine check — the proposed patch is BROKEN as written

`ARTIFACT_SKEW_buildrs_patch.md`'s `PROVEN TO FIRE` table does not transfer to the code it ships. Two defects in its
`array()`/`scalar()`, both confirmed against the real `abilities_gen.rs`:
1. `t.split(k).nth(1)` takes the text after the **first** occurrence. For `OFFSET_DELTAS` that occurrence is inside a
   **doc comment** — "`OFFSET_DELTAS[i]` is the size of the binary-JSON slice for…" — so the extractor reads prose.
2. Even at a real declaration, `b[b.find('[')..b.find(']')]` matches the **type annotation**:
   `STRINGS_OFFSETS: &[u32] = &[0, 4, …]` starts its bracket scan at the `[` of `[u32]`, so `inner` is `"u32"`.
Both arrays come back **empty**, `delta_sum` is 0, and the check emits spurious warnings *on a correct tree* — including
`ability count C 985 vs Rust 985`, which fires while both sides are 985. Measured with the patch's own logic:

| tree | patch's `array()` | corrected |
|---|---|---|
| skewed, as found | 4 warnings (2 real, 2 spurious) | 2 warnings (both real) |
| **after regen, correct** | **3 warnings — 2 spurious, 1 "985 vs 985"** | **0 warnings** |

The corrected extractor (anchoring on the declaration and skipping the type, as `verify_artifacts.py`'s `rs_array()`
already does) reproduces the doc's claimed 2/0 and was run against real bytes throughout.

**`engine/build.rs` was NOT edited.** It is inside the oracle tree, `cargo` is off limits so the Rust could not be
compiled even once, and the patch as written would have shipped permanently noisy — an uncompilable check that fires for
the wrong reason is worse than none. **The equivalent check IS landed where it compiles and proven to fire** — §8.4
section 6, the same comparison from the C side, plus `verify_artifacts.py` exit 0.

## 8.7 RE-MEASURED — failures that moved, not net improvement

`make clean` before both. `^FAIL` = `^FAIL` at line start, case-sensitive, **stdout+stderr**, all five prefixes;
`coverage_gate` excluded from every figure (meta-suite; 274 before / 8 after — it walks the logs and is not a metric).
Per-suite counts include each red suite's own `FAILED: <verdict>` line.

**Total `^FAIL`: 3 902 → 3 904 (net +2). Suites PASSED: 165-resolved set, 52 → 48.** The net is the least informative
number here. **6 suites got worse (+26) and 4 got better (−24).**

WORSE — every one an **expectation pinned to the old generation**, which is what a regeneration is *supposed* to expose:
| suite | before → after | why |
|---|---|---|
| `parity_wave1c` | 45 → 56 | names `PL!-pb2-014-P＋` = **ability 327**; now decodes 2 `move_cards` actions |
| `parity_jidou_watch2` | 69 → 74 | energy-deck leg counts (got 5/15, expected 6/16) |
| `decode_bytecode_shape` | 0 → 3 | `:142-143` hardcode `RB_SHAPE_BYTECODE_LEN 98737u` / `RB_SHAPE_STRINGS_TABLE_LEN 726831u` |
| `parity_enums` | 0 → 3 | `:611,617` hardcode census 2228 and 1782 — `:506` says *"pinned so a regeneration is caught"* |
| `describe_golden` | 0 → 2 | stale pre-regen fixture, ability 327 (§8.5) |
| `vm_decode` | 0 → 2 | `:352` hardcodes 649 |

BETTER — **stale-data-induced failures that now pass**, i.e. the C decodes current data correctly:
`audit_conditions` 25 → **11** (−14), `parity_jidou_life` 32 → **27** (−5), `parity_trigger_paths` 43 → 42,
`artifacts_consistent` 4 → **0** (the new gate going green is the only reason this suite improved).

**`parity_wave1c` is NOT among §5's 20 predicted C files** (§5 named `parity_wave1a`), so the re-pointing scope is
**at least 21 files**, not 20. None of these six were touched: they are not mine, and re-baselining a census is the
owner's call, not a mechanical find-and-replace. §5's 29 Rust files are likewise untouched.

## 8.8 Found, NOT fixed

- **`Makefile:154-155`** still calls 726 831 "canonical" — it is not. One word, outside the pin block I was authorised to
  touch. **Owner's call.** **`Makefile:845-851`** claims `src/web_server.o` is "NOT in SRC or OBJ_LIB" but `:64` **does**
  list it: no target builds it, so a clean tree **fails to link every suite** (`cannot find src/web_server.o`) until you
  run `make src/web_server.o` by hand. Measured, bit me once. Both are Makefile — not fixed.
- **`platforms/genesis/c/pack.py:48-62`** slices `src/abilities_strings.bin` at `RBKA_STRINGS_OFFSETS`. It pins no size
  or digest so nothing can go stale, but it is a **second silent-splice site with no guard at all**, safe only because
  `make data` keeps the pair aligned. Worth a tripwire.
- **`build_lib.py` preserves `c_artifacts` but cannot tell whether it is still TRUE** — now the harness's job, and the
  manifest says so. A future key outside `PRESERVED_MANIFEST_KEYS` is still dropped silently. Likewise `describe.c` now
  cites `gen_data.c:579` and `abilities_gen.rs:25/55`: true today, stale on the next regen, as the numbers they replaced were.
- **PowerShell 5.1 `>` writes UTF-16LE.** It silently corrupted my first two metric files; I caught it only because a
  byte-diff tool read them as mojibake. Anything scripted against PowerShell-captured output must detect the BOM.

## 8.9 ⚠ TWO LIVE ISSUES, both reported not chased

**(a) A NEW skew arrived AFTER this work finished, and every gate caught it.** While §8 was being written, another agent
ran `cards/compile_abilities.py` without layer 3. Measured then: layer 2 = **726 742** `775932947f75711f`, **6034** string
offsets, 98 809-byte `abilities.bin`; layer 3 still my 00:02 generation (726 671 / 6033 / 98 782). All four gates fired
and **the splice was prevented**: `make data` **refused** (exit 2 — `src/` still holds the matching 726 671 blob,
`32fa1a69…`); `verify_artifacts.py` exit 1, "4 057 of 6 032 interned strings do not match"; the §8.6 corrected check
warned `string offsets C 6033 vs Rust 6034`; `test_artifacts_consistent` §6 would fail 2. **I did not chase it** — another
agent is mid-flight and re-running `regen_all.sh` would race them. ⚠ **The brief's contingency is now REAL:** that regen
takes `RBKA_NUM_STRING_OFFSETS` 6033 → **6034**, so whoever runs it **must** re-measure `sizeof(GameState)` and move
`RB_GAMESTATE_BYTES_PINNED` + `Makefile:3` together (§8.2).

**(b) CONFOUND — the two corpora were NOT run against the same engine source.** `ability_queue.c`, `zones.c`, `card.c`,
`engine.c`, `alloc_counter.c` — five `src/**` files I do not own and did not touch — carry mtimes between the two runs.
So §8.7 is *stale data + regen + a third party's concurrent edits* vs *canonical data + regen + those same edits*, and is
**not cleanly attributable to the data change alone**; log mtimes are OneDrive-distorted so the overlap cannot be bounded
precisely — stated rather than guessed. What survives inspection: all 6 newly-red suites fail on generation-specific
constants or on ability 327, regen-attributable by reading the assertion; the 4 improvements are **not** individually
attributable. Settling it needs both corpora re-run on a quiescent tree (2 × ~11 min).
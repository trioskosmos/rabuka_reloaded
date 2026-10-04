# RUST_ORACLE_REPAIR — the oracles that never ran

**HEAD measured: `5ce311e683141ccdc7abfce452980c9c310ae20b`** (`git stash list` → `stash@{0}: On
master: session-wip`, untouched).

**Read from the WORKING TREE, which is dirty.** `engine/src` carries **7 modified files**
(`abilities_gen.rs`, `ability/choice.rs`, `ability/condition/card.rs`, `condition_decoder_gen.rs`,
`effect_decoder_gen.rs`, `ability/effects/move/mod.rs`, `core/card.rs`) — this contradicts the audit's
Rank 8 ("nothing"). The generated artefacts were **regenerated twice during this session**
(≈05:54Z and again 11:01:02–11:01:06Z), so the C-vs-Rust skew measured below is a *fourth* state.
Nothing was compiled. `cargo` was never run. No `engine/src/**` implementation file and nothing under
`engine_c_wip/src/**` was edited by me — the `engine_c_wip/src/*` modifications in `git status` are a
peer's, not mine.

## Summary

| # | item | verdict |
|---|---|---|
| 1 | `pvp_room_test` (14 tests, never compiled) | **REPAIRED** — split; routing half now compiles |
| 2 | `bytecode_deep_compare_test` | **REPORTED** — deliberately NOT enabled (would be red) |
| 3 | 3 phantom `mod.rs` | **REPAIRED** — deleted, files + directories |
| 4 | `unique_abilities_test.rs` (0 bytes) | **REPAIRED** — deleted |
| 5 | `self_to_waitroom_…own_card_is_candidate_test.rs` | **ALREADY FIXED** by a peer — now tracked + registered |
| 6 | `build.rs:325` discards the staleness verdict | **REPORTED** — patch given, not applied |
| 7 | remaining un-failable tests | **REPORTED** — 3 real, 2 borderline, audit corrected |
| 8 | Q213 has zero effective coverage | **REPAIRED** — 3 tests written, 2 fail today by design |

---

## 1. `pvp_room_test` — REPAIRED by splitting, not by adding a feature

**Decision: (b) move the assertions into a suite that compiles.** Not (a), not (c).

Why not (a) `server` in `default`: `server` (`Cargo.toml:71`) pulls actix-web, actix-web-actors,
actix, actix-cors, actix-files, tokio, tokio-stream, local-ip-address, uuid, bytes into **every**
build, including the 27 binaries that link this crate and the console targets that set explicit
feature sets. The cost of 10 phase tests on one function is the whole async web stack.

Why not (c) delete: the JSON key `"choice_player_id"` is the wire contract read by
`game/web_server.rs:1240-1249`, which **drops the entire `pending_choice` object when the key is
missing OR names anyone but the requester**. A missing key is a HIDE, not a fallback — so on a key
regression the C's own web client would blank the prompt for both seats. Deleting the only assertion
on that key leaves the contract unguarded.

**What I did:**

- **Added `engine/tests/test_modules/integration/choice_player_id_routing_test.rs`** (4 tests),
  **registered at `integration/mod.rs:3`**. I simulated `build.rs`'s generator over the whole
  `mod.rs` tree (byte-exact header, ordinal sort) and got **0 mismatches** — so this file is
  registered and the next `cargo test` will not rewrite it.
- **DELETED `pvp_room_test.rs`** outright rather than leaving a trimmed husk. It cannot compile
  (`lib.rs:149` + `Cargo.toml:62`), so its 10 `pvp_player_can_act` phase tests contribute nothing and
  its header would be the next reader's first stop. **Recorded loss:** those 10 tests now have no home
  at all; the only way to run them is `cargo test --features server`, which nothing does. Anyone who
  wants them should re-add the file together with the `Cargo.toml` feature and the `build.rs`
  exclusion removal below. Re-verified the generator simulation after the deletion: still **0
  mismatches** (the file was never declared in `integration/mod.rs`, so nothing needed updating).

Coverage restored in the compiling suite (each verified against source before writing):

| case | assertion | verified against |
|---|---|---|
| opponent stamp honoured verbatim | `get_pending_choice_player_id() == "p2"`, JSON key `"p2"` | `abilities.rs:2457-2469`, `:2559-2568` |
| default = entry owner, not "whoever is active" | stamp is not `p2`/`player2` | `queue.rs:354-360` |
| `SelectAutoAbility` seat reaches the wire | `player_id == "p2"`, JSON `player_id == "p2"` | `types.rs:567-582` `flatten_choice` |
| end-to-end two seats, real ライブ開始時 window | `SelectAutoAbility.player_id` `"p1"` then `"p2"`, `options.len() == 3`, JSON `"choice_player_id"` `"p1"` then `"p2"` | `pvp_room_test.rs:204-345` (live_start flow) |

Two writer paths matter and both are now covered: `abilities.rs:2565` (entry stamp) and
`abilities.rs:2692` (a **second, later** insert that OVERWRITES the key for
`SelectAutoAbility`/`SelectLiveSuccess`). For a `SelectAutoAbility` stored via
`pause_for_auto_ability_choice` there is **no queue entry**, so both inserts are skipped and the
top-level key is absent entirely — routing then lives only on the variant's `player_id`. The test
pins that, which is the case the dead file never reached.

**One coverage gap, stated rather than papered over.** The "a pre-existing stamp is never
overwritten" rule (`queue.rs:319`) is covered only *indirectly* — the end-to-end test asserts the
surviving JSON value across a real `execute_choice` re-stamp, and
`abilities.rs:2565` vs `:2692` is the overwrite pair that actually competes. I did **not** write a
direct unit test for it: the only way to exercise it is through
`ability/effects/choice.rs:142-153` (`execute_choice`'s `choice_maker` arm — note this is the **277-line**
`ability/effects/choice.rs`, **not** the 3921-line `ability/choice.rs`; a bare `choice.rs:142`
resolves to the wrong file), which needs a live ability mid-resolution. I would have to compile to
confirm such a test, and `cargo` is off limits. Leaving it as a named gap rather than shipping an
unverified test.

⚠️ **Required patch I could not apply — `engine/build.rs:90`.** I did not edit `build.rs` (outside my
scope). `EXCLUDED_MODULES` currently reads
`const EXCLUDED_MODULES: &[&str] = &["pvp_room_test"];`
Recommended: delete the entry entirely — `pvp_room_test.rs` no longer exists, so the list has no
scope). `EXCLUDED_MODULES` currently reads
members and only serves to make a reader believe an excluded suite is being held back.
**This is not urgent:** with the file gone, `EXCLUDED_MODULES` simply never matches, so leaving it
costs nothing but a stale line. The `const` (and its doc comment, which says `EXCLUDED_FILES` while
the identifier is `EXCLUDED_MODULES`) can be removed whenever `build.rs` is next touched.

## 2. `bytecode_deep_compare_test` — REPORTED, deliberately NOT enabled

Gated `#[cfg(all(feature = "bytecode_abilities", feature = "json_path_test"))]` at
`characterization/bytecode_deep_compare_test.rs:1`; `build.rs:149-152` only adds
`#[cfg(feature = "bytecode_abilities")]` to the `mod.rs` line, so the module is declared and then
compiles to nothing internally.

**New fact that changes the calculus:** `json_path_test = ["serde_support"]` (`Cargo.toml:66`) and
`serde_support` is **already in `default`**. So `json_path_test` is a pure **marker feature with zero
build cost** — the gate is not weight, it is purely "would go red".

### ⚠️ The brief's numbers are stale — the skew is bigger and still moving

| | Rust (measured 11:01Z) | C (committed) |
|---|---|---|
| bytecode payload | **98782** (`abilities.bin` 98790 − 8-byte `RBKA` header) | **98737** (`RBKA_BYTECODE_LEN`, header already stripped) |
| `sum(OFFSET_DELTAS)` | 98782 = payload ✔ exact | 98737 ✔ exact |
| interned strings | 6033 offsets | 6033 offsets |
| manifest | `bytes 98790`, `interned_strings 6032`, input `sha256 5dff63de521b84b2` | — |

The audit measured 98725/6030 vs 98737/6032. It is now 98782 vs 98737 — **Rust is +45 ahead**, i.e.
the direction reversed and the gap grew. Both sides remain internally self-consistent.

**Divergence is 15 abilities, not 2.** Comparing slice-by-slice (the only meaningful measure — a raw
byte-stream diff counts string-table renumbering, which is why "63 277 of 98 737 bytes differ" is
misleading; **969 of 985 slices differ in raw bytes but only 15 differ in LENGTH**):

| slice | card | Rust | C |
|---|---|---|---|
| 151 | `PL!SP-bp5-004` | 134 | 138 |
| 159 | `PL!SP-bp7-005` | 85 | 89 |
| 302 | `PL!-pb1-015` | 97 | 95 |
| 306 | `PL!-pb1-028` | 126 | 124 |
| **325** | **`PL!-pb2-012-P＋` / `-R`** | **149** | **159** |
| **327** | **`PL!-pb2-014-P＋` / `-R`** | **141** | **113** |
| 412 | `PL!N-bp7-001` | 99 | 103 |
| 643 | `PL!-pb2-021` | 158 | 156 |
| 773 | `PL!N-bp4-018` | 146 | 144 |
| 785 | `PL!N-bp5-013` | 84 | 74 |
| 801 | `PL!N-bp7-022` | 188 | 184 |
| **805** | **`PL!N-bp7-028-L`** | **236** | **238** |
| 812 | `PL!N-pb1-037` | 204 | 198 |
| 912 | `PL!SP-bp4-016` | 105 | 88 |
| 934 | `PL!SP-bp7-016` | 85 | 89 |

**The brief's "3 tests" is an undercount.** 325 and 805 are stable across all three generations, but
they account for 4 test *files*, not 3:

- 325 → `effects/cost/additional_cost_choice/pb2_kidou_additional_cost_discard_two_or_wait_two_printemps_pl_pb2_012_p_plus_test.rs`, `effects/cost/play_cost/pb2_optional_two_distinct_printemps_wait_costs_two_less_pl_pb2_012_p_plus_test.rs`
- 805 → `characterization/card_type_filter_test.rs`, `effects/gain/hearts/live_start/live_start_discard_shuffle_group_members_heart01_test.rs`

Adding the other 13 divergent abilities, **30 distinct files under `engine/tests/test_modules`
reference a card whose encoding differs** (29 test files + 1 helper,
`support/baton_swap_auto_helpers.rs`). The largest new one is 327 =
`effects/move_cards/pb2_reveal_lily_white_live_and_swap_success_zone_pl_pb2_014_p_plus_test.rs`, which
appeared only in the 11:01Z regeneration. Full per-slice file lists are in the audit's
`TEST_INVENTORY.md` `tests` column.

**Required order — do not reorder:**

1. **Regen** one side and pin both to it (the C's `Makefile:122-123` already pins 985/6032/726831/98737
   and guards it at `:134-151`; the Rust side has no such guard).
2. **Re-point every test touching those 15 abilities** to the new encoding — 29 test files, not 3.
3. **Then** enable: add `json_path_test` to `Cargo.toml:62` `default` and fix `build.rs:149-152` to
   emit `#[cfg(all(feature = "bytecode_abilities", feature = "json_path_test"))]` so the `mod.rs`
   gate matches the in-file gate.

Enabling before step 2 turns the suite red immediately — which is *correct* for an oracle, and is
exactly why the order is the deliverable.

## 3. Three phantom `mod.rs` — REPAIRED (deleted, with their directories)

Each declared a module absent from disk **and from all of git**, and each parent directory held
**nothing else**. Measured: **zero** `.rs` siblings and **zero** subdirectories.

- `engine/tests/test_modules/ability_gates/` (151 B) → `bounded_fan_cost_equality_gate_test`
- `engine/tests/test_modules/expansion/` (137 B) → `pbl02_card_table_test`
- `engine/tests/test_modules/effects/hearts/` (148 B) → `set_heart_type_multi_target_test`

**Sharper than the audit recorded: none of the three was reachable from the crate root.**
`test_modules/mod.rs` declares neither `ability_gates` nor `expansion`, and `effects/mod.rs` does not
declare `hearts` (the `pub mod hearts;` at `effects/gain/mod.rs:5` is a *different* directory,
`effects/gain/hearts/`, which holds four real child suites). So they could never break a build — they
were inert files that read as coverage. Deleted; no parent edit needed, which is why the generator
simulation reports 0 mismatches.

**Sweep for others:** I checked every `mod.rs` under `engine/tests/test_modules` for "no `.rs`
siblings **and** no subdirectories". After the deletions the count is **0**. (A first pass that
ignored child directories reported ~23 false positives; `effects/gain/hearts/mod.rs` looks empty at
the top level but has 4 populated subdirectories.)

## 4. `unique_abilities_test.rs` (0 bytes) — REPAIRED

Deleted, and removed from `characterization/mod.rs` (it was line 28). Declared at HEAD, 1 byte on
disk (a newline) — it compiled as an empty module and its name implied unique-ability coverage that
does not exist. No other file references it; `corpus_smoke_test.rs` /
`corpus_decode_coverage_test.rs` carry the real corpus work.

## 5. `self_to_waitroom_recover_member_own_card_is_candidate_test.rs` — ALREADY REPAIRED by a peer

The brief's premise is **stale**. Measured now:

- **Untracked files under `engine/`: 0.** The peer's correction ("there is 1, not 6") was right when
  written and has since been acted on — the file is now **tracked** (`git ls-files` confirms).
- **Registered at `effects/recover/to_hand/mod.rs:38`**, and `to_hand/mod.rs` is *not* in
  `git status`, so the registration is at HEAD, not a working-tree edit.
- The file is good quality and I left it untouched: 3 tests, a real negative control
  (`:138-155` `a_live_in_the_waitroom_is_not_a_candidate`), and `:21-25` names the exact
  "snapshot the cards this cost moved" optimisation that would make it vacuous.

**Nothing to do.** Confirming rather than re-doing, per the instruction.

## 6. `build.rs:325` — REPORTED, not applied (`build.rs` is outside my edit scope)

```rust
// build.rs:321-325, current
// In CI, stale is a hard error; locally it still warns but `cargo test` will surface the warning.
// To make it a hard error everywhere, uncomment the next two lines:
// if stale { panic!("Stale generated files — regenerate as warned above"); }
// For now keep it as warning so local `cargo test` stays green while CI can gate on `cargo:warning` → error via `ci.yml` deny-warnings.
let _ = stale;
```

**Would it fire today? NO — I verified all four checks pass:**

| check | line | measured |
|---|---|---|
| `abilities.json` older than `cards.json` | `:270` | abilities 2026-10-04T11:01:02Z > cards 2026-09-29T03:34:22Z ✔ |
| `abilities_gen.rs` older than `abilities.json` | `:278` | gen 11:01:05.17Z > abilities 11:01:02.15Z ✔ |
| manifest `input.sha256` vs `abilities.json` | `:295` | manifest `5dff63de521b84b2`; `sha256(abilities.json)[..16] = 5dff63de521b84b2` ✔ (recomputed after the 11:01Z regen) |
| `abilities.bin.z` older than `abilities.json` | `:315` | bin.z 11:01:04.97Z > abilities 11:01:02.15Z ✔ |

**Recommendation: enable it.** It is safe today, and the "keep local `cargo test` green" comment is
the weaker argument — a stale-data bug that only surfaces as a gameplay failure far from its cause is
worse than a build that stops. But it is a judgement call, and the honest framing is that **enabling
it alone fixes nothing here**: it is a no-op now and a tripwire later, and its window is narrow.

### ⚠️ The sharper point: this detector is structurally blind to the cross-engine skew

All four checks read files **inside the Rust generation chain** — `cards/cards.json`,
`cards/abilities.json`, `engine/src/ability/abilities_gen.rs`, `cards/build/abilities.bin.z`,
`cards/build/generation_manifest.json`. **Not one names `bytecode_blob.c`, `gen_data.c`,
`abilities_strings.bin` (the C copy), or any C file.** Rust cannot report that its committed dataset
is four generator generations behind the C's, because it never looks at the C's. That skew went
unnoticed for the whole project and this check would still not see it.

Note also what it *cannot* catch even inside Rust: a generator run landing between
`abilities.json` and `abilities_gen.rs` — the three artefacts are written inside a 4-second window
(11:01:02 → 11:01:06Z) every single run, so that is not a theoretical window.

**What would close the gap** — three layers, cheapest first:

1. **Content hash of the C tables, recorded in the manifest.** `generation_manifest.json` already
   carries `output.sha256` and `interned_strings`. Add the C artefact digests as
   `output.c_side: {blob_sha256, offsets_sha256, strings_sha256, bytecode_len, strings_len}`, emitted
   by `tools/gen_from_rs.py` / `gen_bytecode.py` when they write the `.c` files. One field; makes a
   skew visible in a diff of a JSON file.
2. **A build-time cross-check in `build.rs`.** Add `engine_c_wip/src/core/generated/bytecode_blob.c`
   and `gen_data.c` to `rerun-if-changed`, parse `RBKA_BYTECODE_LEN` / `RBKA_NUM_ABILITIES` /
   `RBKA_NUM_STRING_OFFSETS`, and `panic!` when they disagree with
   `OFFSET_DELTAS.len()` / `sum(OFFSET_DELTAS)` / `STRINGS_OFFSETS.len()` / `STRINGS_BLOB.len()`.
   This is ~20 lines, needs no new toolchain, and turns the skew into a build failure instead of a
   silent divergence. It is the same class of assertion as the C's
   `tests/test_artifacts_consistent.c` (offsets monotonic, whole-table FNV, `RBKA_BYTECODE_LEN ==
   sum(RBKA_OFFSET_DELTAS)`) — but on the Rust side, and comparing the two ends.
3. **`bytecode_deep_compare_test` enabled** (§2). This is the only check that compares *decoded
   semantics* rather than lengths. It is necessary, not sufficient: it proves Rust's blob matches
   Rust's JSON, which says nothing about the C's blob. To bind the C it needs a C-side twin
   (`test_decode_bytecode_shape.c` is the right shape) plus a shared fixture.

Layers 1+2 are the ones that would have caught the 325/805 skew on day one. Layer 3 is what stops the
next one.

## 7. Tests that cannot fail — REPORTED

**Verified clean (correcting the audit where it overreached):**

- **0** assertion-free tests. A mechanical pass flagged 67, then 5 after resolving local callees; all
  67 are wrappers whose assertion lives in a helper (`check_heart_reduction` at
  `jellyfish_…test.rs:29`), in a `TestGame` method (`assert_card_in_group`), or in a macro
  (`assert_ability!`).
- **0** real `#[ignore]`d tests. Three textual hits, all inside `///` doc comments
  (`cost_tier_remainder_to_deck_top_routing_test.rs:154`,
  `yell_real_yell_no_blade_heart_grants_heart_test.rs:81`,
  `q172_heart_total_includes_ability_but_not_blade_test.rs:147`) — each narrating that a former
  `#[ignore]` was removed *because* it was showing the wrong thing.
- **0** tests excluded by env var. Only 3 files read the environment, and both switches are
  diagnostics that ADD output and gate no assertion: `RABUKA_RULE_LOG`
  (`helpers/diagnostics.rs:93`, `Drop for TestGame`) and `RABUKA_TRACE_DIR`
  (`helpers/trace.rs:37`, where absent ⇒ in-memory trace, which is the tested default).
- **0** `assert_eq!(x, x)` and 0 constant-vs-constant across 4184 `#[test]`/`#[tokio::test]` fns.

**Genuinely un-failable / weaker than named:**

| file:line | assertion | why |
|---|---|---|
| `jidou/discard_watch/live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs:59` (pre-edit) | `after_waitroom >= before_waitroom` | satisfied by zero cards added; file never read a heart modifier. **REPAIRED — see §8.** |
| `effects/compound/cost_and_effect/nahone_live_start_same_name_test.rs:878` | `assert!(blade1 >= 1); assert!(blade2 >= 1);` | name says *not buffed*; asserts a **floor of 1**, which a second buff only raises. Needs an upper bound or equality against the un-buffed baseline. Still present. |
| `effects/score/per_card/live_success_surplus_score_zero_floor_test.rs:83` | `p1_constant_total_score_bonus >= -2` | **new finding.** The comment at `:81` says "total should still be >=0" and the message at `:84` says "total stays >=0", but the bound is `-2`. An implementation returning `-2` passes while both the comment and the message claim `>=0`. The test name matches the assertion; the intent does not. Tighten to `>= 0` or fix the prose. |
| `integration/full_round/live_score_tie_live_zone_placement_across_rounds_test.rs:258` | `turn_number >= 1` | true before any pass (initialised to 1). **Audit correction:** this is NOT the test's only assertion — `:252-256` strongly asserts both success zones are empty. So "no winner / no placement" IS covered; only the **rollover** half of the name is unverified. Fix by asserting the counter *advanced past its pre-loop value*. |
| `effects/look_select/reorder_top/live_success_look_three_deck_size_q36_test.rs:34` | `deck_after <= deck_before` | permits equality, so "nothing happened at all" also passes. The name ("does not increase") is faithful to `<=`, but `<` would actually pin Q36. |

**Audit correction — `dive_ab0_…test.rs` is NOT vacuous.** The brief and the audit both say its only
assertion is `guard <= 20`. That is the liveness guard inside the drain loop at `:67`; the test then
makes three strong assertions at `:80-93` (P1's DIVE! placed in the live zone, P2's DIVE! in hand,
P2's DIVE! **not** in the live zone). It cannot fail. Same for
`cost_tier_remainder_to_deck_top_routing_test.rs:225` — a `guard <= 20` liveness guard.

**Deliberately-weak-but-legitimate, recorded so they are not re-audited:** the many
`assert!(prompts <= N)` / `assert!(activations <= 1)` / `assert!(guard <= N)` soft guards
(`otherwise_condition_flow_test.rs` ×4, `turn_limited_activation_offered_once_test.rs:145`,
`energy_discard_live_recovery_pl_sp_sd2_006_sd2_test.rs:126`). These are anti-hang guards that sit
*alongside* real `assert_eq!`s; the second refinement pass confirmed none of them is a test's sole
assertion.

## 8. Q213 — REPAIRED: 3 tests, 2 fail today by design

**Rewrote `jidou/discard_watch/live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs`**
in place (same path, so no `mod.rs` edit; already declared). It now reads
`get_need_heart_modifier(hanamusubi, HeartColor::Heart04)` — which the old file never did.

⚠️ **These are KNOWN-FAILING REFERENCE TESTS. A failure in either is the documented Q213 defect in
the Rust engine, NOT a C regression.** Marked as such in the file header.

| test | asserts | today |
|---|---|---|
| `a_renosora_live_card_in_the_slot_does_reduce_heart04` | control: a 蓮ノ空 **live** card gives `-2` | **PASSES** |
| `q213_a_renosora_member_in_the_slot_reduces_nothing` | Q213: a 蓮ノ空 **member** gives `0` | **FAILS** (engine yields `-2`) |
| `q213_the_renosora_member_is_in_the_waitroom_when_live_start_resolves` | Q213: the member is in the **waitroom**, not the live slot, while the live card stays | **FAILS** (member still in the live slot) |

The control is what makes the other two measurements rather than assumptions: if it ever reads 0 the
fixture is broken and the member assertions would pass for the wrong reason. Fixture preconditions
(card types) are asserted, not assumed. The window is entered by NAME via
`advance_to_phase(Phase::LiveCardSetFirstAttacker)` / `(Phase::FirstAttackerPerformance)` — never a
blind `for _ in 0..5 { pass() }`, because Q213 *is* a phase-ordering defect and a frozen pass count
would move with the phase list silently. The third test closes a real hole: an implementation that
fixed the reduction by adding an `is_live()` filter at the counting site, while leaving the member
sitting in the live slot, would pass test 2 and fail test 3 — and would still be wrong, because the
ruling says the card **moves**.

**The chain it pins** (all read in the working tree):

- `turn/phases.rs:240` — `check_timing` with `face_up == false`
- `turn/phases.rs:245-246` — `trigger_live_start_abilities`, the Q213 window
- `turn/phases.rs:343` — `face_up = true`, first statement of `execute_performance_phase`
- `turn/phases.rs:349` — `check_timing`, now evicting non-live cards
- `turn/actions/mod.rs:1732-1736` — `check_invalid_live_cards` early-returns unless `face_up`
- `ability/condition/card.rs:4518-4519` — the `LiveCardZone` counting arm, **no `is_live()` filter**

`execute_performance_phase` runs at `phases.rs:258`, i.e. **after** the scan at `:245-246`. So at
resolution time the member is still in `live_card_zone.cards` and the eviction has not run for that
seat. The audit's note that `turn/actions/mod.rs:1734` "defers the eviction one phase too late" is
correct in effect; the precise mechanism is the `face_up` gate plus the ordering of `:245` vs `:343`.

**Required implementation patch — REPORTED, NOT APPLIED** (`engine/src/**` is outside my scope):

- Minimal, at the counting site: add an `is_live()` filter to the `Zone::LiveCardZone` arm at
  `ability/condition/card.rs:4518-4519`. Makes test 2 pass; leaves test 3 failing.
- Correct, and what the ruling actually says: make the eviction happen **before**
  `trigger_live_start_abilities` at `phases.rs:245-246` — set the performing seat's `face_up = true`
  (or call the eviction directly) prior to the scan, so the member is in the 控え室 when the ability
  resolves. Makes both tests pass.
- Precedent for the same ruling in the C: `engine_c_wip/src/turn/phase.c:1651-1654`. ⚠️ That comment
  **misdiagnoses the bug** — it blames writing `live_face_up` *after* `rb_check_timing`, but in Rust
  the flag is written *before* it (`phases.rs:343` then `:349`). The defect is that
  `execute_performance_phase` runs after `trigger_live_start_abilities`. An agent following that
  comment will fix the wrong line. **Fix the comment before someone acts on it.**

**Net effect on the C:** the C inherits the defect by having nothing to port, not by porting a bug —
`engine_c_wip/src/turn/phase.c:1658-1663` reproduces Rust's ordering inside the performance phase,
itself downstream of the live_start trigger. The C is faithful; the oracle was silent. Now the oracle
speaks, and it convicts the Rust side.

## Also verified (cheap, and it changes what the next reader should trust)

- **`trigger_counts` keys are NOT mojibake** — confirmed, and I found the likely source of the
  false claim: the committed generated `mod.rs` files **do** contain mojibake when read through
  PowerShell `Get-Content` (`// Auto-generated by build.rs 窶・ DO NOT EDIT.`) purely because
  `Get-Content` mis-decodes UTF-8 in this tree. The bytes on disk are correct: `e2 80 94` = em dash,
  no BOM, LF endings. The same trap applies to any file. Use the `read` tool, not `Get-Content`.
- **Untracked Rust test files: 0.** The "1" figure was right and is now stale.
- **`build.rs` generator simulation: 0 of ~430 committed `mod.rs` differ** from what the generator
  would write (byte-exact header, Rust's **ordinal** sort — PowerShell's culture-aware sort produces
  two false positives at `effects/gain/hearts/constants/mod.rs` and
  `jidou/yell/no_blade_heart_reveal_gain/mod.rs`, both of which are correct as committed). My edits
  are registered correctly and the next `cargo test` will not rewrite the tree.
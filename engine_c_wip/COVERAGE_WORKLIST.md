# COVERAGE WORKLIST — ordered test-writing tasks for the fixer wave

**Author:** coverage audit (test-coverage cluster). **Date:** 2026-10-04.
Read-only audit; this is a work list, not an implementation. Every task names the Rust test to
port, the behaviour, the measured reachability, the C file that should receive it, and the
divergence already spotted — so a fixer does **not** re-derive the analysis.

**Ownership reminder (`PARITY_CHECKLIST.md` §"OWNERSHIP MAP"):** `tests/**` is contested.
Claim per file. Wave 0 below touches files no other wave needs.

---

## 0. ORDERING RULE

Tasks are ordered by **reachability first, then by whether the task unblocks measurement**. A
task that makes a number trustworthy outranks a task that adds coverage to a trusted number,
because the brief records three tallies in this project that were wrong *in the direction that
makes work look unnecessary*.

* **Wave 0** — harness. Nothing below can be *measured* until these land.
* **Wave 1** — restore proven coverage where a test exists but proves nothing (vacuous/inverted).
* **Wave 2** — close `NO C TEST` with real reachability.
* **Wave 3** — close the phantom-suite and uncited-suite mapping debt.
* **Wave 4** — per-card work where reachability is 1–5 (cheap, but low value; do last).

---

# WAVE 0 — HARNESS (do first; everything else is unmeasurable until these land)

Targets: new file `engine_c_wip/tests/test_coverage_gate.c` **plus** small edits to named suites.
All counts in the "measured damage" column come from a from-scratch, strictly-sequential,
159-suite run in a clean snapshot with the canonical 726 831 B / `89204df0…` blob, verified before
and after.

### 0.1 ⛔ Make a non-existent suite FAIL, not pass

**Behaviour:** `make t T=<stem>` and `make run-all-tests` must fail loudly when `tests/test_<stem>.c`
does not exist. Today `make t` *does* check (`Makefile:148`: `no such test: …; exit 2`) but
`run-all-tests` iterates a glob, so a phantom name simply never appears.
**Why:** **30 suite names in circulation have no file** (`COVERAGE_HARNESS.md` §6). Each reads as
passing.
**Divergence to fix:** add a `tests/test_suite_manifest.txt` listing every suite the project's own
documents claim, and have `run-all-tests` diff the glob against it, exiting 2 on any name with no
file. **Do not** add a `PASSED` line to existing suites — see 0.2.

### 0.2 ⛔ Give the harness a positive verdict

**Behaviour:** every suite must print exactly one `PASSED: <suite>` line on success.
**Measured damage:** `PASSED` verdict lines = **0** corpus-wide against 990 `FAILED`. A suite that
segfaults before its first assertion is byte-indistinguishable from a suite that passed.
**Divergence:** `verdict_failed()` in `test_parity_cluster_e.c:32` already prints
`FAILED: %s`; add the symmetric `PASSED: %s` in the same `FLAT_VERDICT` idiom.

### 0.3 ⛔ Count `^FAIL` without the colon, on stdout **and** stderr

**Behaviour:** the measurement must count `^\s*FAIL:` ∪ `^\s*FAIL \[` (assertions) and
`^\s*FAILED\b` (verdicts).
**Measured damage:** a `^FAIL:` scan misses **246 assertion failures across 9 suites**
(`parity_integration_extra` 69, `parity_full_round2` 68, `parity_full_round` 44,
`parity_integration_rounds` 25, `parity_full_round3` 17, `parity_energy_position_delta` 10,
`parity_realcard_walkthrough` 7, `audit_fields` 4, `parity_support` 2) and scores all nine
**0 failures**.
**Divergence:** the bracket form is emitted by exactly those 9 suites (175 lines:
`parity_full_round2` 54, `parity_integration_extra` 47, `parity_full_round` 29,
`parity_integration_rounds` 19, `parity_full_round3` 12, `parity_energy_position_delta` 6,
`parity_realcard_walkthrough` 5, `audit_fields` 2, `parity_support` 1). On PowerShell, count with
`-cmatch` and stringify make's stderr via `ForEach-Object { "$_" }` — `-match` is case-insensitive
and will count `failures (shared) : 54` as a failure.

### 0.4 ⛔ Never treat exit status as the signal

**Behaviour:** the gate must key on FAIL-line counts, never on `$?`.
**Measured damage:** **12 suites exit 0 while carrying 548 `FAIL` lines** (`parity_small_clusters`
104, `parity_under_member` 87, `parity_conditional_gated` 69, `parity_zone_source` 69,
`parity_compound_per_card` 64, `parity_blade_under` 50, `parity_recover_score_state_delta` 46,
`parity_rules_phases` 28, `parity_abilitymod_conditional` 13, `parity_rules_conditions` 10,
`parity_energy_remainder` 7, `parity_fw_effects` 1). Conversely `test_audit_conditions` exits
**2** and `test_state_effects` exits **139** with **zero** `FAIL` lines.
**Divergence:** this is the same trap as `PARITY_CHECKLIST.md`'s note that `parity_recover_live`
exits 21.

### 0.5 ⛔ `tests/test_audit_conditions.c` — fix the load, or delete the suite

**Rust twin:** none needed; this is a suite defect.
**Behaviour:** stderr is exactly `load_cards failed` / `load_cards failed` / `cannot load card db`,
exit 2, **0 stdout bytes, 0 assertions**.
**Measured damage:** a **1 002-line** suite that runs **nothing** and scores 0 `FAIL` — a perfect
score for zero coverage.
**Divergence:** its 14 assertion-message literals never execute. The `load_cards` fallback path is
the documented landmine ("`load_card_db()` falls back to `../cards/build`: `rb_load("src")` fails
under an isolated out-of-tree build"). The snapshot at
`Temp/kilo/cov/tree` *does* contain `src/cards.bin`, so the fix is to pass the right `rb_load`
directory, not to relax the check.
**Target file:** `tests/test_audit_conditions.c`.

### 0.6 ⛔ `tests/test_state_effects.c` — SIGSEGV, exit 139, zero assertions

**Behaviour:** dies inside the `energy_state_change` execution. Last stderr line:
`[RESUME_EXEC] def=0xa00195ec0 action=energy_state_change is_cost=0 host=-1`.
**Measured damage:** 33 assertion-message literals, none evaluated.
**Divergence:** the trace shows `target=0x0` on the same line — a null `target` reaches
`energy_state_change`. That is the same shape as the `-Waddress` cluster-W rows W1/W2
(`choice.c:1083`, `choice.c:5037`, two always-true address comparisons). **Report as a source bug
for the `effects/energy` owner; do not work around it in the test.**
**Target file:** `tests/test_state_effects.c`.

### 0.7 ⛔ `tests/test_p1_helpers.c` — 0 B stdout, 249 message literals

**Behaviour:** every green assertion is invisible; only `FAIL: opponent spawn routes choice to
opponent` reaches stderr. Exit 1.
**Measured damage:** 249 literals, 1 reported failure. The suite looks 249× green on a
`stdout`-only scan.
**Divergence:** it prints its `ok:` lines nowhere. Compare `test_parity_cluster_e.c:47-49`, which
uses `printf("ok: %s\n", msg)` on stdout — that is the idiom to copy.
**Target file:** `tests/test_p1_helpers.c`.

### 0.8 ⛔ `tests/tmp_list.c` — delete or make it assert

**Behaviour:** **0 B stdout, 0 B stderr, exit 0.** A suite that tests nothing and reads as passing.
**Target file:** `tests/tmp_list.c`. Also `tests/debug_eli.c` and `tests/test_zzprobe.c` are scratch
files in circulation as suites (`test_zzprobe` prints 171 lines, `debug_eli` 301 B); move them out
of `tests/` so `run-all-tests` stops counting them.

### 0.9 Fix the `VACUOUS` accounting so it can be scanned

**Behaviour:** every `VACUOUS`/`SETUP BUG` line must begin at column 0.
**Measured damage:** 314 lines **contain** the word `vacuous`; only **44 begin with it**. The 314
are dominated by per-case summaries reading `… 0 failure(s), 0 setup bug(s), 0 vacuous`. Any
substring scan counts a line *reporting zero vacuous assertions* as one vacuous assertion. That is
the mechanism behind the published "43 VACUOUS + 5 SETUP BUG" figure, which is wrong in both
count and location.
**Divergence:** rename the summary field from `vacuous` to `vacuous_asserts` (or `vac=0`), or emit
it as `vacuous-assertions=0`. One line per suite is enough.
**Target files:** every suite that prints the `N assertion(s), M failure(s), … N vacuous` shape —
`test_parity_required_hearts`, `test_parity_cost_discount`, `test_parity_state_per_card`,
`test_parity_full_round`, `test_parity_look_per_card`, `test_parity_target_player`,
`test_parity_recover_q79` and 25 more (full distribution in `COVERAGE_HARNESS.md` §4.1).

### 0.10 Fix the cross-stream case-name split

**Behaviour:** the same case must be named identically on stdout and stderr.
**Divergence:** case **`f_center_zero_heart03`** (stdout) and **`f_center_rin_zero_heart03`**
(stderr) are the same case under two names, in `test_parity_required_hearts`. Any per-case join
across streams loses it. This is the same split as `FAIL:` / `FAILED` / `FAIL [`.

### 0.11 `test_parity_look_per_card.c` — 2 children die with exit 139

`CRASH    n_tomari_accept_6  <-- unexpected child exit 139` and `n_tomari_empty_hand`.
`PARITY_CHECKLIST.md` H3 records this as non-reproducing; **it reproduced on the first run of a
fresh from-scratch build**. Fix or quarantine; do not leave a flaky child in the corpus.

---

# WAVE 1 — restore proven coverage (a test exists and proves nothing)

### 1.1 ⛔ `tests/test_parity_jidou_watch.c:675,680` — INVERTED + non-asserting

**Rust twin:** the Q213 heart-requirement reduction, whose live-slot count excludes a
non-ライブ card.
**Behaviour:** `CHECK(after >= before, "a non-ライブ card forced into the live slot is removed
before ライブ開始時 …")` asserts something about the **WAITROOM length**, which cannot detect
removal from the LIVE slot. The message and the assertion describe different things.
**Reachability:** `K=location` **992 nodes**; the live-zone-placement family is 30/985 abilities
across `A=live_card_zone` placements.
**Divergence already established:** the *second* assertion at `:680`,
`CHECK(!test_zone_has_id(&game, 0, "live", hasetsu), …)`, asserts the member is **gone from the
live zone by ライブ開始時**. **Rust keeps it until AFTER the live** — the divergence is recorded in
`PARITY_CHECKLIST.md` (T3 header item 5: `live_face_up[]` — "Rust evicts a wrongly-set member
AFTER the live, C evicts BEFORE") and T6 open item 3. **The C test is pinned to the C's wrong
behaviour and will go RED when the engine is fixed.**
**Fix:** (a) rewrite `:675` to assert what its message claims (the card is absent from the live
zone *and* `live_zone` count is the expected value); (b) mark `:680` with the `live_face_up[]`
divergence in a comment and **invert its expectation to match Rust** — or, if the divergence is
not being fixed this wave, add `// DIVERGENCE (T3-5/T6-3): Rust evicts AFTER the live; this
CHECK pins the C's current WRONG behaviour and must be inverted when triggers.c is fixed.`
**Owner:** `src/ability/effects/rules.c` (cluster C) for the engine half.
**Target file:** `tests/test_parity_jidou_watch.c`.

### 1.2 ⛔ `tests/test_parity_hearts_live_start.c:1598-1630` — guard missing two setup legs

**Rust twin:** `engine/tests/test_modules/effects/gain/hearts/live_start/live_start_discard_distinct_heart_colors_q246_test.rs`.
**Behaviour:** Q246 — put any number of 虹ヶ咲 members from hand into the waitroom; for each
**distinct** heart colour those cards carry, gain 1 heart of that colour until live end. The
load-bearing behaviour is deduplication across overlapping colours and exclusion of colours on
unselected cards.
**Reachability:** `K=distinct` **23/985 abilities · 52 pairs · 111 effect nodes · 741 condition
nodes**; `X=distinct` 19/985 · 36 pairs; `K=heart_colors` 41 nodes; the joint card `LL-bp6-001-R＋`
alone.
**Divergence:** fixture 2 pins only 3 of its 5 cards.

```c
1601:  int kosuzu2 = mid(&tg2, "PL!HS-pb1-005-R");     // NEVER ident()-pinned
1602:  int dia2    = mid(&tg2, "PL!S-bp3-004-R");      // NEVER ident()-pinned
1603:  int kotori2 = mid(&tg2, "PL!-bp3-003-R");
1605:  if (!ident(&tg2, joint2,  "LL-bp6-001-R＋")) return;
1606:  if (!ident(&tg2, kotori2, "PL!-bp3-003-R"))   return;
1607:  if (filler2 < 0) return;
```

Fixtures 1 (`:1567-1570`) and 3 (`:1638-1640`) pin every card. If `kosuzu2` or `dia2` ever fails to
resolve, `mid()` substitutes a different card, `ident()` is never asked, and the two assertions that
carry the case —

```c
1627:  CHECK_EQ(heart_of(&tg2, joint2, H01), 0, "H01 must NOT come from the unselected kotori");
1629:  CHECK_EQ(heart_of(&tg2, joint2, H03), 0, "H03 must NOT come from the unselected kotori");
```

— silently become assertions about the wrong cards, and both still pass because both expect `0`.
**Fix:** add `if (!ident(&tg2, kosuzu2, "PL!HS-pb1-005-R")) return;` and
`if (!ident(&tg2, dia2, "PL!S-bp3-004-R")) return;` at `:1605`.
**Data verified correct** (do not change the expectations): `PL!HS-pb1-005-R` = H04/H05/H06,
`PL!S-bp3-004-R` = H02/H05, `PL!-bp3-003-R` = H01/H03/H06. `heart_of()` at `:139` reads
`rb_mods_get_heart(&state.mods, …)` — the modifier total, not the printed slice — so `== 1` is
measuring the grant delta, as intended.
**Target file:** `tests/test_parity_hearts_live_start.c`.

### 1.3 The 13 vacuous cases in `tests/test_parity_required_hearts.c`

**Rust twin:** `engine/tests/test_modules/effects/cost_mod/required_hearts/*` (9 files) and
`engine/tests/test_modules/effects/gain/hearts/constants/*` — the `modify_required_hearts` family.
**Behaviour:** the required-heart reduction / increase gates.
**Reachability:** `A=modify_required_hearts` **28/985 abilities · 43 pairs · 87 effect nodes ·
315 condition nodes**. Gate keys: `K=appearance` 69 nodes, `K=movement` 117 nodes,
`K=baton_touch_trigger` 30/985 · 63 pairs · 817 cond nodes, `X=heart_colors` 466 nodes,
`X=heart_color` 311 nodes.
**Measured damage:** **all 13 cases are vacuous**, each because its paired positive is RED in the
same run.

| case | paired positive (RED) | what to write |
|---|---|---|
| `c_five_plus_duplicate` | `c_six_distinct_reduce_two` | land the positive, then the negative |
| `c_five_plus_renosora_live` | `c_six_distinct_reduce_two` | same |
| `c_five_plus_wrong_group` | `c_six_distinct_reduce_two` | same |
| `d_mirage_one_baton` | `d_mirage_two_baton_heart05` | same |
| `d_mirage_non_renosora` | `d_mirage_two_baton_heart05` | same |
| `d_mirage_renosora_no_baton` | `d_mirage_two_baton_heart05` | same |
| `f_center_zero_heart03` *(stderr: `f_center_rin_zero_heart03`)* | `f_center_two_heart03_reduces_1` | same + fix the name split (0.10) |
| `f_no_center_member` | `f_center_two_heart03_reduces_1` | same |
| `f_center_non_mus` | `f_center_two_heart03_reduces_1` | same |
| `i_no_qualifying` | `i_two_appeared_reduce_2` | same |
| `i_neither_appeared_nor_moved` | `i_two_appeared_reduce_2` | same |
| `i_appeared_then_left` | `i_two_appeared_reduce_2` | same |
| `l_heart02_lt5_no_effect` | `l_heart02_ge5_increase` | same |

**Rule for the fixer:** a negative assertion is only worth writing **after** its paired positive
is GREEN. That is what the suite's own `VACUOUS:` line already says — make it a hard precondition
(`if (!positive_is_green) FAIL("cannot prove the negative");`) instead of a note.
**Target file:** `tests/test_parity_required_hearts.c`.

### 1.4 The 10 vacuous cases in `tests/test_parity_cost_discount.c`

**Rust twin:** `engine/tests/test_modules/effects/cost/play_cost/*` and
`engine/tests/test_modules/effects/cost_mod/**` — the `modify_cost` family.
**Behaviour:** the cost-reduction gates (Wien, N, K, G prefixes in the case names).
**Reachability:** `A=modify_cost` **22/985 abilities · 48 pairs · 97 effect nodes · 235 condition
nodes**; `X=cost_limit` **76/985 · 171 pairs · 465 eff · 260 cond**;
`X=cost_limit_operator` 181 nodes.
**Measured damage:** all 10 vacuous, each because its paired positive is RED.

| case | paired positive (RED) |
|---|---|
| `f_no_reduction_for_member_with_ability` | `f_reduction_applies_to_no_ability_member` |
| `n_does_not_reduce_low_cost_group_member` | `n_reduces_cost_seventeen_group_member_by_two` |
| `n_does_not_reduce_wrong_group_member` | `n_reduces_cost_seventeen_group_member_by_two` |
| `n_absent_after_source_removed_before_play` | `n_reduces_cost_seventeen_group_member_by_two` |
| `k_gameplay_decline_pays15_keeps_hand` | `k_gameplay_accept_cost10_discards_hand` |
| `k_accept_with_ten_energy_pays_ten` | `k_gameplay_accept_cost10_discards_hand` |
| `k_passive_cost_not_set_by_discard` | `k_gameplay_accept_cost10_discards_hand` |
| `k_passive_cost_not_set_with_hand_cards` | `k_gameplay_accept_cost10_discards_hand` |
| `g_four_unique_groups_cost_zero` | `g_reduction_charges_effective_cost` |
| `g_zero_cost_all_rarities` | `g_reduction_charges_effective_cost` |

**Target file:** `tests/test_parity_cost_discount.c`.

### 1.5 `tests/test_parity_blades_live_start.c` — 2 wrong-card fixtures

**Measured damage:** `SETUP BUG: expected card PL!HS-pb6-002-R, resolved id=-1 (<none>)` and
`SETUP BUG: expected card PL!HS-bp1-002-CL, resolved id=2793 (村野さやか)`. The first card does not
exist; the second resolves to a **different card**. Every assertion in those cases tests the wrong
card and passes.
**Reachability:** `A=gain_resource` blade **272/985** — the family is huge, so a fixture that
silently tests the wrong card is expensive.
**Fix:** correct the card numbers or delete the cases. `PL!HS-bp1-002-CL` is almost certainly meant
to be a `-CL` print that the parser resolves to card 2793 via `rb_normalize_card_no`'s folding —
verify with `pin_id()`/`ident()` before changing the literal, and note that
`rb_card_no_eq` is documented as fuzzy and must not be used as an identity pin.
**Target file:** `tests/test_parity_blades_live_start.c`.

---

# WAVE 2 — `NO C TEST`, real reachability first

### 2.1 `group_name_equivalence` → **273/985** — the highest-reachability gap in the audit

**Rust file:** `engine/tests/test_modules/rules/group_name_equivalence.rs` — 3 `#[test]`, **no
corpus card named anywhere in the file** (the reason it fell out of every card-based matcher):
* `fast_path_matches_allocating_path_for_whole_corpus` (L70, 2 asserts)
* `fast_path_precondition_matches_get_card_names_semantics` (L100, 2 asserts)
* `multi_name_cards_are_covered` (L134, 1 assert)

**Behaviour:** the fast group-name path must agree with the allocating path for the **whole
corpus**, and the fast-path precondition must agree with `get_card_names` semantics.
**Reachability:** `X=group_names` **273/985 · 543 pairs · 1 574 effect nodes · 1 708 condition
nodes**; `K=group_names` (the condition surface) **155/985 · 270 pairs · 543 eff · 3 035 cond**.
**Divergence:** `A17`'s six `CardFilter` carriers (`groups[]`, `exclude_group_names`,
`cost_limit_min/max`, `original_heart_total`) have **0 producers** — `card_filter_from_effect`
(`src/core/card.c:957`) sets none — so the matchers at `src/ability/util.c:1527/1537/1552/1567/1584/1586/1701`
read a filter that can never carry them. **A C test written now would be testing dead code.**
Write it against `rb_card_matches_group_str` (118 755 pairs, zero disagreements) and against the
raw `group_names` CSV, and gate it on the producer landing.
**Target file:** `tests/test_parity_condition_and_effect.c` (currently one of the 52 suites with no
Rust citation — this task also fixes its mapping debt).

### 2.2 `max_distinct_names` → **23/985**

**Rust file:** `engine/tests/test_modules/rules/targeting/max_distinct_names_test.rs` — 3
`#[test]`, no card named:
* `dp_matches_brute_force` (L35, 2)
* `degenerate_inputs` (L71, 2)
* `cyclic_overlap_is_solved_exactly` (L82, 6)

**Behaviour:** the distinct-name cap solved by DP must equal brute force, must survive degenerate
inputs, and must solve cyclic overlap exactly.
**Reachability:** `K=distinct` **23/985 · 52 pairs · 111 eff · 741 cond**; `X=distinct` 19/985 ·
36 pairs · 66 eff · 252 cond; `A=move_cards` from hand **996 pairs / 430 abilities** overall.
**Divergence:** `tests/test_max_distinct_names.c` already exists (454 L) and emits **4 043 `ok:`
lines with 6 FAIL**, but it carries **no Rust citation** — it is one of the 52 uncited suites, so
its mapping is unwritten. **Its numbers cannot be checked against the oracle.** Add the header
citation and reconcile its 6 FAIL against the three Rust `#[test]`s before writing anything new.
**Target file:** `tests/test_max_distinct_names.c`.

### 2.3 `pvp_room_test.rs` → 14 tests, **1/985**

**Rust file:** `engine/tests/test_modules/integration/pvp_room_test.rs` — **14 `#[test]`,
346 lines, COMMITTED TO GIT, NEVER COMPILED.** `engine/build.rs` line ~86:
`const EXCLUDED_MODULES: &[&str] = &["pvp_room_test"];`
**Behaviour:** choice routing in a two-human (pvp) seat model, where `active_player` is not the
driver:
`rps_both_can_act` (L28), `rps_p1_chooses_then_cannot` (L35), `rps_both_choose_then_neither_can`
(L43), `choose_first_attacker_winner_only` (L52), `mulligan_first_attacker` (L60),
`mulligan_second_attacker` (L69), `main_phase_first_attacker_only` (L78),
`main_phase_second_attacker_only` (L87), `live_card_set_first_attacker` (L103),
`live_card_set_second_attacker` (L111), `opponent_choice_routed_via_choice_player_id_works_in_pvp`
(L126), `opponent_choice_via_select_auto_ability_works_in_pvp` (L149),
`normal_choice_stays_with_active_player_in_pvp` (L175),
`both_players_multiple_live_start_abilities_get_correct_choice_routing` (L204, **16 asserts**).
**Reachability:** 1/985 · 1 pair · 3 effect nodes (`PL!N-bp3-015-N` ab#0; `PL!-sd1-010-SD` carries
none). **Low corpus reachability, high structural value:** this is the only oracle covering
`choice_player_id` routing, and `src/core/game_state_abilities.c` has three separate readers of
"who owns this choice". Rank it in Wave 2 for the routing risk, not the corpus count.
**Divergence:** `PARITY_CHECKLIST.md` B4 records `rb_resolver_spawn_target` (`choice.c:286`) as
"wire or delete with Rust proof" with 0 callers — `opponent_choice_via_select_auto_ability_works_in_pvp`
is exactly the test that would settle it. Also B1 (`choice.c:4907` bare `cur++; n_entries = 0;`)
and B5 are in the same functions.
**Target file:** `tests/test_parity_full_round2.c` (which already carries 68 `FAIL` lines and is
one of the 9 suites the `^FAIL:` scan cannot see — fix 0.3 first).

### 2.4 `cost_modifier_expiry_sign` → **1/985**

**Rust file:** `engine/tests/test_modules/effects/cost_mod/per_card/cost_modifier_expiry_sign_test.rs`
— 3 `#[test]`, and its only card is `PL!HS-bp5-005-AR`, which **no C test file stages at all**
(the only `T4_NO_C_TRACE` engine-side file in the audit):
* `expiring_a_reduction_leaves_a_competing_increase_intact` (L33, 3)
* `expiring_an_increase_leaves_a_competing_reduction_intact` (L73, 2)
* `a_lone_modifier_reverts_to_nothing_in_both_directions` (L96, 2)

**Behaviour:** when a cost modifier expires, a **competing** modifier of the opposite sign must
survive, and a lone modifier must revert to nothing in **both** directions.
**Reachability:** 1/985 · 1 pair · 6 effect nodes · 6 condition nodes — but the *family* is
`A=modify_cost` 22/985 and `X=cost_limit` 76/985, so the sign logic is worth more than its single
node suggests.
**Divergence:** this is the behaviour the 10 vacuous cases in §1.4 cannot currently prove, because
their paired positives are RED. Land this **first** — it is the cheapest way to turn 10 vacuous
cases into proven ones.
**Target file:** `tests/test_parity_cost_discount.c`.

### 2.5 `card_type_filter` → 1/985

**Rust file:** `engine/tests/test_modules/characterization/card_type_filter_test.rs` — 3 `#[test]`:
`each_type_filter_matches_only_its_own_type` (L31), `type_filter_spellings_do_not_change_the_verdict`
(L77), `an_unrecognized_type_filter_matches_nothing` (L112).
**Behaviour:** a `card_type` filter matches only its own type; spelling variants do not change the
verdict; an unrecognised type matches nothing.
**Reachability:** `PL!-pb1-029-SRL` alone = 1/985 · 1 pair · 1 effect node · **19 condition nodes**.
`K=card_type` is **756 nodes corpus-wide** — so the *key* is huge even though this card is not.
**Divergence:** the C stages `PL!-pb1-029-SRL`, `PL!-pb2-001-R`, `PL!N-bp7-028-L` but asserts
nothing about them (tier T3). `card_type_field` is a fixed `char[24]` on `AbilityEffect`
(`rabuka.h:143`) — the C reads the struct field, not the wire key, so a spelling variant test is a
**C-specific** divergence, not a port.
**Target file:** `tests/test_parity_x_class.c`.

### 2.6 `niko_debut_draw_per_activated` → 2/985 — **UNTRACKED, NEW**

**Rust file:** `engine/tests/test_modules/effects/choice/selection_filter/niko_debut_draw_per_activated_parser_gap_test.rs`
— **brand-new untracked file**, 2 `#[test]`:
`niko_debut_draw_ability_has_no_activation_action` (L96, 3),
`niko_debut_draw_payout_per_unit_is_parsed` (L132, 4).
**Behaviour:** a parser gap — the 靠自己 draw-per-activated ability has **no `activation_action`**,
and its `per_unit` payout does not survive the parser.
**Reachability:** `PL!-pb2-018-P＋` = 2/985 · 2 pairs · 3 effect nodes · 13 condition nodes.
`X=per_unit` 155 nodes, `X=per_unit_type` 153 nodes corpus-wide.
**Divergence:** `test_parity_fw_cost_cardtype.c` stages `PL!-pb2-018-P＋` but asserts nothing about
it. Note this is the **same card** as the C3 landed item in `PARITY_CHECKLIST.md` — the
`distinct:"card_name"` hand cost dedupe (2 of 901 decoded cost nodes) lives on
`PL!-pb2-018-P＋`/`-R＋`. **One card, two C test homes needed.**
**Target file:** `tests/test_parity_fw_cost_cardtype.c` (per-unit parse) **and**
`tests/test_parity_jidou_extra.c` (the cost-dedupe home the C3 note names).

### 2.7 `self_to_waitroom_liella_recovery` → **0/985**

**Rust file:** `engine/tests/test_modules/effects/recover/to_hand/self_to_waitroom_liella_recovery_test.rs`
— 2 `#[test]`: `sp_bp4_018_activation_self_to_waitroom_recovers_liella_card` (L9, 4),
`sp_bp4_018_self_only_liella_pool_returns_self_to_hand` (L42, 3).
**Reachability:** its two cards, `PL!S-bp4-018-N` and `PL!S-pb1-001-PR`, carry **0 abilities in the
corpus** — i.e. **the test's fixtures are cards that do nothing**. That is itself the finding: the
Rust test uses generic members to exercise a recovery path, so reachability must be measured on
the *behaviour* (`A=move_cards` to hand: **430/985**), not on the card. The C stages both cards and
asserts nothing about them (tier T3).
**Divergence:** `A=move_cards` is 430/985 — the single largest action family. Any regression in the
hand-recovery path is corpus-wide. Priority is **high despite 0 card reachability.**
**Target file:** `tests/test_parity_compound_per_card.c` (64 `FAIL` lines, exit 0 — see 0.4).

### 2.8 `card_database_parity` → 2/985

**Rust file:** `engine/tests/test_modules/characterization/card_database_parity_test.rs` — 2
`#[test]`: `every_pl_card_in_cards_json_resolves_in_the_baked_database` (L31),
`the_database_carries_no_cards_that_cards_json_dropped` (L77).
**Reachability:** `PL!N-bp1-002-R` = 2/985 · 2 pairs · 7 effect nodes.
**Divergence:** `K7` is live — `cards/build/abilities_strings.bin` is **726 649 B / `6225b9b5…`**
against the canonical **726 831 B / `89204df0…`**, and `OFFSET_DELTAS` disagrees with
`RBKA_OFFSET_DELTAS` at **2 of 985** slices (**325** and **805**). This task is a blob-identity
assertion, so it is the natural place to pin the 726 649-vs-726 831 skew loudly rather than
quietly.
**Target file:** `tests/test_artifacts_consistent.c` (already prints
`abilities=985 interned_strings=6032 table_bytes=726831 … all digests match`; add the
`cards/build` size/digest comparison it currently omits).

### 2.9 `run_all.rs` invariants → 0/985 (invariant, not coverage)

**Rust file:** `engine/tests/run_all.rs` — 3 `#[test]`: `test_parse_heart_color` (L4, **10 asserts**),
`test_no_custom_actions` (L59, 1), `scenario_oracle` (L99, 2).
**Behaviour:** lenient heart-colour parsing (unknown → Heart00); the corpus contains no
`action:"custom"`; the oracle scenario.
**Divergence:** `test_parity_enums.c` (650 L) already round-trips every enum wire table and prints
`7199 assertions, 0 failures` — but it carries **no Rust citation** and prints **zero `ok:` lines**,
so its 7199 assertions are invisible to a `stdout` scan. Add the citation and the `ok:` lines.
**Target file:** `tests/test_parity_enums.c`.

---

# WAVE 3 — the phantom-suite and uncited-suite mapping debt

### 3.1 Create, or delete the claim, for each of the 30 phantom names

**Full list:** `COVERAGE_HARNESS.md` §6. Grouped by what the fix is:

**(a) rename-only phantoms — the real file exists under a different name (7).** Write the correct
name into the documents; **do not** create a file.

| phantom name in circulation | the file that actually exists |
|---|---|
| `parity_vm` | `tests/test_vm_decode.c` |
| `parity_resolver` | *(none — see 3.2)* |
| `parity_max_distinct_names` | `tests/test_max_distinct_names.c` |
| `parity_survey_probe` | `tests/test_survey_probe.c` |
| `parity_tp_joint` | `tests/test_tp_joint_card_live_start.c` |
| `parity_tp_zone_change_gate` | `tests/test_tp_zone_change_gate.c` |
| `parity_condition_decode` | `tests/test_parity_t7_condition_decoder.c` |
| `parity_x_class` *(inverted prefix)* | `tests/test_parity_x_class.c` |

**(b) VERIFIED-NOOP phantoms — the behaviour does not exist in the corpus (2).** Record the
measurement and close; **writing a test would be writing a test for nothing.**

| phantom name | measurement |
|---|---|
| `parity_dynamic_count` | `X=dynamic_count` = **0 nodes** corpus-wide (consistent with F4's `tools/census_fw.c` finding of 0 `selected_card_score` nodes) |
| `parity_nazuna_at_deck_top` | `PL!N-bp7-028-L` **is** ability **805** — one of the two K7-skewed slices. See 3.3. |

**(c) real coverage holes — the suite was promised and never written (21).** Each needs a new
file, or the claim withdrawn. Ranked by reachability:

| phantom name | what it was supposed to cover | n/985 |
|---|---|---:|
| `parity_card_filters` | A17's six `CardFilter` carriers — matcher landed, **producer 0** | **273** (`X=group_names`) |
| `parity_effect_conditions` | `conditional_on_result` / `_on_optional` / `conditional_alternative` | 23 / 11 / 8 |
| `parity_draw_top` | `src=deck_top` | **119** |
| `parity_lookup` / `parity_watchers` | `watches_area_move` | **17** |
| `parity_card_text` | `game/display.rs` — **VERIFIED-NOOP, 0/985** (T5-1: there is no display layer in C at all) | 0 |
| `audit_modifiers` | `src/core/modifiers.c` — the twin of `tools/census_e.c`; cluster E has `test_parity_cluster_e.c` instead | mixed |
| `audit_cards` / `audit_game_state` | `src/core/card.c` / `game_state_abilities.c` | mixed |
| `audit_placeholders` / `audit_vs_official` / `audit_field_coverage` / `audit_id_in_assert` / `audit_fixture_annotations` / `audit_emitted_keys` / `audit_x_const` / `audit_x_lookat` / `audit_cluster_d` / `audit_w_gate` / `audit_t7_*` | **these are TOOL names, not suite names** (`tools/audit_*.c`) — the documents conflate the tool graph with the suite graph | n/a |
| `audit_additive_modifier_stacks_on_base` / `audit_allocations_match_filled` / `audit_both_players_fail_no_winner` / `audit_p1_passes_p2_fails_p1_wins` / `audit_set_zero_removes_requirement` / `audit_sufficient_hearts_passes` / `audit_two_cards_second_fails` / `audit_zero_stage_live_card_must_fail` | **CASE names** from `TEST_PARITY_SURVEY.md`, not suite names — they read as suites because the survey lists them bare | see case-level reachability |

**Distinguishing tool names from suite names is itself a task:** `tools/` currently holds 10
`audit_*.c` files (`audit_actions`, `audit_cluster_d`, `audit_t7_decoders`, `audit_t7_resource`,
`audit_t7_wire`, `audit_t7_zeroheart`, `audit_w_gate`, `audit_t7_*`) and every document that
mentions an `audit_*` name is ambiguous between the two graphs.

### 3.2 Give the 52 uncited C suites a header citation

**52 of 159 suites carry no Rust-test citation at all** (full list with sizes:
`COVERAGE_MAP.md` §2.4). The largest are `test_parity_zones_position.c` (1 061 L, 194 msgs),
`test_parity_look_reveal.c` (1 151 L, 158), `test_parity_queue_resume.c` (924 L, 154),
`test_parity_deck_order.c` (773 L, 133), `test_parity_executor_dispatch.c` (1 217 L, 121),
`test_audit_triggers.c` (1 386 L, 119), `replay.c` (1 285 L, 115), `test_audit_zones.c` (1 496 L,
103), `test_audit_fields.c` (1 246 L, 73).

**Why this is Wave 3 and not later:** a Rust test file can hold up to **37** `#[test]`s, and a T0
citation claims the whole **file**. `effects/compound/per_card/position_change_draw_and_rotation_multi_card_fixes_test.rs`
alone has 37. **3 730 of the 4 319 mapped tests inherit a file-level claim with no per-test
manifest** — that is the single largest unmeasured quantity in the audit, and only a per-file
manifest closes it. Format: one `*  <rust path>::<fn>  -> <c case fn>` line per ported test.

### 3.3 ⚠ Flag every test that depends on ability 325 or 805 (K7)

`PARITY_CHECKLIST.md` K7: `cards/build/abilities_strings.bin` is **726 649 B / `6225b9b5…`** against
the canonical **726 831 B / `89204df0…`**, and Rust's `OFFSET_DELTAS` disagrees with C's
`RBKA_OFFSET_DELTAS` at **2 of 985** slices. **For these two the C may be correct and any
Rust-derived expectation is suspect.**

| ability | cards | C tests that depend on it |
|---:|---|---|
| **325** | `PL!-pb2-012-P＋` (ab#1), `PL!-pb2-012-R` (ab#1) — use_limit 1, cost `sequential_cost`, effect `move_cards` hand→discard filtered on `group_names = Printemps` | **`tests/test_parity_wave1b.c`** §C (`:2396`, `:2661`; header `:18`) |
| **805** | `PL!N-bp7-028-L` (ab#0) "Cooking with Love" — effect with `conditional_action` + `optional_action` + `heart_colors` + `shuffle` + `group_names`, no cost | **`tests/test_parity_hearts_live_start.c`** at **4 sites**: `:889-890`, `:912-913`, `:941-942`, `:973-974` |

**Action:** add a `// K7: ability 805 — RBKA_OFFSET_DELTAS vs OFFSET_DELTAS disagree; the C may be
correct here.` banner to each of the 5 sites, and make `test_describe_golden.c`'s
`EXCLUDED_SKEW` set assert that it is **exactly {325, 805}** (it already does — keep that, and note
it is the only test in the corpus that knows about the skew).
**Reachability:** 1/985 each, so the corpus exposure is small; the risk is entirely in the tests.

### 3.4 The `src/` files Rust tests that the C does cover — say so in the C header

`src/core/constants.rs` (3 `#[test]`) is fully covered by `test_describe_golden.c` (all 5 constants,
all 10 functions). `src/game/deck_ordering.rs` (5) is covered by `test_parity_deck_order.c`.
Both currently count as **uncited Rust-only infrastructure**, which is wrong in the direction that
understates the C. Add the citations.

---

# WAVE 4 — cheap, low reachability, do last

| task | Rust test | n/985 | target |
|---|---|---:|---|
| `set_heart_type` multi-target | **`effects/hearts/set_heart_type_multi_target_test.rs` DOES NOT EXIST** — the generated `mod.rs` declares it, git has never had it | 9 | recreate the Rust oracle first, then port |
| `pbl02_card_table_test` | **`expansion/pbl02_card_table_test.rs` DOES NOT EXIST** | — | same |
| `bounded_fan_cost_equality_gate` | **`ability_gates/bounded_fan_cost_equality_gate_test.rs` DOES NOT EXIST** | — | same |
| `count_as_multiple` weight (E10) | `rb_mods_count_as_multiple_weight` | 1 | `tests/test_parity_cluster_e.c` already has `test_e10_count_as_multiple_weight` (13 asserts) — ✅ covered |
| `replacement_appear_wait` (E12) | `rb_mods_replacement_appear_wait_applies` | 1 (5 with `heart_total_limit`) | `tests/test_parity_cluster_e.c` has `test_e12_replacement_appear_wait` (16 asserts) — ✅ covered |
| `non_stackable` dedupe (E1) | `evaluate_success_zone_constant_modifiers` | 3 | **VERIFIED-INERT**: all 3 carry distinct `action:text` keys so the dedupe can never fire — record, do not test |

---

## APPENDIX — every number in this file, and where it came from

| number | how measured |
|---|---|
| 4 319 Rust tests; 899 files; 4 175 + 144 split | `#[test]` walk over `engine/tests/**/*.rs` + `engine/src/**/*.rs`, brace-balanced per-function assertion census |
| 4 154 / 4 319 have a C counterpart | 3 730 T0 (784 files cited) + 424 T1 (card named in a C assertion message) |
| 1 236 WEAKENS (floor) | 3 mechanical rules over the 4 154 T0∪T1 pairs; `COVERAGE_MAP.md` §4.1 |
| 21 NO C TEST; 23 VACUOUS cases; 2 INVERTED | §2, §4, §1.1, §1.2 above |
| every `n/985` | decode-level walk of 2 116 pairs with `rb_decode_card_ability`, deduped by printed-ability text to 985; self-check reproduces 2 639 / 985 / 2 116 / 4 925 / 1 589 exactly, and `test_artifacts_consistent` independently reproduces 985 / 6 032 / 726 831 |
| 159 suites; 3 486 FAIL; 44 `^VACUOUS`; 2 `SETUP BUG`; 0 `PASSED` | 159-suite strictly-sequential run in `Temp/kilo/cov/tree`, canonical blob verified before and after, `make data` never run |

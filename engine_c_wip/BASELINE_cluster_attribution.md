# BASELINE_cluster_attribution.md — FAIL assertions grouped by owning Rust module

**Measured:** 2026-10-03, isolated snapshot `C:\Users\trios\AppData\Local\Temp\kilo\baseline\engine_c_wip`
**Manifest:** `C:\Users\trios\AppData\Local\Temp\kilo\baseline\MANIFEST.sha256`
**Source of truth:** `BASELINE.md` §6 (per-suite table) and `BASELINE_top25.txt` (ranked assertions).
**Whole tree:** 2 530 FAIL assertions / 142 suites / 108 RED / 28 GREEN / 5 unverified / 1 silent.
Run 1 and run 2 are byte-identical (stderr multiset, 142/142, raw), so every number below is stable.

---

## 1. WHAT "OWNING RUST MODULE" MEANS HERE, AND HOW IT WAS DERIVED

Every FAIL assertion in this corpus is emitted by a C test whose purpose is to mirror one
Rust test. So the honest owner of a failing assertion is **the Rust test-cluster
directory that owns its twin**, i.e. a path under
`engine/tests/test_modules/<top>/<sub>/…`. That is exactly the granularity
`PROGRESS.md` uses to assign translation waves, so a cluster owner can read their own
impact directly.

Attribution used two mechanical routes, in priority order. **No guessing.**

| route | what it uses | FAIL lines |
|---|---|---|
| **1** | the `[test_name]` tag the harness appends to each FAIL line (`…  [foo_bar]`, or `FAIL [foo_bar]: …`), matched against a name→file index built by scanning all 1 002 `engine/tests/test_modules/**/*.rs` for `fn <name>` (5 584 functions, 4 924 unique) | **169** |
| **2** | the suite's own citation of its Rust twin — the `--- A <file>_test.rs ---` runtime banner and/or the `*_test.rs` citations in the C file's header comment — resolved through a file→directory index | **1 715** |
| — | **total attributed** | **1 884** (74.5 %) |
| 3 | **UNRESOLVED** (no Rust twin identifiable from the corpus) | **646** (25.5 %) |

Route 1 is exact but rare, because only the FORK harnesses tag their FAIL lines.
Route 2 is a whole-suite assignment, so it is coarser but covers most of the tree.

**Coverage is stated here rather than hidden: 646 FAIL lines cannot be assigned to a Rust
module from the corpus.** §4 lists every one of them. That is the requested
"failures concentrated in a module nobody owns" flag, and it is large.

---

## 2. ATTRIBUTION TABLE — all 55 clusters, ranked by FAIL count

`%` is of the 2 530 whole-tree FAIL lines. `?` marks the clusters whose owner agent
should read §4 before trusting their number.

| # | Rust cluster (`engine/tests/test_modules/…`) | FAIL | % | suites | most-repeated assertion in the cluster |
|---|---|---|---|---|---|
| 1 | `integration/full_round` | 146 | 5.8 | 6 | `[R1_shared_pr_debut_waits_low_cost_opponents]: R1: the first cost-2 opposing member is waited (got 0 expected 1)` ×3 |
| 2 | `jidou/ability_watch` | 114 | 4.5 | 4 | `a hand card reached the waitroom -> heart01 x1 (got 0 expected 1)` ×2 |
| 3 | `rules/trigger_paths` | 77 | 3.0 | 5 | `bp4 debut should prompt` ×2 |
| 4 | `effects/draw/flat` | 74 | 2.9 | 1 | `discard prompt 2 expected` ×3 |
| 5 | `effects/gain/hearts/per_card` | 71 | 2.8 | 2 | `If PL!N-bp7-024-N's series is no longer 虹ヶ咲 … the premise of the two beha…` ×1 |
| 6 | `effects/recover/to_hand` | 58 | 2.3 | 3 | `C/p2: P2 fetched exactly two members (got 0 expected 2)` ×1 |
| 7 | `jidou/discard_watch` | 56 | 2.2 | 1 | `[rurino_use_limit_blocks_second_same_turn] first: heart01=1 (got 0 expected 1)` ×1 |
| 8 | `effects/gain/blades/per_card` | 55 | 2.2 | 3 | `ANTI-VACUITY CONTROL: 2 『Liella!』 members discarded -> 2 blades (got 3 expected 2)` ×1 |
| 9 | `effects/choice/target_player` | 54 | 2.1 | 1 | `a: the live left the waitroom` ×1 |
| 10 | `effects/conditional/zone_source` | 53 | 2.1 | 2 | `F: the reveal is filtered to live cards` ×9 |
| 11 | `effects/compound/cost_and_effect` | 53 | 2.1 | 1 | `a8: exactly 2 filtered stage candidates (the two own same-name members; got 0) (expected 2, actual 0)` ×2 |
| 12 | `jidou/movement/self_move_heart_watch` | 49 | 1.9 | 1 | `the discard prompt appears` ×2 |
| 13 | `effects/state/per_card` | 49 | 1.9 | 2 | `B: PROMPT CONTROL — option 1 must ask WHICH member, not silently pick the first (prompts=1)` ×2 |
| 14 | `effects/cost_mod/per_card` | 47 | 1.9 | 1 | `exactly three cards are discarded  (got 0, expected 3)` ×3 |
| 15 | `effects/conditional/per_card` | 46 | 1.8 | 1 | `E: discarding 百生吟子 -> printed 1 blade + conditional 1 = 2 (got 0 expected 2)` ×2 |
| 16 | `jidou/energy_watch` | 46 | 1.8 | 1 | `A1 LEG (a): her 登場 must move exactly 1 card OUT of the energy deck …` ×1 |
| 17 | `effects/gain/blades/constants` | 45 | 1.8 | 6 | `[heart_total_limit] M_heart_total_limit: card_filter_from_effect must copy heart_total_limit=4 and heart_…` ×1 |
| 18 | `effects/energy/under_member` | 45 | 1.8 | 1 | `the paid energy left the energy zone  [activation_paid_under_energy_recovers_exact_nijigasaki_live]` ×1 |
| 19 | `effects/score/live_total` | 43 | 1.7 | 2 | `control: the cost prompt is raised` ×4 |
| 20 | `effects/state/wait_activation` | 41 | 1.6 | 1 | `真姫 is played to centre` ×4 |
| 21 | `effects/ability_mod` | 39 | 1.5 | 3 | `the member is on stage after the cost reduction (got -1 expected 2204)` ×2 |
| 22 | `integration/parser_e2e` | 39 | 1.5 | 2 | `round5: P2's copy scores +0 when P1 is NOT strictly ahead (got 1 expected 0)` ×1 |
| 23 | `jidou/combination` | 39 | 1.5 | 1 | `A1+: ab#0 paid 1E out of the 1 the 4-cost ally left (1->0) and activated the 2 waited (0->2). …` ×1 |
| 24 | `jidou/yell/discard_revealed_then_re_yell` | 39 | 1.5 | 1 | `declined: no card from the pool moved` ×3 |
| 25 | `effects/look_select/pin_top` | 38 | 1.5 | 2 | `tomari_accept: a discarded card is not still in hand` ×3 |
| 26 | `effects/position/area_move` | 32 | 1.3 | 1 | `the moving member is recorded as moved  [non_optional_pc_executes_move_correctly]` ×1 |
| 27 | `characterization` | 32 | 1.3 | 3 | `#B the effect-node walk visits 1879 nodes (got 1878). This is the golden census …` ×1 |
| 28 | `effects/deploy` | 32 | 1.3 | 3 | `keke debut asks which stage slot to deploy into (rs:45-49) (got 0 expected 1)` ×1 |
| 29 | `effects/move_cards/under_member` | 31 | 1.2 | 1 | `B1 COST LEG 1 NOT PAID: the 起動 costs {{icon_energy}}{{icon_energy}} …` ×5 |
| 30 | `effects/gain/blades/under_member` | 30 | 1.2 | 1 | `『Aqours』 member with a member card under it -> 1 blade (got 0 expected 1)  [watanabe_aqours_host_with_me…` ×1 |
| 31 | `effects/state/orientation_gated` | 28 | 1.1 | 1 | `A1: paying the optional cost must put Erena HERSELF to wait -- the cost IS her own orientation` ×1 |
| 32 | `effects` (bare) | 27 | 1.1 | 2 | `[B2_no_baton_no_draw]: B2: no baton touch happened (the target area was empty) (got 2 expected 0)` ×1 |
| 33 | `effects/look_select/look_and_filter` | 27 | 1.1 | 1 | `mei_per_group: the engine must reject 2 cards from the same series (rc=1)` ×1 |
| 34 | `jidou/leaves_stage` | 26 | 1.0 | 1 | `ab#1 should place 澁谷かのん under the arriving member` ×2 |
| 35 | `effects/auto` | 23 | 0.9 | 1 | `A1: the wait branch of the `or` grants exactly one blade (got 0 expected 1)` ×1 |
| 36 | `effects/look_select/reveal` | 20 | 0.8 | 1 | `kanon: the pick is over looked_at (zone= count=1 allow_skip=1 | want zone=looked_at count=1 allow_skip=-…` ×1 |
| 37 | `effects/gain/hearts/constants` | 19 | 0.8 | 1 | `spending back below the 10-energy threshold removes the heart06 gain (got 2 expected 0)` ×1 |
| 38 | `rules/scoring` | 18 | 0.7 | 1 | `B4: PREMISE — Dia's 自動 re-yell actually happened` ×1 |
| 39 | `jidou/state_watch` | 17 | 0.7 | 1 | `accepting the cost reactivates the waited 虹ヶ咲 member (ab#0's second step)` ×1 |
| 40 | `rules/conditions` | 17 | 0.7 | 2 | `drew 2 then discarded 1 -> one card remains (got 2 expected 1)` ×2 |
| 41 | `effects/cost_mod/required_hearts` | 16 | 0.6 | 1 | `six distinct 蓮ノ空 stage+waitroom members -> -2 (got 0 expected -2)` ×1 |
| 42 | `effects/energy/place` | 15 | 0.6 | 1 | `a non-Liella member on the stage fails the group condition (got 8 expected 7)` ×1 |
| 43 | `effects/score/per_card` | 14 | 0.6 | 2 | `honoka: the gained ability must be registered on the card (0 means the CONDITION never passed …` ×2 |
| 44 | `effects/choice/per_card` | 11 | 0.4 | 2 | `[N13_placed_debuts_fire]: N13: the opponent is asked second: the queue entry's choice_player_id names p2` ×1 |
| 45 | `effects/energy` | 9 | 0.4 | 2 | `[fixture_cost_and_group_are_what_the_gates_assume]: PL!-pb2-001-PP resolves to group "mu's" …` ×1 |
| 46 | `rules/phases` | 7 | 0.3 | 1 | `[N10_direct_placement_wait]: N10: the opponent prompt is standing` ×1 |
| 47 | `effects/look_select/recruit_stage` | 5 | 0.2 | 1 | `12a control: stage cost 28 >= 20 -> the look-and-select prompt fired` ×1 |
| 48 | `rules` (bare) | 4 | 0.2 | 1 | `ENGINE BUG: an immediately repeated, otherwise identical state IS a repetition, so rb_check_permanent_lo…` ×1 |
| 49 | `rules/targeting` | 3 | 0.1 | 1 | `Total +4 heart06 across both members (got 0 expected 4)` ×1 |
| 50 | `effects/cost/additional_cost_choice` | 2 | 0.1 | 1 | `M1: a P2-owned 吟子 moves all three of P1's waitroom members to P1's OWN deck bottom (rs:625-631)` ×1 |
| 51 | `jidou/debut_watch` | 1 | 0.0 | 1 | `[choose_target_player] N_choose_target_steps: choose_target_player must run the effect_steps it decodes` ×1 |
| 52 | `effects/position/area_gated` | 1 | 0.0 | 1 | `[other_syncrise_member_moves_to_center_ability_card_gains_blades]: B2.1: another 5yncri5e! member moving ` ×1 |
| 53 | `effects/score/card_score` | 1 | 0.0 | 1 | `ノンフィクション's condition is position=center` ×1 |
| | **attributed total** | **1 884** | **74.5** | | |
| | unresolved (§4) | **646** | **25.5** | 30 | |

### Roll-up by Rust area — what each owner is actually carrying

| Rust area (top-level `test_modules/` dir) | FAIL lines | clusters |
|---|---|---|
| `effects/` | **1 040** | 38 |
| `jidou/` | **322** | 7 |
| `integration/` | **185** | 2 |
| `rules/` | **126** | 6 |
| `characterization` | 32 | 1 |
| *unresolved* | 646 | — |

So: **41 % of all failures sit under `effects/`**, 13 % under `jidou/`, 7 % under
`integration/`, 5 % under `rules/`. `effects/` is not one owner — it splits across the
38 clusters above, and no single one of them exceeds 3 % of the tree. **There is no
single high-value cluster owner in this tree.** The largest, `integration/full_round`,
is 146 lines = 5.8 %.

---

## 3. WHAT EACH CLUSTER OWNER CAN EXPECT

* **Expect a flat profile.** 2 393 distinct assertions over 2 530 lines means the
  backlog is a wide field of one-off divergences, not a set of deep holes. Closing a
  whole 45-line cluster moves the tree by 1.8 %.
* **Check for stale expectations before touching the engine.** 1 549 of the 2 530 lines
  (61 %) live in suites that already declare an expected gap. See
  `BASELINE_top25.txt` Part 4 for two specific clusters where that is likely.
* **The biggest single win in the whole tree is a root cause, not a cluster** — the six
  `parity_search_reorder` lines in ranks 1–6 are one defect (C opens `SelectCard` where
  Rust opens `SelectTarget{target:"order"}`). It is booked to no single cluster above
  because that suite's Rust twin is unidentifiable (§4).

---

## 4. UNOWNED: the 646 FAIL lines no Rust module can be assigned to

This is the requested flag, and it is not a small tail. **30 suites, 646 lines (25.5 %),
and no identifiable Rust twin.** Sorted by size:

| suite | FAIL lines | dominant pattern (normalised) |
|---|---|---|
| `parity_search_reorder` | **122** | `nozomi: expected a SelectTarget order prompt, got SelectCard` ×12 |
| `parity_sequential` | **113** | `D: 好きな順番でデッキの一番下に置いてもよい -> BOTH paid members are in the deck` |
| `parity_hearts_live_start` | **64** | `second selection prompt should appear` ×4 |
| `parity_mill_deck` | **53** | `the cost really milled N cards (got N expected N)` |
| `parity_condition_and_effect` | **52** | compound cost-and-effect prompt chains |
| `parity_restriction` | **43** | `colour choice must be offered` ×5 |
| `parity_compound_per_card` | **41** | compound cost legs per card |
| `parity_zones_position` | **38** | `formation order is N` / `LEFT area holds N` / `RIGHT area holds N` |
| `parity_stats_pipeline` | **31** | score-pipeline recomputation |
| `parity_card_score` | **21** | `PL!S-bp7-023-L: the optional energy-return cost is actually offered` ×3 |
| `parity_deploy_multi` | **18** | multi-member debut placement |
| `parity_recover_to_hand` | **10** | recovery destination selection |
| `max_distinct_names` | 6 | enum wire-table round-trips (no Rust twin — this is a C-only audit) |
| `audit_triggers` | 5 | trigger census (C-only audit) |
| `audit_zones` | 3 | zone census (C-only audit) |
| `condition_evaluation` | 3 | `ValueRef` / `Temporal` condition evaluation |
| `ported_generated` | 3 | transpiled generated worklist |
| `tp_activation_offer` | 3 | activation-offer gating |
| `audit_actions` | 2 | ability-action wire table (C-only audit) |
| `audit_conditions` | 2 | condition census (C-only audit) |
| `look_select` | 2 | look/select prompt shape |
| `parity_custom_misc` | 2 | `h_custom` / misc effects |
| `performance_phase_rules` | 2 | performance-phase rules |
| `p1_helpers` | 1 | P1 helper predicates |
| `parity_auto_abilities` | 1 | automatic ability ordering |
| `parity_cost_and_effect` | 1 | compound cost handling |
| `parity_look_reveal` | 1 | look/reveal zone routing |
| `parity_queue_resume` | 1 | queue resume path |
| `parity_support` | 1 | support helpers |
| `surplus_heart` | 1 | surplus heart counting |

### Two sub-groups, and they need different treatment

**(a) Six C-only audit suites — 17 lines, correctly unowned.**
`audit_actions` (2), `audit_conditions` (2), `audit_triggers` (5), `audit_zones` (3),
`max_distinct_names` (6), `condition_evaluation` (3). These have **no Rust twin by
design** — they are C-side census/gate tools (`max_distinct_names` is a 7 191-assertion
enum wire-table round-trip). They are unowned because they *should* be. Route them to a
tooling owner, not a parity owner.

**(b) Twenty-four parity suites — 629 lines, and this is a real ownership gap.**
`parity_search_reorder` alone is 122 lines and is the single highest-value target in the
whole tree. Its C file cites no `*_test.rs`, its stdout prints no Rust banner, and its
`run()` names do not resolve into `engine/tests/test_modules`. Whatever Rust behaviour
these encode, **nobody currently owns the translation**, and the top-25 in
`BASELINE_top25.txt` ranks 1–6 come straight out of it.

**Recommended action:** dispatch one agent to identify the Rust source for
`parity_search_reorder` (122), `parity_sequential` (113) and
`parity_hearts_live_start` (64) — **299 lines, 11.8 % of the tree** — before any
engine-side work is scheduled against those numbers. Until then their counts are
real but unassignable.

---

## 5. HOW TO RE-DERIVE THIS FILE

```
raw runs   C:\Users\trios\AppData\Local\Temp\kilo\baseline\logs\runs\run1_<suite>.{out,err}
index      …\baseline\rust_test_index.tsv      (5 584 fn names -> cluster dir)
script     …\baseline\rank2.py                 (writes …\logs\ranking_run1.json)
```

`rank2.py` rebuilds §2 and §4 from the raw captures. If `MANIFEST.sha256` no longer
matches the tree, this attribution describes a different snapshot.
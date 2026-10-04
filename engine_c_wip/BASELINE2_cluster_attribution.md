# BASELINE2_cluster_attribution.md — FAIL assertions grouped by owning Rust module

**Measured:** 2026-10-04, isolated snapshot `C:\Users\trios\AppData\Local\Temp\kilo\base2\engine_c_wip`
**Manifest:** `C:\Users\trios\AppData\Local\Temp\kilo\base2\MANIFEST.sha256` (265 entries)
**Index built over:** `engine/tests/test_modules/**/*.rs` — 1 007 `.rs` files, 5 646 `fn` declarations, 4 984 unique names
**Whole tree:** 2 466 comparable FAIL assertions / 155 suites / 114 RED / 29 GREEN / 12 unverified / 0 silent.
Run 1 and run 2 agree EXACTLY (155/155 on both streams, raw), so every number here is stable.

Method is IDENTICAL to `BASELINE_cluster_attribution.md` so the two tables are directly
comparable. Two routes, no guessing:

| route | what it uses | FAIL lines |
|---|---|---|
| **1** | the `[test_name]` tag a harness appends to a FAIL line, matched against a name→file index of every `fn` in `test_modules/**/*.rs` | **125** |
| **2** | the suite's own citation of its Rust twin — the `--- A <file>_test.rs ---` runtime banner and/or the `*_test.rs` names in the C file's header — resolved through a file→directory index | **1484** |
| — | **total attributed** | **1609** (65.2 %) |
| 3 | **UNRESOLVED** (no Rust twin identifiable from the corpus) | **857** (34.8 %) |

Prior measurement, same method: 1 884 of 2 530 attributed (74.5 %%), 646 unresolved (25.5 %%).
**Coverage went DOWN from 74.5 pct to 65.2 pct and the unresolved bucket went UP from 25.5 pct to 34.8 pct** — the
13 new suites are all unresolved-by-construction, and `parity_search_reorder` alone now
contributes 112 unresolved lines with a newly-written Rust-cited assertion block that the
`--- A ---` banner route does not reach.

---

## 1. ATTRIBUTION TABLE — all clusters, ranked, with the movement against BASELINE

| # | Rust cluster (`engine/tests/test_modules/…`) | BASELINE | NOW | delta | %% of tree | suites | most-repeated assertion in the cluster |
|---|---|---|---|---|---|---|---|
| 1 | `UNRESOLVED` | 854 | **857** | +3 | 34.8 | 43 | `FAIL: nozomi: the mandatory order prompt is open` ×12 |
| 2 | `integration/full_round` | 165 | **166** | +1 | 6.7 | 6 | `FAIL [NN_both_two_eligible]: NN: prompt N is standing` ×3 |
| 3 | `jidou/ability_watch` | 89 | **88** | -1 | 3.6 | 3 | `FAIL: BN round N: the grant must not accumulate as scans repeat  (got N, expected N)` ×5 |
| 4 | `rules/trigger_paths` | 77 | **77** | +0 | 3.1 | 5 | `FAIL: HeartN from kotori (got N expected N)` ×3 |
| 5 | `jidou/energy_watch` | 71 | **71** | +0 | 2.9 | 2 | `FAIL: a hand card reached the waitroom -> heartN xN (got N expected N)` ×2 |
| 6 | `effects/gain/hearts/per_card` | 71 | **65** | -6 | 2.6 | 2 | `FAIL: If PL!N-bpNN-N's series is no longer 虹ヶ咲 (read "ラブライブ！虹ヶ咲学園スクールアイドル同好会"), the premise of the two behavio` ×1 |
| 7 | `effects/draw/flat` | 74 | **57** | -17 | 2.3 | 1 | `FAIL: discard prompt N expected` ×5 |
| 8 | `effects/recover/to_hand` | 58 | **55** | -3 | 2.2 | 3 | `FAIL: EN: the cost prompt must be a SelectCard (got "")` ×2 |
| 9 | `jidou/debut_watch` | 56 | **54** | -2 | 2.2 | 1 | `FAIL: [hand_discard_batch_qN_two_discarded_auto_fires_once] the 登場 must have offered its 「好きな枚数」 hand select` ×1 |
| 10 | `effects/gain/blades/per_card` | 55 | **53** | -2 | 2.1 | 3 | `FAIL: heartN is printed -> +N (got N expected N)` ×4 |
| 11 | `effects/compound/cost_and_effect` | 53 | **51** | -2 | 2.1 | 1 | `FAIL: aN: heartN is granted (expected N, actual N)` ×3 |
| 12 | `effects/state/per_card` | 49 | **48** | -1 | 1.9 | 2 | `FAIL: B: PROMPT CONTROL — option N must ask WHICH member, not silently pick the first (prompts=N)` ×2 |
| 13 | `effects/choice/target_player` | 54 | **48** | -6 | 1.9 | 1 | `FAIL: cN: the LiveStart choice is offered` ×2 |
| 14 | `effects/cost_mod/per_card` | 47 | **47** | +0 | 1.9 | 1 | `FAIL: exactly three cards are discarded  (got N, expected N)` ×3 |
| 15 | `jidou/movement/self_move_heart_watch` | 49 | **47** | -2 | 1.9 | 1 | `FAIL: an area move grants this card heartN xN (got N expected N)` ×2 |
| 16 | `effects/energy/under_member` | 45 | **45** | +0 | 1.8 | 1 | `FAIL: the paid energy left the energy zone  [activation_paid_under_energy_recovers_exact_nijigasaki_live]` ×1 |
| 17 | `effects/conditional/zone_source` | 53 | **44** | -9 | 1.8 | 2 | `FAIL: F: CROSSROADS is in the waitroom` ×5 |
| 18 | `effects/conditional/per_card` | 46 | **44** | -2 | 1.8 | 1 | `FAIL: E: discarding 百生吟子 -> printed N blade + conditional N = N (got N expected N)` ×2 |
| 19 | `effects/ability_mod` | 39 | **39** | +0 | 1.6 | 3 | `FAIL: success_count=N: up to TWO 『Aqours』 members each gain live-total +N (got N expected N)` ×2 |
| 20 | `jidou/yell/no_blade_heart_reveal_gain` | 39 | **39** | +0 | 1.6 | 1 | `FAIL: declined: no card from the pool moved` ×3 |
| 21 | `integration/parser_e2e` | 39 | **38** | -1 | 1.5 | 2 | `FAIL: roundN: PN's copy scores +N when PN is NOT strictly ahead (got N expected N)` ×1 |
| 22 | `effects/position/area_move` | 32 | **31** | -1 | 1.3 | 1 | `FAIL: the moving member is recorded as moved  [non_optional_pc_executes_move_correctly]` ×1 |
| 23 | `effects/gain/blades/under_member` | 31 | **31** | +0 | 1.3 | 2 | `FAIL: modifier read (direct): しずく's heartN modifier copies かすみ's N (got N expected N)  [shizuku_heart_copy_use` ×2 |
| 24 | `characterization` | 32 | **31** | -1 | 1.3 | 3 | `FAIL: §K prN_costN: a costN baton must NOT draw` ×3 |
| 25 | `effects/deploy` | 31 | **31** | +0 | 1.3 | 2 | `FAIL: keke debut asks which stage slot to deploy into (rs:NN) (got N expected N)` ×1 |
| 26 | `effects/kidou` | 31 | **31** | +0 | 1.3 | 1 | `FAIL: BN COST LEG N NOT PAID: the 起動 costs {{icon_energy}}{{icon_energy}} -- active energy was N after the pla` ×5 |
| 27 | `effects/look_select` | 25 | **30** | +5 | 1.2 | 1 | `FAIL: 「N枚まで好きな順番で」 -> count N (got N expected N)` ×1 |
| 28 | `effects/state/orientation_gated` | 28 | **28** | +0 | 1.1 | 1 | `FAIL: AN CONTROL: three candidates, so the prompt opens` ×2 |
| 29 | `rules/conditions` | 6 | **23** | +17 | 0.9 | 2 | `FAIL: BN: PN's card lands in PN's waitroom (got N expected N)` ×2 |
| 30 | `effects/auto` | 23 | **23** | +0 | 0.9 | 1 | `FAIL: AN: the wait branch of the `or` grants exactly one blade (got N expected N)` ×1 |
| 31 | `effects/look_select/look_and_filter` | 27 | **21** | -6 | 0.9 | 1 | `FAIL: kaho: the Nth pass must raise the deck-top inspection (nothing pending)` ×2 |
| 32 | `jidou/combination` | 39 | **21** | -18 | 0.9 | 1 | `FAIL: AN+: ab#N paid NE out of the N the N-cost ally left (N->N) and activated the N waited (N->N).  The zone ` ×1 |
| 33 | `effects/gain/hearts/constants` | 19 | **18** | -1 | 0.7 | 1 | `FAIL: SP-PRN: N combined members -> heartN` ×2 |
| 34 | `rules/phases` | 18 | **18** | +0 | 0.7 | 1 | `FAIL: GN: no draw — the hand shrank by one (got N expected N)` ×2 |
| 35 | `jidou/state_watch` | 17 | **17** | +0 | 0.7 | 1 | `FAIL: accepting the cost reactivates the waited 虹ヶ咲 member (ab#N's second step)` ×1 |
| 36 | `effects/cost_mod/required_hearts` | 16 | **16** | +0 | 0.6 | 1 | `FAIL: six distinct 蓮ノ空 stage+waitroom members -> N (got N expected N)` ×1 |
| 37 | `effects/score/per_card` | 14 | **14** | +0 | 0.6 | 2 | `FAIL: honoka: the gained ability must be registered on the card (N means the CONDITION never passed, so the ag` ×2 |
| 38 | `effects/energy/place` | 15 | **12** | -3 | 0.5 | 1 | `FAIL: a non-Liella member on the stage fails the group condition (got N expected N)  [non_liella_stage_member_` ×1 |
| 39 | `effects/score/card_score` | 11 | **11** | +0 | 0.4 | 2 | `FAIL: live-start driver: never reached the live-card set phase (stopped at Energy)` ×2 |
| 40 | `effects/energy` | 9 | **9** | +0 | 0.4 | 2 | `FAIL [fixture_cost_and_group_are_what_the_gates_assume]: PL!-pbNN-PP resolves to group "mu's" (series-derived)` ×2 |
| 41 | `effects/choice/per_card` | 10 | **8** | -2 | 0.3 | 1 | `FAIL: aN: 「自分のステージにメンバーがN人以上いる場合 … カードをN枚引く」 -- pN gains more than the control live (sunny N vs control N)` ×1 |
| 42 | `rules` | 4 | **4** | +0 | 0.2 | 1 | `FAIL: ENGINE BUG: an immediately repeated, otherwise identical state IS a repetition, so rb_check_permanent_lo` ×1 |
| 43 | `rules/targeting` | 3 | **3** | +0 | 0.1 | 1 | `FAIL: Total +N heartN across both members (got N expected N)` ×1 |
| 44 | `effects/position/area_gated` | 1 | **1** | +0 | 0.0 | 1 | `FAIL [other_syncrise_member_moves_to_center_ability_card_gains_blades]: BN.N: another NyncriNe! member moving ` ×1 |
| 45 | `effects/look_select/recruit_stage` | 5 | **1** | -4 | 0.0 | 1 | `FAIL: Proof: Hasunosora stage cost N >= N -> look N / take N grows the hand by N (got N expected N)` ×1 |

### Clusters that CLOSED entirely since BASELINE
* (none)

### The five biggest MOVEMENTS, in either direction

| cluster | BASELINE | NOW | delta | what moved |
|---|---|---|---|---|
| `UNRESOLVED` | 854 | 857 | +3 | the nozomi deck-order cluster (ranks 1–3) changed shape; see BASELINE2_top25.txt Part 4 |
| `jidou/combination` | 39 | 21 | -18 | |
| `effects/draw/flat` | 74 | 57 | -17 | |
| `effects/conditional/zone_source` | 53 | 44 | -9 | |
| `rules/conditions` | 6 | 23 | +17 | |
| `effects/look_select` | 25 | 30 | +5 | |

---

## 2. ROLL-UP BY Rust AREA

| Rust area (top-level `test_modules/` dir) | BASELINE | NOW | delta | %% of tree |
|---|---|---|---|---|
| `effects` | 972 | **912** | -60 | 37.0 |
| `UNRESOLVED` | 854 | **857** | +3 | 34.8 |
| `jidou` | 360 | **337** | -23 | 13.7 |
| `integration` | 204 | **204** | +0 | 8.3 |
| `rules` | 108 | **125** | +17 | 5.1 |
| `characterization` | 32 | **31** | -1 | 1.3 |

---

## 3. UNOWNED — the 857 FAIL lines no Rust module can be assigned to, in 43 suites

| suite | NOW | BASELINE | delta | dominant pattern (normalised) |
|---|---|---|---|---|
| `parity_sequential` | **113** | 113 | +0 | `FAIL: DN: the optional two-member cost prompts` ×4 |
| `parity_search_reorder` | **112** | 122 | -10 | `FAIL: nozomi: the mandatory order prompt is open` ×12 |
| `parity_hearts_live_start` | **67** | 64 | +3 | `FAIL: second selection prompt should appear` ×4 |
| `parity_mill_deck` | **53** | 53 | +0 | `FAIL: gin_ko: chosen card PL!-sdNN-SD left the opponent's discard` ×3 |
| `parity_condition_and_effect` | **48** | 52 | -4 | `FAIL: baton touch over a costN member -> the debut does NOT draw (got N expected N)` ×3 |
| `parity_restriction` | **43** | 43 | +0 | `FAIL: colour choice must be offered` ×5 |
| `parity_compound_per_card` | **41** | 41 | +0 | `FAIL: DN: the 「〜てもよい」 pay gate was not answerable; the pending prompt was: kind=N zone="hand" ta` ×4 |
| `parity_wait_activation` | **39** | 41 | -2 | `FAIL: 真姫 is played to centre` ×4 |
| `parity_zones_position` | **37** | 38 | -1 | `FAIL: a two-card deck with an empty waitroom cannot pay the cost` ×3 |
| `parity_live_total` | **33** | 33 | +0 | `FAIL: control: the cost prompt is raised` ×4 |
| `parity_blades_live_start` | **31** | 34 | -3 | `FAIL: two waited opponent members should grant >= +N blade` ×1 |
| `parity_stats_pipeline` | **31** | 31 | +0 | `FAIL: energy-threshold live start ability found` ×2 |
| `parity_jidou` | **30** | 26 | +4 | `FAIL: ab#N should place 澁谷かのん under the arriving member` ×2 |
| `parity_look_per_card` | **30** | 37 | -7 | `FAIL: mirai_costN: the selected card moves out of the revealed set` ×5 |
| `parity_card_score` | **21** | 21 | +0 | `FAIL: PL!S-bpNN-L: the optional energy-return cost is actually offered (a +N here must be the ga` ×3 |
| `parity_deploy_multi` | **21** | 21 | +0 | `FAIL: MN: a PN-owned 吟子 moves all three of PN's waitroom members to PN's OWN deck bottom (rs:NN)` ×1 |
| `parity_reveal_extra` | **20** | 20 | +0 | `FAIL: kanon: the pick is over looked_at (zone= count=N allow_skip=N | want zone=looked_at count=` ×1 |
| `live_card_zone_movement` | **12** | 0 | +12 | `FAIL: player N invalid card moves to player N waitroom` ×2 |
| `parity_rules` | **11** | 11 | +0 | `FAIL: drew N then discarded N -> one card remains (got N expected N)` ×2 |
| `parity_recover_to_hand` | **10** | 10 | +0 | `FAIL: N-card hand-discard cost must be prompted` ×2 |
| `harness_self` | **6** | 0 | +6 | `FAIL: AN sizeof(GameState) keeps RB_MIN_SAFE_LOCALS of them inside the N MB linked stack` ×1 |
| `max_distinct_names` | **6** | 6 | +0 | `FAIL: 'PL!HS-pbNN' resolved to N expected N` ×1 |
| `mechanics` | **5** | 0 | +5 | `FAIL: invalid live member leaves the live zone` ×1 |
| `audit_zones` | **3** | 3 | +0 | `FAIL: RED-PINNED: RB_MAX_HAND (N) cards in hand is not the end of the road -- Rust Hand is a Sma` ×1 |
| `condition_evaluation` | **3** | 3 | +0 | `FAIL: PN left_side should gain +N blade from its own LiveStart (got N expected N)` ×2 |
| `parity_queue_resume` | **3** | 1 | +2 | `FAIL: the tail keeps its order (got N expected N)` ×1 |
| `ported_generated` | **3** | 3 | +0 | `FAIL: generated opponent movement watcher fires (got N expected N)` ×1 |
| `tp_activation_offer` | **3** | 3 | +0 | `FAIL: every accepted activation charged exactly the {{E}}{{E}} = N cost` ×1 |
| `audit_actions` | **2** | 2 | +0 | `FAIL: the zone count WEIGHS the count_as_multiple card: N other + N for PL!-pbNN-L = N. RED unti` ×1 |
| `audit_conditions` | **2** | 2 | +0 | `FAIL: PL!N-pbNN-L temporal aggregate:total true in the live window when need heartN >= N (got N ` ×1 |
| `audit_fields` | **2** | 3 | -1 | `FAIL [heart_total_limit] M_heart_total_limit: heart_total_limit/operator must reach the appear-t` ×1 |
| `audit_triggers` | **2** | 5 | -3 | `FAIL: [AN.plain_hand_prompt_positive] AN_same_unit_name_prompt: PAIRED POSITIVE: a plain optiona` ×1 |
| `look_select` | **2** | 2 | +0 | `FAIL: look-and-select followup runs after finalization (got N expected N)` ×1 |
| `parity_custom_misc` | **2** | 2 | +0 | `FAIL: pay_cost_all:discard_all empties the hand (got N expected N)` ×1 |
| `performance_phase_rules` | **2** | 2 | +0 | `FAIL: Sumire heartN matches the yell contents (got N expected N)` ×1 |
| `p1_helpers` | **1** | 1 | +0 | `FAIL: opponent spawn routes choice to opponent` ×1 |
| `parity_auto_abilities` | **1** | 1 | +0 | `FAIL: gained entries use the GAINED_ABILITY_INDEX_BASE index space` ×1 |
| `parity_cost_and_effect` | **1** | 1 | +0 | `FAIL: A: net hand change is N cost card +N drawn card (got N expected N)` ×1 |
| `parity_cost_compound` | **1** | 0 | +1 | `FAIL: paying that unpayable cost fails (got N expected N)` ×1 |
| `parity_look_reveal` | **1** | 1 | +0 | `FAIL: the followup action still runs on the unmatched path (got N expected N)` ×1 |
| `parity_state_family` | **1** | 0 | +1 | `FAIL: OPEN GAP (test_parity_score_effects.c:N): a SET on live_total is refused and applies nothi` ×1 |
| `parity_support` | **1** | 1 | +0 | `FAIL [B_mandatory_select_card_not_skippable]: §B: 鬼塚夏美's ライブ成功時 must raise its mandatory hand-di` ×1 |
| `surplus_heart` | **1** | 1 | +0 | `FAIL: QN energy must be placed in WAIT state (got N expected N)` ×1 |

### Sub-group (a) — C-only audit / gate tools. Unowned BY DESIGN, route to a tooling owner.

`audit_actions` (2), `audit_conditions` (2), `audit_fields` (2), `audit_triggers` (2), `audit_zones` (3), `condition_evaluation` (3), `harness_self` (6), `max_distinct_names` (6)

### Sub-group (b) — parity suites with a real ownership gap. This is the actionable list.

831 lines across 35 suites. The top three alone are **19 lines = 0.8 % of the whole tree** and no
Rust owner currently exists for them in this repo's translation plan.

```
    12  live_card_zone_movement
     2  look_select
     5  mechanics
     1  p1_helpers
     1  parity_auto_abilities
    31  parity_blades_live_start
    21  parity_card_score
    41  parity_compound_per_card
    48  parity_condition_and_effect
     1  parity_cost_and_effect
     1  parity_cost_compound
     2  parity_custom_misc
```

---

## 4. HOW TO RE-DERIVE THIS FILE

```
raw runs   C:\Users\trios\AppData\Local\Temp\kilo\base2\logs\runs\run1_<suite>.{out,err}
baseline   C:\Users\trios\AppData\Local\Temp\kilo\baseline\logs\runs\run1_<suite>.{out,err}   (BASELINE.md 2 530)
script     C:\Users\trios\AppData\Local\Temp\kilo\base2\attrib.py
index      rebuilt each run from engine\tests\test_modules\**\*.rs (4 984 unique fn names)
```
If `MANIFEST.sha256` no longer matches the tree you are looking at, this attribution
describes a different snapshot.

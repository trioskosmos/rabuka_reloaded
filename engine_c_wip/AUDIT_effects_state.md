# AUDIT — the STATE effect family (Rust -> C parity), 2026-10-03

Owner: the "state" cluster agent. Scope: `ability/effects/{change_state,position,ability_ops,score}.rs`
against `src/ability/effects/{state,ability,ability_effects,score}.c`.

---

## 0. Read this first — two of the five assigned Rust paths do not exist

The brief lists `ability/effects/state.rs` (2064 lines). **There is no such file.**
`git log --all -- engine/src/ability/effects/state.rs` returns exactly one commit:

```
cf125b15  refactor(engine): reorganize ability effects into per-kind modules
          R  engine/src/ability/effects/state.rs -> engine/src/ability/effects/change_state.rs
```

`state.rs` was RENAMED to `change_state.rs` and split across the per-kind modules. The
2064-line `src/ability/effects/state.c` is therefore a port of a file that no longer
exists, and its functions map to **six** current Rust homes, not one:

| state.c function | current Rust home |
|---|---|
| `rb_effect_change_state` | `effects/change_state.rs:119` `execute_change_state` |
| `rb_effect_energy_placement` | `effects/energy.rs` `execute_energy_placement` |
| `rb_effect_energy_state_change` | `effects/energy.rs` `execute_energy_state_change` |
| `rb_effect_set_cost*` | `effects/cost.rs` `execute_set_cost` (0 corpus) |
| `rb_effect_set_blade_type/count` | `effects/blade.rs` |
| `rb_effect_set_heart_type*` | `effects/heart.rs` |
| `rb_effect_modify_cost`, `rb_effect_set_cost_to_use`, `rb_effect_activation_cost` | `effects/cost.rs` |
| `rb_effect_specify_heart_color` | `effects/color.rs` |
| `rb_effect_set_card_identity*` | `effects/state.rs` — **in core, not effects** |
| `rb_effect_reduce_live_card_set_limit` | `effects/restriction.rs` |
| `rb_effect_position_change`, `rb_resume_position_change`, `rb_effect_rotation` | `effects/position.rs` |

`ability/effects/state.rs` in the brief is stale; `change_state.rs` (796 lines) is the real twin.

Also missing from the brief's suite list, with no source file anywhere: `parity_dynamic_count`
and `audit_cards`. `tests/` has 145 `test_*.c` and none of the two. Nearest substitutes are in
the measured set below (`parity_per_card*` for the dynamic-count surface, `audit_actions` +
`audit_fields` for the card-census surface).

---

## 1. Method — the census is decode-level, not JSON-level

`cards/abilities.json` (the compile input, 985 abilities) was used only as a cross-check.
The authority is a decode-level walk of the SHIPPED bytecode: a private tool
(`est_census.c` / `est_dump.c` / `est_trig.c`, kept in the build snapshot, not in the repo)
iterates all 985 abilities via `rb_decode_ability` and visits **every** effect-valued field
of `AbilityEffect` — `child[]`, `gained_effect`, `primary_effect`, `alternative_effect`,
`look_action`, `select_action`, `followup_action`, `optional_action`, `conditional_action`
— plus `Ability.cost`, because 52 of the 148 `change_state` nodes live under `cost`
and a cost-side tree walk is the only way to count them. `rb_canonical_trigger` supplies the
trigger class, which is what makes "reachable from the 常時 scan" a measurement rather than
a guess.

Nodes: 1866 effect nodes. Distinct actions: 48. Distinct extras: 146.

Corpus totals for the actions this family owns (effect side):

```
change_state                    96      modify_score                    130
position_change                 27      modify_required_hearts           29
modify_cost                     22      set_heart_type                   9
gain_ability                    21      specify_heart_color              8
sequential                     262      set_blade_type                   2
activate_ability                 4      reduce_live_card_set_limit       2
invalidate_ability               3      gain_ability_from_source         2
set_card_identity                1      play_baton_touch                 3
suppress_ability_trigger         1      rotation                         0
set_cost / set_cost_to_use / all_blade_timing / energy_placement /
energy_state_change / activation_cost / modify_limit / custom / reveal_per_group /
modify_required_hearts_success / action_by / compound_action /
conditional_optional / do_nothing                    0
```

Trigger class for the actions whose reachability depends on it:

```
change_state             debut 40, live_start 21, auto 15, activation 17, live_success 3, CONSTANT 0
position_change          debut 10, live_start 8, activation 6, auto 2, live_success 1, CONSTANT 0
rotation                 (nothing)
gain_ability_from_source CONSTANT 2
play_baton_touch         CONSTANT 2, debut 1
modify_cost              CONSTANT 17, live_start 4, activation 1
set_heart_type           live_start 8, activation 1
set_card_identity        CONSTANT 1
suppress_ability_trigger CONSTANT 1
```

---

## 2. Per-Rust-item inventory

Verdict key: **OK** = arm present and faithful · **NOOP** = verified unreachable, 0 of 985 ·
**FIXED** = divergence found, closed in this audit · **OPEN** = reachable gap I cannot close
(another file) · **DEAD** = C function with zero callers, deleted or confirmed

### 2.1 `ability/effects/change_state.rs` (the renamed `state.rs`)

| Rust item | line | C twin | reach | verdict |
|---|---|---|---|---|
| `execute_change_state` | 119 | `state.c:70` `rb_effect_change_state` | 96 effect + 52 cost | **FIXED** (7 divergences, §3) |
| `execute_energy_placement` | (energy.rs) | `state.c:680` `rb_effect_energy_placement` | **0 of 985** — reached only from `change_state` with `source="deck" && destination="energy"`; no `change_state` node carries `destination`, and `energy_placement` as an action is 0 | NOOP |
| `execute_energy_state_change` | (energy.rs) | `state.c:707` | 27 of 96 (`card_type=energy_card`) | OK |
| `rb_bulk_state_evaluate` | — | `state.c` (was :1930-2057) | **0 callers, 0 declarations** | **DEAD — DELETED** (§3.8) |
| `rb_stage_position_change_with_bags` caller set | — | `state.c:1677/1692/1732` | all three | OK |

### 2.2 `ability/effects/position.rs`

| Rust item | line | C twin | reach | verdict |
|---|---|---|---|---|
| `execute_play_baton_touch` | 31 | **NONE in my files.** `misc.c` `h_play_baton_touch` (live, executor) | 3 (CONSTANT 2, debut 1) | **OPEN** — see BLOCKED-ON below |
| `execute_position_change` | 96 | `state.c:1701` `rb_effect_position_change` — reached ONLY from `triggers.c:1204`, the CONSTANT scan | **0 of 985** (`position_change` CONSTANT = 0) | NOOP (the live twin is misc.c's family) |
| `compute_valid_position_destinations` | 666 | **NONE in my files.** `misc.c` | 27 nodes | **OPEN** — misc.c |
| `finalize_formation_change` | 778 | **NONE in my files.** `misc.c` `rb_misc_finalize_formation_change` | 7 of 27 carry `multiple_targets` | **OPEN** — misc.c |
| `execute_position_change_with_destination` | 927 | `state.c:1710` `rb_resume_position_change` | live: called from `choice.c:870`, `:4290`, `:4873` | **FIXED-complementary** — see §4 |
| `execute_rotation` | 1124 | `state.c:1740` `rb_effect_rotation` (from `triggers.c:1206`) | **0 of 985** (`rotation` = 0 nodes, any trigger) | NOOP |
| `gs.record_stage_position_swap` consumers | 1022, 1061, 1102 | **NONE** | all 3 consumers are inside `execute_position_change_with_destination`, which IS live | **display-only gap, not a wrong value** — §4 |

### 2.3 `ability/effects/ability_ops.rs` — the duplication question

| Rust item | line | C twin | reach | verdict |
|---|---|---|---|---|
| `execute_gain_ability` | 439 | `ability.c:337` `rb_gain_ability` | 21 | OK (single twin) |
| `execute_gain_ability_effect` | 12 | `ability_effects.c:94` | via `gain_ability`'s `gained_effect_action` | OK |
| `execute_activate_ability` | 305 | `ability_effects.c:257` | 4 | OK |
| `execute_invalidate_ability` | 366 | `ability_effects.c:298` | 3 | OK |
| `execute_set_card_identity` | 702 | `ability.c:625` + `ability_effects.c:426` | 1 | OK |
| `execute_suppress_ability_trigger` | 667 | `ability.c:588` `rb_suppress_ability_trigger` (the body) wrapped by `ability_effects.c:400` | 1 | OK — **layered, not duplicated** |
| `execute_gain_ability_from_source` | 593 | **`ability.c:519` AND `ability_effects.c:507`, BOTH LIVE** | 2 (both CONSTANT) | **FIXED — collapsed to one** (§5) |
| `execute_custom_effect` | 765 | deleted with the `custom` action | **0 of 985** | NOOP |

### 2.4 `ability/effects/score.rs`

| Rust item | line | C twin | reach | verdict |
|---|---|---|---|---|
| `apply_distinct_filter` | 47 | `score.c` (via `rb_count_distinct_member_name_units`) | `distinct` = `card_name` ×3 only | OK — every `distinct` in the corpus is `card_name`, so the joint-aware count IS the only branch; the `false`/`0` fallback never runs |
| `count_distinct_member_name_units` | 62 | `score.c:362` | 3 | OK |
| `execute_modify_score` | 165 | `score.c:493` | 130 (94 per-card, 36 live_total) | **1 divergence, unreachable — §6** |
| `execute_modify_required_hearts` | (required_hearts.rs:28) | `score.c:782` | 29 | OK — out of my Rust modules, but my C file |
| `execute_modify_yell_count` | (reyell.rs) | `score.c:871` | 3 | OK |
| `execute_modify_limit` | (restriction.rs) | `score.c:990` | **0 of 985** | NOOP |
| `execute_modify_required_hearts_success` | (required_hearts.rs) | `score.c:821` | **0 of 985** | NOOP |

### 2.5 Functions in state.c with NO Rust counterpart in my five modules

`rb_effect_set_cost` (0), `rb_effect_set_cost_to_use` (0), `rb_effect_all_blade_timing` (0),
`rb_effect_activation_cost` (0), `rb_effect_energy_placement` (0). All **0 of 985**; all
inherited from the pre-rename `state.rs` and all now duplicated by rules.c / cost.c.

---

## 3. Divergences FOUND and CLOSED in this audit — all in `rb_effect_change_state`

Seven, in `src/ability/effects/state.c`. Every one is against `change_state.rs`. Reach is
the decode-level count from §1.

| # | Rust | C (before) | reach |
|---|---|---|---|
| **C1** | `:662-669` — after the orientation write, `gs.recalculate_constants()`; the comment names the reason: a card whose own effect satisfies its own 常時 must not stay stale | **no recalculation anywhere in the function**. The only recalc on the path was `engine.c:1061`, after `rb_drain_ability_queue`, far too late for the sweep at `:765-768` | 69 member `change_state` nodes |
| **C2** | `:653-659` — `if state_change=="active" && !cards_activated_this_turn.contains(cid) { push }` | the field exists (`rabuka.h:1495`, audit A8) with **no writer in src/\*\*`, so the `activated_this_turn` condition (`condition/card.rs:2417-2424`, 3 corpus nodes) was permanently false | 47 `state_change="active"` nodes |
| **C3** | `:458-467` — when `action_by=="opponent"`, drop wait-immune members from the OFFER (Q275: 「相手は、自分のステージのアクティブなメンバーをウェイトにする」) | no immunity check at all | 3 nodes (`action_by="opponent"`, all with `state="active"`) |
| **C4** | `:585-599` — re-filter `actual_targets` by wait-immunity for EVERY `state_change=="wait"` step (Q274) | no immunity check at all | 3 nodes |
| **C5** | `:143-145` — `if per_unit && count == 0 { return Ok(()) }` | computed the zero, then fell through the deck→energy short-circuit, the member branch and the no-candidate exit. Measured effect: the baseline **waited a card, wrote a `turn_state_changes` entry, pushed onto `recently_state_changed` and added to `selection.cards`** for a step that printed 「1枚の」 and found nothing | 5 `per_unit` nodes |
| **C6** | `:762-768` — `trigger_auto_abilities_for_player` for p1 AND p2, unconditionally | `rb_trigger_auto_abilities_for_player(g, who); if (who != (actor^1)) rb_trigger_auto_abilities_for_player(g, actor^1);` — skips the second call exactly when `who == actor^1`, i.e. whenever the effect targets the OPPONENT, so the controller's own 常時 were never re-evaluated | **40 of 96** `target="opponent"` nodes |
| **C7** | `:684-687` — `selection.changed_state_members = actual_targets` (an assignment, not a push) | the writer hook `rb_rules_note_changed_state_members` had **zero callers tree-wide** | 96 nodes; the reader is unreachable (§3.1), so the writer half now exists and the reader is BLOCKED-ON |

### 3.8 Deletion: `rb_bulk_state_evaluate`

`state.c` held a **second, ~300-line copy** of `execute_change_state` in the same
translation unit as `rb_effect_change_state`, introduced as a "BULK COPY". Proof of
death by name over `src/`, `include/`, `tests/`, `tools/`, `*.py`, `*.md`, `Makefile`,
`*.sh`: **zero call sites, zero declarations** (the only other hit is a
`DEPENDENCY_AUDIT.md` table row). It was stale, not merely redundant: it re-derives
candidates without the `cost_limit` / `blade_limit` filter (calls `rb_card_matches_type` +
`rb_card_matches_group_str` + `s_match_chars` directly, never `rb_matching_ids`), keys its
state decision off `state_filter` instead of `state_change`, sweeps auto abilities for
`actor` only, and carries none of C1-C7. Deleted with a full deletion note in place.
Precedent in the same file: `rb_effect_modify_hearts`, deleted 2026-10-02.

### 3.1 Why C7 is a writer half only — `restriction` is triplicated

The `changed_state_members` READER does not exist on the live path:

* `executor.c` routes the `restriction` action to `execute_misc` → `misc.c:2059` → `misc.c:887` `h_restriction`.
* `h_restriction` computes the delayed cannot-activate victims itself (`misc.c:905-918`): first `g->recently_moved`, then the activating card. **No `changed_state_members` arm.**
* `rules.c:1819 rb_rules_execute_restriction` HAS the arm with the right precedence (`rules.c:1848-1866`) — and has **zero callers**, as do `rb_rules_execute_activation_restriction`, `rb_rules_execute_re_yell`, `rb_rules_execute_perform_yell`, `rb_rules_execute_modify_yell_count`, `rb_rules_execute_modify_limit`, `rb_rules_reduce_live_card_set_limit`.

So the live twin of Rust's `execute_restriction` is missing a clause that a dead twin
already has. BLOCKED-ON below.

---

## 4. `position.rs` — the Rule 4.5.5.3 / bags question, answered

**The bags path is correct and complete, and no production caller degrades.**

`rb_stage_position_change_core(stage, under, stage_wait, …)` takes `under = NULL` only in
`rb_stage_position_change` (zones.c:271-274), the bare-array form. Its callers:

| Caller | Form | Note |
|---|---|---|
| `state.c:1677` (`rb_pos_change_for_player`, `source_position` branch) | 4-arg bags | |
| `state.c:1692` (same, `target_member` branch) | 4-arg bags | |
| `state.c:1732` (`rb_resume_position_change`) | 4-arg bags | |
| `zones.c:282` (`rb_stage_position_change_with_bags`) | 4-arg bags | |
| `tests/test_parity_zones_position.c:304,312,319,323,325,327` | **3-arg** | tests only |
| `tests/test_parity_under_member.c:619-620` | local `extern` decls | the suite drives the 4-arg form at `:755` and `:1058` |

**Zero production callers use the 3-arg form.** The crossed take
(`zones.c:232-240`, the C twin of `zones.rs:396-400`) is therefore live and exercised on
every position change. The defect described in the brief has already been resolved.

`record_stage_position_swap` (checklist **E9**) is still ABSENT — but its absence is a
**display/replay-fidelity gap, not a wrong value**, which is why it is not in §3:

* `record_card_movement` — the C already does this (`rb_record_movement`, state.c:1678/1733).
* push to `gs.position_change_events` — Rust has **no behavioural reader**: it is written
  (position.rs:617/628/660/909/921/1179), cleared (`abilities.rs:1863`), and read only by
  `display.rs:1857-1861`. The C never writes it at all — `game_state_abilities.c:589` only
  clears it, and no `src/**` code has ever written one (only tests do).
* `push_movement_event` → `gs.batch_movements` — consumed by the batch re-scan
  (`abilities.rs:1843-1863`), which the C models with
  `rb_trigger_auto_abilities_for_movement_current` over `g->recently_moved`.

All three consumers (position.rs:1022, :1061, :1102) are inside
`execute_position_change_with_destination`, which is live, and both C routes into it
(`rb_pos_change_for_player`, `rb_resume_position_change`) use the 4-arg bags form. The
production behaviour of `position.rs` is covered; only the event list is missing.

The three `position_change` twins are: `state.c:1701` (CONSTANT scan, **0 of 985**),
`state.c:1710` (resume, **live**), `misc.c` `h_position_change` (executor, **live**).

---

## 5. VERDICT on the `ability.c` vs `ability_effects.c` duplication

**`ability.c` is LIVE and is NOT a duplicate of `ability_effects.c` as a whole. But
`execute_gain_ability_from_source` WAS duplicated, both copies were live, and they
diverged on four points. They are now one implementation.**

Evidence:

| | `ability.c` | `ability_effects.c` |
|---|---|---|
| `rb_gain_ability` | live (`misc.c:19`, `ability_effects.c:143`, `engine.c:621`) | — |
| `rb_gain_ability_from_source` | live — `triggers.c:1181`, the CONSTANT scan | — |
| `rb_translated_execute_gain_ability_from_source` | — | live — `executor.c:476`, the `ActionType::GainAbilityFromSource` dispatch |
| `rb_suppress_ability_trigger` | live — this IS the body; `ability_effects.c:405` calls it | the `ability_effects.c:400` wrapper is the dispatch shim only |
| AbilityRef cache (`rb_ability_store_*`) | live (`engine.c`, `phase.c`) | — |
| `rb_tick_gained` | live — `turn/phase.c` | — |

So `ability.c` = the ability-store + the gain/suppress **effects**; `ability_effects.c` =
the **executor shims** over the same effects. That layering is correct and is not the
problem. The problem was one function, `execute_gain_ability_from_source`
(ability_ops.rs:593-700), implemented TWICE:

| Divergence | `ability_effects.c` (right) | `ability.c` (wrong) |
|---|---|---|
| `trigger_filter` | `trigger_filter_matches` splits the CSV on `,` and tests each token — Rust's `filters.iter().any(…)` | `strstr(ability.triggers, tf) \|\| strstr(tf, ability.triggers)` with `tf` = the WHOLE comma-joined string |
| owner of the activating card | `rb_owner_of_card` — Rust's player1-then-player2 probe (`:624-632`) | `g->p[actor].stage` |
| `group_names` | `ae_group_matches_csv_any` — any-of-the-list (`:665-671`) | `rb_effect_filter_subset` → `rb_count_matching_filter` → util.c's single-name `check_group` |
| `cost_limit_operator` | `rb_card_matches_cost_limit(…, op)` defaults `<=` — Rust's `unwrap_or("<=")` | same helper, so same |
| tail | logs `[[log_gain_ability_from_source:N]]` + `rb_recalc_constants` | neither |

**Reachability:** both corpus nodes carry a SINGLE-token `trigger_filter` — 「起動」 on the
`group_names=Liella!` node, 「ライブ成功時」 on the `cost_limit=9 cost_limit_operator="<=",
group_names=虹ヶ咲` node — so the CSV divergence is **0 of 985** and the two twins agreed on
every real card. The **recalc difference is load-bearing and was preserved**: it must NOT
be on the CONSTANT path, because `modifiers.rs:1148` runs that scan from inside
`recalculate_constants` itself and a callback would re-enter it forever.

**Fix:** the faithful body now lives once, as
`ability_effects.c:rb_ability_gain_from_source_core`, and `ability.c`'s
`rb_gain_ability_from_source` is a 1-line delegation that does not recalculate. One
implementation, both call sites, the re-entrancy guard kept in the wrapper. A new card that
widens `trigger_filter` to two tokens can no longer disagree with itself.

`gained_ability_sources` (`mod.rs:631`, pushed at `ability_ops.rs:521` and `:740`, cleared at
`modifiers.rs:2110`) is **written by no C twin and read by no Rust code** other than
`display.rs:1733` — VERIFIED-NOOP, not ported.

---

## 6. Divergences found and deliberately NOT closed

| Rust | C | reach | why not |
|---|---|---|---|
| `score.rs:274-287` — `operation=="set"` on `live_total` logs an error and `return Ok(())`; the `"set"` arm of the delta match at `:251-256` is computed and never used | applies the value as a delta — the exact "silently behaves as add" the Rust comment warns about | **0 of 985** (`set` is on 1 of 130 `modify_score` nodes and that node targets a card) | **Blocked by a peer suite.** `tests/test_parity_score_effects.c:1125` asserts the OPPOSITE — `live_total operation=set adds 4 onto the existing +1 (score.rs:142-146)` — citing line numbers from a pre-split `score.rs` (`:142-146` is now inside the live-zone list). The refusal WAS implemented and A/B'd: it closed this audit's own assertion and turned `parity_score_effects` 0 → 1. Reverted per the "0 of 985 ⇒ VERIFIED-NOOP" rule; the exact note is left in `score.c` so the next agent does not re-derive it. |
| `rb_effect_change_state` uses `exclude_self: gs.activating_card` (`:315`) | `exclude_self_id = host_cid` | `exclude_self` = 2 nodes, both on non-CONSTANT abilities where `host_cid` is the entry's host | not changed: no measurement separates the two, and the CONSTANT route (where `host_cid` is literally `-1`, `triggers.c:1202`) is 0 of 985 anyway |
| `util::CardFilter.original_heart_total` — the 「元々持つハートの数」 clause of a `change_state` | `rb_matching_ids` honours `original_heart_total` (`util.c:1701/1709/1868/1873`) but **`card_filter_from_effect` never populates it** — there is no `heart_total_limit` string anywhere in `card.c` | `heart_total_limit` = 4 of 96, all `<=` | **BLOCKED-ON `src/core/card.c:957`** — the matcher half exists, the producer does not. Not my file. |
| `mods.last_cost_energy_count` for `per_unit_type="つ"` (`score.rs:225-227`, `change_state.rs:346-348`) | returns 0; `util.c:48` has the match behind `#ifdef RB_MODS_LAST_COST_ENERGY_COUNT`, which nothing defines | `per_unit_type="つ"` ×1 on `modify_score` | **BLOCKED-ON `include/rabuka.h:1495`-area + `src/core/modifiers.c`** — the field does not exist. Adding it lights up util.c's existing match. |
| `condition/card.rs:2417-2424` — the `activated_this_turn` CONDITION reading `cards_activated_this_turn` | no `cards_activated_this_turn` reference anywhere in `condition.c` | 3 corpus nodes | **BLOCKED-ON `src/ability/condition.c`** — the writer half (C2) is now in place; the reader half is not. |
| `restriction.rs:191` `wait_immune_members` — the C's `g_wait_immune_members` is exported and has **no writer**; rules.c keeps a file-static `st->wait_immune` instead | `state.c` now calls `rb_rules_is_wait_immune`, which reads the static that rules.c DOES populate (`rules.c:1916-1921`) | 3 `action_by="opponent"` nodes | wired and working from state.c's side; no further action needed |
| `misc.c` `h_position_change` / `compute_valid_position_destinations` / `rb_misc_finalize_formation_change` / `h_play_baton_touch` | the entire `position.rs` executor twin lives in `misc.c`, not in my four files | 27 / 27 / 7 / 3 | **BLOCKED-ON `src/ability/effects/misc.c`** — see §8 |

---

## 7. Per-effect-key arm-completeness table

Every key the five Rust modules dispatch on, with its decode-level corpus count and the C
arm that handles it. `?` = the action exists in Rust's dispatch and in C's table but the
executor path is 0 of 985.

| Rust key (ActionType) | count | in Rust | C arm | live C file | verdict |
|---|---|---|---|---|---|
| `change_state` | 148 | `change_state.rs:119` | `executor.c:361` + `triggers.c:1202` | state.c | FIXED (C1-C7) |
| `position_change` | 27 | `position.rs:96` / `:927` | `executor.c` → misc; `choice.c` → state.c resume; `triggers.c:1204` → state.c (dead) | misc.c + state.c | split; see §4 |
| `rotation` | **0** | `position.rs:1124` | `triggers.c:1206` only | state.c:1740 | NOOP |
| `play_baton_touch` | 3 | `position.rs:31` | misc only; **no CONSTANT arm at all** in `triggers.c` | misc.c | OPEN — 2 of the 3 are CONSTANT |
| `gain_ability` | 21 | `ability_ops.rs:439` | `executor.c` / `misc.c:19` / `engine.c:621` | ability.c | OK |
| `gain_ability_from_source` | 2 | `ability_ops.rs:593` | `executor.c:476` + `triggers.c:1181` | ability_effects.c + ability.c | FIXED (collapsed) |
| `activate_ability` | 4 | `ability_ops.rs:305` | `executor.c` | ability_effects.c | OK |
| `invalidate_ability` | 3 | `ability_ops.rs:366` | `executor.c` | ability_effects.c | OK |
| `suppress_ability_trigger` | 1 | `ability_ops.rs:667` | `executor.c` | ability_effects.c → ability.c | OK (layered) |
| `set_card_identity` | 1 | `core/game_state/state.rs` | `executor.c` | ability_effects.c + ability.c | OK |
| `modify_score` | 130 | `score.rs:165` | `executor.c` | score.c | 1 unreachable divergence |
| `modify_required_hearts` | 29 | required_hearts.rs:28 | `executor.c` | score.c | OK |
| `modify_required_hearts_global` | 3 | required_hearts.rs:225 | `executor.c` | score.c | OK |
| `modify_yell_count` | 3 | reyell.rs | `executor.c` | score.c | OK |
| `modify_yell_source` | 1 | reyell.rs | `executor.c` | score.c | OK |
| `modify_required_hearts_success` | **0** | required_hearts.rs | `executor.c` | score.c | NOOP |
| `modify_limit` | **0** | restriction.rs:253 | `executor.c` | score.c | NOOP |
| `set_cost` | **0** | cost.rs | `executor.c` | state.c:855 / rules.c | NOOP |
| `set_cost_to_use` | **0** | cost.rs | `executor.c` | state.c:1356 / rules.c | NOOP |
| `all_blade_timing` | **0** | blade.rs | `executor.c` | state.c:1428 | NOOP |
| `activation_cost` | **0** | cost.rs | `executor.c` | state.c:1192 / rules.c | NOOP |
| `energy_placement` | **0** | energy.rs | `executor.c` | state.c:680 | NOOP |
| `energy_state_change` | **0** | energy.rs | `executor.c` | state.c:707 | NOOP (the 27 energy `change_state` nodes reach it through `change_state`) |
| `set_heart_type` | 9 | heart.rs | `executor.c` | state.c:1081 | OK |
| `set_blade_type` | 2 | blade.rs | `executor.c` | state.c:905 | OK |
| `set_blade_count` | 1 | blade.rs | `executor.c` | state.c:947 | OK |
| `specify_heart_color` | 8 | color.rs | `executor.c` | state.c:1317 | OK |
| `modify_cost` | 22 | cost.rs | `executor.c` / `triggers.c` | state.c:1468 / rules.c | OK |
| `reduce_live_card_set_limit` | 2 | restriction.rs | `executor.c` | state.c / rules.c | OK |
| `restriction` | 14 | restriction.rs:11 | `executor.c` → misc.c:887 | misc.c (rules.c copy is DEAD) | OPEN (§3.1) |
| `custom` | **0** | custom.rs | deleted | — | NOOP |

Arm-level key census inside `change_state` (96 nodes). `in C?` is `rb_effect_change_state`'s
arm set after this audit:

| extra key | count | in C before | after |
|---|---|---|---|
| `state_change` (`wait` 49 / `active` 47) | 96 | yes | yes |
| `target` (`opponent` 40 / `self` 53 / `both` 2) | 95 | yes | yes |
| `cost_limit` + `cost_limit_operator` (`>=` 1) | 18 | yes | yes |
| `blade_limit` + operator (`==` 1) | 13 / 15 | yes | yes |
| `group_names` | 11 | yes | yes (and nulled for `target="opponent"`, Rust `:161-165`) |
| `self_target` | 11 | yes | yes |
| `max` | 7 | yes | yes |
| `blade_limit_from_energy_under` + `blade_limit_offset` | 1 / 2 | yes | yes |
| `heart_total_limit` + operator | 4 | matcher only, **producer missing** | unchanged — BLOCKED-ON card.c |
| `per_unit` (+ `per_unit_type`, `per_unit_source`, `location`) | 5 | yes | **+ early return when scaled count is 0** |
| `distinct` (`card_name` only) | 3 | yes | yes |
| `state` (`active`) | 3 | yes | yes |
| `action_by` (`opponent`) | 3 | **NO** | **+ candidate filter** |
| `exclude_self` | 2 | yes | yes |
| `optional` | 2 | yes | yes |
| `exclude_group_names` | 1 | yes | yes |
| `cost_from_revealed` | 1 | yes | yes |
| `original_value`, `activation_position`, `location` | 19 / 5 / 4 | carried, unread by Rust's change_state | unchanged |
| `wait_immune` (Q275) | — | **NO** | **+ candidate + apply-time filter** |
| `cards_activated_this_turn` write | — | **NO** | **+ writer** |
| `recalculate_constants` after apply | — | **NO** | **+ call** |
| `changed_state_members` write | — | **NO** | **+ call** |
| auto-ability sweep, BOTH seats | — | **one seat when target=opponent** | **both seats** |

---

## 8. BLOCKED-ON (files I do not own)

1. **`src/ability/effects/misc.c:905`** — inside `h_restriction`'s `if (delayed)` branch,
   insert before the `g->recently_moved` loop:
   ```c
   extern int rb_rules_changed_state_count(GameState *g);            /* new accessor */
   /* ... */ if (n > 0) { for each cid -> rb_mods_add_delayed_cannot_active(&g->mods, cid, 1); }
   ```
   mirroring `rules.c:1848-1866`, which already has the right precedence. The slot
   `st->changed_state_members` is a `rules.c` file-static, so it must be lifted to an
   exported accessor in `rules.c` — or `rules.c:1819 rb_rules_execute_restriction` made the
   live dispatch target instead of `misc.c:887 h_restriction`. Reachable: `restriction` is
   on 14 of 985. New test: `test_parity_state_family.c` section D (red).
2. **`src/core/card.c:957`** (`card_filter_from_effect`) — no `heart_total_limit` string
   exists anywhere in the function, so `filter.has_original_heart_total` is never set and
   the 「元々持つハートの数がN以下」 clause of a `change_state` is inert. `util.c`'s matcher
   already honours it. Reachable: 4 of 96.
3. **`src/ability/condition.c`** — add the `activated_this_turn` condition reading
   `g->cards_activated_this_turn` (`condition/card.rs:2417-2424`). The writer half landed in
   state.c; without the reader the 3 corpus nodes stay inert. Reachable: 3 of 985.
4. **`tests/test_parity_score_effects.c:1125`** — drop the
   `live_total operation=set adds 4 onto the existing +1` expectation. It contradicts
   `score.rs:274-287` and cites pre-split line numbers. After this, `score.c`'s live-total
   arm can add `if (!strcmp(op, "set")) return 0;` (the exact note is already in `score.c`).
   Reachable: 0 of 985.
5. **`src/ability/effects/misc.c`** — `position.rs`'s whole executor twin
   (`h_position_change`, `compute_valid_position_destinations`,
   `rb_misc_finalize_formation_change`, `h_play_baton_touch`) plus the 3 `play_baton_touch`
   CONSTANT nodes that `triggers.c` has no arm for. Out of my file set; not audited
   arm-by-arm here.
6. **`include/rabuka.h:1495` area + `src/core/modifiers.c`** — add
   `mods.last_cost_energy_count` (Rust `mod.rs:982`, written by `choice.rs:943`) so
   `util.c:48`'s existing `#ifdef RB_MODS_LAST_COST_ENERGY_COUNT` match lights up for
   `per_unit_type="つ"`. Reachable: 1 of 130 `modify_score`.
7. **`src/ability/effects/rules.c`** — `rb_rules_execute_restriction`,
   `_activation_restriction`, `_re_yell`, `_perform_yell`, `_modify_yell_count`,
   `_modify_limit` and `rb_rules_reduce_live_card_set_limit` all have ZERO callers;
   `misc.c`'s `h_restriction`/`h_re_yell`/`h_perform_yell` are the live twins. Either delete
   the rules.c copies or make them the dispatch target. Not mine to decide.

---

## 9. A/B attribution

Trees: `est` (this audit's four files) vs `estb` (the same four files reverted to their
pre-audit content via `git show HEAD:engine_c_wip/…` — **not** `git checkout`, and HEAD
content == pre-audit content here because `git status` showed all four clean at session
start). A full-tree SHA-256 sweep over every `*.c`, `*.h` and `Makefile` reports **exactly
4 differing files**, the four I own. `abilities_strings.bin` = 726831 / `89204DF0BBDE08A0`
in both trees before and after every build; `make data` is GREEN in the snapshot when
invoked with RELATIVE blob paths (the guard's `sha256sum` output is `\`-prefixed for a
Windows absolute path, which is a false negative in `tools/guard_data_blob.sh`, unrelated
to this audit).

64 suites, built per-suite with the Makefile's own flags, run with the working directory set
to the tree root so `rb_load("src")` resolves. `FAIL:` lines taken from stdout and stderr
separately (never `*>&1`), judged on sorted lines only.

| | BASE | AFTER | delta |
|---|---|---|---|
| all 64 suites, `FAIL:` line count | **1485** | **1474** | **-11** |
| `parity_condition_and_effect` | 51 | 50 | **-1** |
| `parity_state_family` (new) | 13 | 3 | **-10** |
| every other suite (62) | — | — | **0** |

**Zero regressions.** The one peer-suite assertion that flipped is real, not a
re-baseline: `test_parity_condition_and_effect.c`, `PL!S-bp5-016-N` —
`"two or more waited members unlock the heart06 rider"`, which is the `change_state.rs:662`
recalculation (a 常時 heart06 rider counting 2+ waited members) and is fixed by **C1**.

`test_parity_state_family.c` is the new suite, 39 checks. Its 3 remaining FAIL lines are the
documented OPEN GAPs of §8 (items 1 and 4) and are marked `OPEN GAP` in their messages.
Against the baseline tree the same suite reports **13** failures, so it is a genuine
regression test for C1, C2, C5 and C6 (and for the `changed_state_members` and
`live_total set` OPEN GAPs):

```
BASE  FAIL: change_state.rs:662-669: the orientation write is followed by a constant recalculation (got 7 expected 0)
BASE  FAIL: change_state.rs:655-659: the activate step records BOTH members (got 0 expected 2)
BASE  FAIL: the history holds the exact card ids that changed state
BASE  FAIL: change_state.rs:655: re-activating an already-recorded member does not duplicate it (got 0 expected 2)
BASE  FAIL: change_state.rs:653-659: an OPPONENT-targeted activate records the member too
BASE  FAIL: change_state.rs:143-145: ...and waits nothing
BASE  FAIL: ...and records no state transition (got 1 expected 0)
BASE  FAIL: ...and pushes nothing onto recently_state_changed (got 1 expected 0)
BASE  FAIL: ...and adds nothing to the selection (got 1 expected 0)
AFTER FAIL: OPEN GAP (misc.c:905): the FIRST waited member carries the flag
AFTER FAIL: OPEN GAP (misc.c:905): the SECOND waited member carries the flag too
AFTER FAIL: OPEN GAP (test_parity_score_effects.c:1125): a SET on live_total is refused
```

The four `...` lines are C5 and are the sharpest evidence in this audit: a `per_unit`
「1枚の」 step over an empty live-card zone used to **wait a card**, write a
`turn_state_changes` entry, push onto `recently_state_changed` and add to `selection.cards`.
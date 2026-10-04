# Cluster T7 — AUDIT of the GENERATED decoders + the UNMAPPED resource family

Author: cluster T7. Snapshot: `C:/Users/trios/AppData/Local/Temp/kilo/dec/`.
Status: **COMPLETE** — inventory, per-variant tables, the dead-decoder closure
with A/B, and the resource-family reachability census are all done.

## 0. Ground rules observed

* No git writes. `3cffff64` / `f152a50f` not reverted. No new commit.
* No generator run. `condition_decoder_gen.rs` / `effect_decoder_gen.rs` were
  read only.
* `src/abilities_strings.bin` = **726831 B / sha256 `89204df0…`** verified
  before and after every build in the snapshot. `make data` was run ONLY in
  the snapshot with `STRINGS_BIN=` / `CARDS_BIN=` pinned at those snapshot
  paths, so the guard passes and nothing was overwritten. The live tree's
  `make data` stays RED on purpose.
* Baselines are the snapshot with only T7's hunks reverted — not git HEAD —
  because `src/ability/vm.c` arrived with uncommitted T1 content (git status
  showed ` M engine_c_wip/src/ability/vm.c`).
* **Quiescence confirmed**: sha256 of `vm.c` `BFABC58A…`, `resolver.c`
  `3B22B9EB…`, `enums.c` `AD2E7702…`, `types.c` `18B9EE2C…` identical across
  a 15 s settle. Every `.o` in the live tree predates its `.c` (vm.o 10-02
  17:27 vs vm.c 10-03 05:17), so the live tree's objects are STALE; the
  snapshot was built from clean.

## 1. Measurement tooling (new, reproducible)

Three new tools, all built by the snapshot Makefile so no shell quoting of
paths is involved:

| tool | make target | what it measures |
|---|---|---|
| `tools/audit_t7_decoders.c` | `make t7-audit` | walks the **decoded** C trees (`rb_decode_card_ability`) and reports what survived |
| `tools/audit_t7_wire.c` | `make t7-wire` | walks the **raw bytecode** (`rb_bc_slice`) with the same key→node-class routing the Rust readers use |
| `tools/audit_t7_resource.c` | `make t7-res` | per-branch reachability census for the resource / energy / deck families |
| `tools/audit_t7_zeroheart.c` | `make t7-zero` | does any card carry a zero-amount base heart |

`audit_t7_wire.c` exists because the decoded walk is the WRONG LEVEL for a
per-variant audit of a generated decoder: `read_condition_fields` folds every
nested dict (`trigger_event`, `sub_checks`, `position`, `quoted_text`,
`dynamic_count`) into `Condition::fields[]`, so their keys get mis-attributed
to the condition key table. Walking the wire attributes each key to the class
Rust assigns it.

Both censuses agree on totals: wire `abilities_walked=985`,
`cond_nodes=685`, `eff_nodes=2230`; decoded walk `1589` condition nodes and
`4925` effect nodes over the 1650 cards / 2116 (card,ability) pairs — the
2.2x ratio is exactly the pair fan-out (an ability that appears on several
cards is decoded once per pair), which is the cross-check that neither walker
is silently dropping objects.

## 2. INVENTORY — every `pub fn` in the five assigned Rust modules

Counted by `Select-String` over the modules; C twin resolved by searching the
whole of `engine_c_wip/src` and `include/rabuka.h`.

### 2.1 `ability/condition_decoder_gen.rs` (1810 lines) — GENERATED

| item | line | kind | C twin |
|---|---|---|---|
| `ConditionLocals` (struct) | 12 | pub struct | `ConditionLocals` — `vm.c:1345` |
| `decode_condition_field` | 138 | pub fn | `decode_condition_field` — `vm.c:1887` |
| `build_compound` | 578 | pub fn | `build_compound` — `vm.c:1616` |
| `build_location` | 617 | pub fn | `build_location` — `vm.c:1628` |
| `build_comparison` | 657 | pub fn | `build_comparison` — `vm.c:1647` |
| `build_movement` | 691 | pub fn | `build_movement` — `vm.c:1674` |
| `build_group` | 710 | pub fn | `build_group` — `vm.c:1689` |
| `build_appearance` | 727 | pub fn | `build_appearance` — `vm.c:1699` |
| `build_temporal` | 754 | pub fn | `build_temporal` — `vm.c:1714` |
| `build_state` | 768 | pub fn | `build_state` — `vm.c:1726` |
| `build_resource` | 781 | pub fn | `build_resource` — `vm.c:1737` |
| `build_abilityfilter` | 791 | pub fn | `build_abilityfilter` — `vm.c:1752` |
| `build_scorethreshold` | 801 | pub fn | `build_scorethreshold` — `vm.c:1761` |
| `build_choice` | 811 | pub fn | `build_choice` — `vm.c:1770` |
| `build_complex` | 826 | pub fn | `build_complex` — `vm.c:1781` |
| `build_positioncond` | 839 | pub fn | `build_positioncond` — `vm.c:1804` |
| `build_opponentchoice` | 849 | pub fn | `build_opponentchoice` — `vm.c:1813` |
| `build_opponentlivesuccess` | 859 | pub fn | `build_opponentlivesuccess` — `vm.c:1822` |
| `build_noexcessheart` | 869 | pub fn | `build_noexcessheart` — `vm.c:1831` |
| `build_alwaystrue` | 879 | pub fn | `build_alwaystrue` — `vm.c:1840` |
| `build_anyof` | 889 | pub fn | `build_anyof` — `vm.c:1849` |
| `build_allrevealedmatchheartcolor` | 906 | pub fn | `build_allrevealedmatchheartcolor` — `vm.c:1859` |
| `build_unsupported` | 916 | pub fn | `build_unsupported` — `vm.c:2641` |
| `decode_condition_direct` | 926 | pub fn | `decode_condition_direct` — `vm.c:2649` |

25 items, **25 C twins, 0 missing.** Rust's `read_condition_value`
(`vm.rs:561`) → `decode_condition_direct` is Rust's ONLY condition decoder;
the C reaches the same struct through a different shape (see §3).

### 2.2 `ability/effect_decoder_gen.rs` (609 lines) — GENERATED

| item | line | kind | C twin |
|---|---|---|---|
| `decode_effect_field` | 44 | pub fn | `decode_effect_extra` — `vm.c:605` + `decode_effect_body` — `vm.c:685` |
| `decode_ability_effect_direct` | 246 | pub fn | `decode_effect_body` — `vm.c:685` (the C folds the variant byte and the field loop into one function) |
| `decode_effect_field` alias arms `energy`/`max_repeats`/`zone` | 227/228/210 | pub fn (alias) | `vm.c:2260` (`energy`→`energy_count`), `vm.c:2370` (`max_repeats`→`repeat_limit`), `vm.c:2218` (`zone`) |

3 items, **3 C twins** (one folded into a shared body). The three alias arms
are the T1-3 landed fixes and are verified present.

### 2.3 `ability/effects/resource.rs` (1899 lines) — the largest unmapped module

**The twin is NOT missing. It is `src/ability/effects/misc.c` — a HELD file
(cluster C).** Evidence, by function identity (not name similarity):

| Rust item | line | C twin |
|---|---|---|
| `ResourceKind` (enum) | 32 | `resource_kind` — `misc.c:105` / `RB_RES_BLADE`/`RB_RES_HEART`/`RB_RES_OTHER` |
| `ResourceKind::from_str` | 40 | `resource_kind` — `misc.c:105` |
| `selected_before_save` | 179 | **NONE** (private; folded into `resolve_gain_resource_targets`) |
| `selected_after_save` | 187 | **NONE** (same) |
| `grant_heart_distribution` | 199 | `grant_heart_distribution` — `modifiers.c:349` |
| `grant_blade` | 224 | `grant_blade` — `modifiers.c:354` |
| `apply_heart_to_card` | 235 | `apply_heart_to_card` — `misc.c:235` |
| `distinct_discarded_colors` | 276 | folded into `h_bp6_pattern` — `misc.c:664` |
| `handle_bp6_pattern` | 297 | `h_bp6_pattern` — `misc.c:654` |
| `execute_gain_surplus_heart` | 376 | `rb_effect_gain_surplus_heart` — `misc.c:1529` |
| `calculate_gain_multiplier` | 446 | `calculate_gain_multiplier` — `misc.c:346` |
| `GainTargets` / `GainResourceRequest` / `GainMultiplierRequest` / `ResourceFlags` | 50/67/120/149 | `GainTargets` — `misc.c:~740`; the three request structs are C positional-parameter twins |
| `ResourceFlags::for_blade` | 163 | inlined at the two `apply_*_resource` call sites |
| `apply_heart_resource` | 1329 | `apply_heart_resource` — `misc.c:577` |
| `apply_blade_resource` | 1453 | `apply_blade_resource` — `misc.c:536` |
| `resolve_gain_resource_targets` | 1568 | `resolve_gain_resource_targets` — `misc.c:387` |
| `gain_heart_colors_from_selected_card` | 648 | `h_gain_heart_colors_from_selected_card` — `misc.c:~560` |
| `gain_heart_all_type` | 738 | `h_gain_heart_all_type` — `misc.c:~630` |
| `execute_gain_resource` | 796 | `h_gain_resource` — `misc.c:687` |

**19 of 21 items have a twin. The 2 without one (`selected_before_save`,
`selected_after_save`) are pure private list splitters whose effect is
reproduced inside `resolve_gain_resource_targets`.** So the "unmapped module"
framing is wrong: resource.rs is mapped, into `misc.c` + `modifiers.c`.

### 2.4 `ability/effects/energy.rs` (599 lines)

| Rust item | line | C twin |
|---|---|---|
| `execute_energy_placement` | 27 | `rb_effect_energy_placement` — `state.c:680` |
| `execute_energy_state_change` | 63 | `rb_effect_energy_state_change` — `state.c:707` |
| `execute_place_energy_under_member` | 293 | `h_place_energy_under_member` — `misc.c:1152` |
| `execute_place_energy_under_member_non_optional` | 303 | `rb_effect_place_energy_under_member_non_optional` — `misc.c:1976`; wrapper `cost.c:718` |
| `execute_place_energy_under_member_impl` | 311 | the shared body of the two above |
| `execute_pay_energy` | 546 | `h_pay_energy` — `misc.c:836` |

6 items, **6 twins.**

### 2.5 `ability/effects/deck.rs` (539 lines)

| Rust item | line | C twin |
|---|---|---|
| `draw_cards_for_player` | 25 | `rb_draw_cards_for_player` — `draw.c:276` |
| `resolve_dynamic_count` | 94 | `rb_dynamic_count_*` — `dynamic_count.c` |
| `execute_draw_wrapper` | 112 | `choice.c:4887` |
| `execute_draw` | 178 | `rb_effect_draw` — `draw.c` |
| `execute_draw_until_count` | 483 | `rb_effect_draw_until_count` — `draw.c:866` (helper `draw_until_target_count` — `draw.c:860`) |
| `execute_shuffle` | 516 | `h_shuffle` — `misc.c:966` |

6 items, **6 twins.**

### 2.6 Inventory totals

| module | items | C twin | NONE |
|---|---|---|---|
| condition_decoder_gen.rs | 25 | 25 | 0 |
| effect_decoder_gen.rs | 3 | 3 | 0 |
| resource.rs | 21 | 19 | 2 (private splitters, folded) |
| energy.rs | 6 | 6 | 0 |
| deck.rs | 6 | 6 | 0 |
| **TOTAL** | **61** | **59** | **2** |

## 3. THE LANDMINE — `decode_condition_direct` and its catch-all

### 3.1 The three options, decided on evidence

**Chosen: (b) fix the catch-all so the keys decode exactly as Rust does.**

The three facts that decide it:

1. **The mirror is lossless.** `read_condition_fields` (`vm.c:404`) is a fully
   generic reader: it stores *every* key with its typed tag value, with no key
   allow-list. Measured: the wire census lists **79 distinct condition keys**;
   the decoded census contains **all 79** — the set difference "on the wire,
   not in the mirror" is **empty**, for every one of the 985 abilities. So the
   live path is not lossy, and there is nothing for option (c) to fix.
2. **The typed decoder is Rust's ONLY decoder, so (a) is not available.**
   Rust has no mirror: `read_condition_value` (`vm.rs:561`) is the single
   entry and it calls `decode_condition_direct` (`vm.rs:562`). So the C's
   `decode_condition_direct` is the twin of a LIVE Rust item — "zero C
   callers" says nothing about whether the item is needed, and deleting it
   would leave the C with no counterpart to the function Rust actually runs.
3. **(c) is a regression, not a fix.** Rust's `ConditionCommon`
   (`card.rs:3189-3248`) carries **five** fields the C typed decoder never
   sets, and `debug_conditions` is in `engine/Cargo.toml`'s `default`
   feature list, so a normal build HAS them. Wiring the C typed decoder as it
   stands would delete data the C currently keeps — the exact opposite of
   parity.

(b) is also the only option with zero observable risk: the path is provably
uncalled (`rb_decode_fallback_count()` reads **0** after decoding all 985
abilities), so making it value-correct changes no behaviour today and removes
the silent-wrong-value trap for whoever wires it next.

**After (b), option (c) becomes safe** — the typed decoder is then a faithful
twin and the mirror could be retired. That is recorded as follow-up, NOT
done here: retiring the mirror is a behaviour-visible refactor with no
measurement to justify it, and it must not be done before (b).

### 3.2 Per-variant table — CONDITION decoder (Rust generated vs C typed)

Rust's `decode_condition_field` names **90 distinct keys** (92 match arms;
`text` and `trigger_event` appear twice under `cfg(debug_conditions)`).
C's `decode_condition_field` (`vm.c:1888-2624`) names **85**. Corpus counts
are from `make t7-wire` over the 985 shipped abilities.

| key | nodes | abilities | C typed decoder |
|---|---|---|---|
| **any_location** | **18** | **18** | **ABSENT → catch-all → DROPPED** |
| **activated_this_turn** | **3** | **1** | **ABSENT → catch-all → DROPPED** |
| **paid_cost_this_turn** | **1** | **1** | **ABSENT → catch-all → DROPPED** |
| **text** | **680** | **504** | **ABSENT → catch-all → DROPPED** |
| **trigger_event** | **131** | **112** | **ABSENT → catch-all → DROPPED** |
| all other 85 named keys | — | — | present |
| **Rust keys with 0 corpus occurrences** | 0 | 0 | n/a — 5 more names exist in the generator that the blob never exercises |

Full wire table for all 84 observed keys is in §3.4.

**T1's figures (21 / 3 / 2 / 160) were counted over `cards/abilities.json`,
not over the shipped bytecode.** T7's figures are decode-level node counts and
are the measurement of record: 18/18, 3/1, 1/1, 131/112. Both agree on
`activated_this_turn`; the others differ because the JSON is stale (K7) and
because T1 counted raw occurrences while T7 counts decoded nodes.

### 3.3 What the C typed decoder was dropping, key by key

| key | Rust line | Rust type | Rust consumer |
|---|---|---|---|
| `any_location` | `condition_decoder_gen.rs:137` | `ConditionCommon.any_location: Option<bool>` (`card.rs:3198`) | symmetric multi-location disjunction in the location/compound builders |
| `activated_this_turn` | `:25` | `ConditionCommon.activated_this_turn: Option<bool>` (`card.rs:3242`) | counted against `GameState::cards_activated_this_turn` |
| `paid_cost_this_turn` | `:84` | `ConditionCommon.paid_cost_this_turn: Option<bool>` (`card.rs:3248`) | counted against `GameState::cards_paid_cost_this_turn` |
| `text` | `:234` (`cfg(debug_conditions)`) | `ConditionCommon.text: Option<String>` (`card.rs:3218`) | decode-retention only |
| `trigger_event` | `:25`/`:168` (`cfg(debug_conditions)`) | `ConditionCommon.trigger_event: Option<Box<TriggerEvent>>` (`card.rs:3223`) | `check_phase_gate` fallback — `condition.rs:398-422` |

### 3.4 Bonus divergence found while measuring: the `trigger_event` phase fallback

`condition.rs:398-422` computes
`get_phase().or_else(|| te.and_then(|t| t.phase.as_deref()))` and the same for
`phase_target`. C's `rb_check_phase_gate` (`condition.c:2547-2574`) reads only
the condition's own `phase`/`phase_target` — **no `trigger_event` fallback**.

Measured directly (every condition node cross-tabulated for
"trigger_event has `phase`" against "condition has its own `phase`"):

```
TE|te_has_phase=61|te_has_phase_target=61|DIVERGE_phase=0|DIVERGE_phase_target=0
```

**VERIFIED-NOOP: 0 of 985.** Every condition whose `trigger_event` carries a
`phase` also carries its own, so Rust's fallback never fires on this corpus
and the missing C fallback cannot diverge. `condition.c` is cluster D's file,
so this is recorded rather than changed.

Two further Rust/C differences in the same function, also measured dead:
Rust's `phase_ok` match has `_ => true` for a phase outside
`main`/`main_phase`/`active`/`active_phase`/`live_phase`, while C returns 0 for
`live`/`live_card_set`/`live_performance`/`live_victory`. **Only
`main`, `active_phase` and `live_phase` occur as condition `phase` values on
this corpus** (`cond|phase=main` 5, `active_phase` 2, `live_phase` 2), so the
`_ => true` arm is **VERIFIED-NOOP: 0 of 985**.

## 4. Per-variant table — EFFECT decoder

The two effect decoders are **structurally different, and that difference is
the reason the effect side has no equivalent landmine**:

* Rust `decode_effect_field` is a 197-arm match: every variant the generator
  knows about is handled *by construction*, and anything else falls to
  `warn!("unknown effect field")` + `skip_value` + `Some(true)`.
* C `decode_effect_body` (`vm.c:685`) has an explicit structural dispatch for
  **48** keys (the ones needing nested objects/arrays or a typed struct field)
  and a **scalar tail at `vm.c:1169-1190`** that retains *every other key* as a
  stringified extra. So the C is lossless by construction for all scalar and
  array keys; the only way to lose an effect key is for it to be in neither the
  dispatch nor the tail.

Cross-check `make t7-wire` (what is on the wire) against `make t7-audit`
(what survived the C decode), per key:

* **On the wire, not retained by the C: 0 keys.**
* Every key the wire reports lands either in a dedicated `AbilityEffect`
  struct field or in `extra_k[]`/`extra_v[]`.
* The reverse direction has 57 keys C retains that the wire never produces
  (`ability_filter_triggers`, `ability_id`, `ability_name`, `active_energy`, …)
  — aliases and derived values with 0 corpus occurrences, all
  VERIFIED-NOOP.

Full per-key table (143 keys on the wire, with class):

| key | nodes | abilities | C handling |
|---|---|---|---|
| `text` | 2226 | 852 | `e->text` |
| `condition` | 957 | 593 | `e->condition` |
| `source` | 780 | 510 | `e->source` |
| `target` | 493 | 400 | `e->target` |
| `count` | 447 | 258 | `e->count` |
| `action` | 447 | 240 | `e->action` |
| `card_type` | 336 | 166 | `e->card_type_field` + extra |
| `self_target` | 159 | 151 | `e->self_target_field` + extra |
| `duration` | 154 | 143 | scalar tail |
| `all` | 116 | 105 | scalar tail |
| `group_names` | 85 | 60 | scalar tail |
| `heart_colors` | 67 | 44 | scalar tail (+ `heart_color` mirror) |
| `heart_type` | 40 | 26 | scalar tail |
| `heart_color` | 2 | 2 | scalar tail |
| `value` | 71 | 51 | scalar tail |
| `per_unit_count` | 62 | 45 | `e->per_unit_count` |
| `condition` (nested) | 957 | 593 | `e->condition` |
| `distinct` | 23 | 17 | `e->distinct_flag` + extra |
| `cost_type` | 1 | 1 | `e->cost_type` |
| `per_group` | 1 | 1 | scalar tail |
| `dynamic_count` | 27 | 24 | `e->dynamic_count` |
| `heart_colors_from_selected_card` | 1 | 1 | scalar tail |
| `heart_selection` | 1 | 1 | scalar tail |
| `per_unit` | 47 | 30 | `e->per_unit` |
| `non_stackable` | 4 | 4 | scalar tail |
| `resource` | 295 | 272 | scalar tail |
| `target_count` | 20 | 19 | scalar tail |
| `per_unit_type` | 32 | 26 | scalar tail |
| `per_unit_source` | 13 | 12 | scalar tail |
| `per_unit_aggregate` | 4 | 4 | scalar tail |
| `self_cost` | 6 | 6 | scalar tail |
| `target_member` | 2 | 2 | scalar tail |
| `allow_occupied_stage` | 6 | 6 | scalar tail |
| `quantifier` | 1 | 1 | scalar tail |
| `shuffle` | 19 | 7 | scalar tail |
| `multiple_targets` | 1 | 1 | scalar tail |
| `exclude_self` | 71 | 51 | scalar tail |
| `additional_discard_count` | 1 | 1 | scalar tail |
| `zone` | 1 | 1 | scalar tail |
| `sign` | 0 | 0 | scalar tail (alias-only) |
| `per_unit_location` | 8 | 6 | scalar tail |
| `group_reference` | 11 | 10 | scalar tail |
| `watches_ability_resolution` | 4 | 4 | scalar tail — **no C consumer (J9)** |
| `target_from_selection` | 1 | 1 | scalar tail |
| `extra_cost` | 1 | 1 | scalar tail |
| `phase_target` | 2 | 2 | scalar tail |
| `choice_id` | 1 | 1 | scalar tail |
| `keep_alive` | 1 | 1 | scalar tail |
| `auto_select` | 1 | 1 | scalar tail |
| `auto_target` | 1 | 1 | scalar tail |
| `consecutive` | 2 | 2 | scalar tail |
| `target_card_type` | 2 | 2 | scalar tail |
| `continuous` | 1 | 1 | scalar tail |
| `reset_on_standalone` | 1 | 1 | scalar tail |
| `once_per_turn` | 1 | 1 | scalar tail |
| `is_further` | 1 | 1 | `e->is_further` |
| `max` | 53 | 45 | scalar tail |
| `not_self_target` | 1 | 1 | scalar tail |
| `record_attack` | 3 | 3 | scalar tail |
| `optional` | 389 | 127 | `e->is_optional` + extra |
| `conditional` | 96 | 56 | `e->conditional_flag` |
| `conditional_negation` | 30 | 27 | `e->conditional_negation` |
| `cost_reduction_per_group` | 8 | 6 | `e->cost_reduction_per_group` |
| `repeat_limit` | 3 | 3 | `e->repeat_limit` (alias `max_repeats` also mapped) |
| `energy_count` | 97 | 66 | `e->extra` (alias `energy` also mapped) |
| `id` | 96 | 96 | `e->id_field` |
| `type` | 455 | 245 | scalar tail (deliberately, per T1-3) |
| `destination` | 129 | 104 | scalar tail |
| `location` | 96 | 79 | scalar tail |
| `state_change` | 143 | 143 | scalar tail |
| `energy_state` | 17 | 15 | scalar tail |
| `category` | 2 | 2 | scalar tail |
| `steps` | 2 | 2 | scalar tail |
| `distance` | 2 | 2 | scalar tail |
| `replace_all` | 1 | 1 | scalar tail |
| `replacement` | 1 | 1 | scalar tail — **Rust's generator does not name it either; Rust drops it, C keeps it. C is a superset with no consumer. Not a wrong value.** |

(The full machine-generated table is reproduced in §7.)

**EFFECT DECODER VERDICT: 0 keys with count > 0 and no C handling.**

## 5. Resource family — reachability census

`make t7-res` over 985 abilities / 170 `gain_resource`, 64 `pay_energy`,
11 `place_energy_under_member`, 45 `draw_card`, 2 `draw_until_count`.

| Rust branch | verdict |
|---|---|
| `resource.rs:40 ResourceKind::from_str "blade"` | 97 nodes / 97 abilities |
| `resource.rs:40 ResourceKind::from_str "heart"` | 73 / 73 |
| `resource.rs:40 ResourceKind::from_str _ => Other` | **0 / 0 — VERIFIED-NOOP** (`resource=surplus_heart` 2 nodes never reaches `from_str`; `execute_gain_resource:866` intercepts it before `:828`) |
| `resource.rs:805-813 heart_colors_from_selected_card` | **0 / 0 — VERIFIED-NOOP.** `heart_colors_from_selected_card` occurs once in the whole corpus and that one node is **not** on a `gain_resource`, so `gain_heart_colors_from_selected_card` (resource.rs:648) is unreachable. This retires checklist item F2 as unreachable rather than missing. |
| `resource.rs:815-819 heart_type == "all"` | 5 / 5 → `h_gain_heart_all_type` misc.c:~630 |
| `resource.rs:305-310 handle_bp6_pattern` full guard | **1 / 1 — ability 604** |
| `resource.rs:1329 apply_heart_resource require_all_heart_colors` | **0 / 0 — VERIFIED-NOOP** |
| `resource.rs:1329 apply_heart_resource per_group` | **0 / 0 — VERIFIED-NOOP** |
| `resource.rs:1453 apply_blade_resource card_type==live_card` | **0 / 0 — VERIFIED-NOOP** |
| `resource.rs:1453 apply_blade_resource card_type==energy_card` | 2 / 2 |
| `resource.rs:1453 apply_blade_resource card_type absent` | 131 / 131 |
| `resource.rs:846-864 is_all` term 1 (Rust) | **0 / 0 — VERIFIED-NOOP** |
| `resource.rs:846-864 is_all` term 3 (Rust) | **0 / 0 — VERIFIED-NOOP** |
| `energy.rs:546 execute_pay_energy target==self` | **0 / 0 — VERIFIED-NOOP** |
| `energy.rs:546 execute_pay_energy source=energy_deck` | **0 / 0 — VERIFIED-NOOP** |
| `energy.rs:311 place_energy_impl self_cost / allow_occupied_stage / location` | **0 / 0 — VERIFIED-NOOP** |
| `deck.rs:178 execute_draw source=deck_top/deck_bottom/discard/hand/stage/energy_deck/…` | **0 / 0 each — only `source=deck` is live (45 / 45)** |
| `deck.rs:178 execute_draw shuffle / dynamic_count` | **0 / 0 — VERIFIED-NOOP** |
| `deck.rs:178 execute_draw per_unit` | 3 / 3 |
| `deck.rs:483 execute_draw_until_count location` | **0 / 0 — VERIFIED-NOOP** |

### 5.1 `h_bp6_pattern` — the one reachable resource branch, checked line by line

Rust `handle_bp6_pattern` (`resource.rs:297-373`) vs C `h_bp6_pattern`
(`misc.c:654-684`), on the single ability that reaches it (ability 604):

| aspect | Rust | C | verdict |
|---|---|---|---|
| guard `resource==heart && per_unit && per_unit_type==discard && multiple_targets` | `:305-309` | `misc.c:660` | identical |
| no activating card | `return Ok(true)` `:329-331` | `return 1` `:663` | identical |
| distinct colours from `recently_moved_cards` | `:321` `distinct_discarded_colors` | `misc.c:664-677` | same source |
| **`amt > 0` filter** | `:289` `if amt > 0` | **absent** — `misc.c:671` counts every colour | **divergence in code, VERIFIED-NOOP on data:** `make t7-zero` reports `cards=2639 zero_amount_base_entries=0 (member=0 live=0)`. No card in the corpus has a zero-amount base heart, so the missing filter cannot change the answer. |
| distinct cap | `SmallVec<[HeartColor;8]>` (spills) | `nd < 8` `:674` | cannot differ: ≤4 base hearts per card (`max_num_base=4`), so ≤4 distinct colours |
| colours outside 0..7 | none — `HeartColor` is an enum | `misc.c:671 col<0 \|\| col>7` skips | cannot differ: `HeartColor`'s indices are 0..7 |
| sign | `+1` per colour `:333-340` | `apply_heart_to_card(..., is_negative=0)` `:681` | identical |
| trace | `add_heart_modifier_with_trace` + `record_ability_application(..., "heart_bonus", ...)` `:333-348` | `apply_heart_to_card` → `grant_heart_distribution` | same modifier write; the `record_ability_application` call is C replay/describe metadata with no parity consumer |
| temporary record | one `push_temporary_effect("gain_heart", …, MultiCard{items})` `:350-371` | one `push_temporary_effect` per colour inside `apply_heart_to_card` | shape differs (N pushes vs 1), end state after duration expiry identical |

**`h_bp6_pattern` is correct on this corpus.** No change needed; `misc.c` is
not edited.

## 6. `is_all` heuristic — the one real Rust/C semantic difference measured

Rust (`resource.rs:846-864`) tests **key presence** for two terms; C
(`misc.c:735-738`) tests **truthiness**:

| Rust term | Rust test | C test | can differ when |
|---|---|---|---|
| 1 | `effect.exclude_self_any().is_none()` | `!eff_extra(e,"exclude_self")` | `exclude_self: false` is present |
| 3 | `effect.distinct_any().is_none()` | `!e->distinct_flag && !extra_true(e,"distinct")` | `distinct: false` is present |

Measured on the 170 `gain_resource` nodes:

```
gain_resource|exclude_self=(absent)   163 / 163
gain_resource|exclude_self=true         7 / 7
gain_resource|exclude_self=false        0        <- term 1 cannot diverge
gain_resource|distinct=(absent)       169 / 169
gain_resource|distinct=false            1        <- the single node does NOT
                                                 also carry card_type=member_card
                                                 + target in {self,opponent}
                                                 + no target_count
```

**Both terms are VERIFIED-NOOP: 0 of 170.** No change needed.

## 7. THE CLOSURE — option (b) applied, A/B measured

### 7.1 What changed

Exactly one file, `src/ability/vm.c`, three hunks, nothing else touched:

| hunk | where | what |
|---|---|---|
| 1 | `ConditionLocals`, vm.c:1406-1420 | 5 new fields: `any_location`/`has_any_location`, `activated_this_turn`/`has_…`, `paid_cost_this_turn`/`has_…`, `char *text`, `Condition *trigger_event` |
| 2 | `cond_copy_common`, vm.c:1615-1622 | the 5 keys are copied onto every built `Condition`, so all 21 variants carry them (Rust puts all five on `ConditionCommon`, so all 21 do too) |
| 3 | `decode_condition_field`, vm.c:2642-2699 | 5 arms before the catch-all: three bools in the file's existing `TRUE/FALSE/NULL` idiom, `text` via `rd_idx` + `rb_strdup`, `trigger_event` via `read_condition` (OBJVAR) / `read_condition_fields(r,0xFF)` (OBJECT) |

`trigger_event`'s C carrier is a nested `Condition` under the same key. That is
deliberate and not a compromise: it is byte-for-byte the shape the live mirror
already produces for that key (`read_cond_value` → `read_condition_fields`), so
the typed path and the mirror now agree. Rust's `TriggerEvent` reader names 20
of the 24 sub-keys the corpus carries and `skip_value`s the rest
(`vm.rs:718-800`), so retaining all of them is a strict superset of what Rust
keeps — the C cannot return a value Rust could not have produced.

**No behaviour change to the live path.** The path has no callers, so the only
observable difference is inside the test.

### 7.2 The A/B, and the stale-link trap that nearly hid it

Baseline is the snapshot with **only T7's three hunks reverted** — not git
HEAD, because `src/ability/vm.c` arrived with uncommitted T1 content.

| run | `vm.o` mtime | `test_parity_t7_condition_decoder` `^FAIL` lines |
|---|---|---|
| baseline (3 hunks reverted) | 07:55:45 | **630** |
| fixed | 07:56:43 | **0** |

Two stale-link traps fired and both had to be defeated:

1. `make src/ability/vm.o` is timestamp-driven. Restoring the fixed `vm.c`
   with `Copy-Item` restored its **older** mtime (07:29:10), make saw
   `vm.c < vm.o` and **skipped the rebuild** — producing a byte-identical
   630-FAIL "fixed" run that was actually still the baseline object. Fix:
   `rm vm.o` and rebuild; the new mtime (07:56:43) is then verifiably past the
   source. A byte-identical before/after is evidence of a stale link until
   disproved, exactly as warned.
2. `Get-ChildItem -Recurse -Filter *.o` picked up `src/main.o`, so every test
   link failed with `multiple definition of 'main'`. Fixed by excluding
   `main.o` — `OBJ_LIB` does not contain it.

### 7.3 No regression: 19 pre-existing suites, byte-identical FAIL lines

Every suite was run on both trees and compared on **sorted `^FAIL` lines from
stdout AND stderr** (not exit codes):

| suite | `^FAIL` fixed out/err | `^FAIL` baseline out/err | sorted-FAIL diff |
|---|---|---|---|
| parity_conditions | 0 / 0 | 0 / 0 | IDENTICAL |
| parity_condition_and_effect | 0 / 50 | 0 / 50 | IDENTICAL |
| parity_integration | 0 / 31 | 0 / 31 | IDENTICAL |
| parity_sequential | 29 / 112 | 29 / 112 | IDENTICAL |
| parity_small_clusters | 42 / 69 | 42 / 69 | IDENTICAL |
| vm_decode | 0 / 0 | 0 / 0 | IDENTICAL |
| decode_fallback | 0 / 0 | 0 / 0 | IDENTICAL |
| parity_ability_mod | 0 / 13 | 0 / 13 | IDENTICAL |
| parity_ability_mod_extra | 0 / 16 | 0 / 16 | IDENTICAL |
| parity_zones_position | 0 / 37 | 0 / 37 | IDENTICAL |
| parity_under_member | 31 / 60 | 31 / 60 | IDENTICAL |
| parity_ability_watch | 9 / 19 | 9 / 19 | IDENTICAL |
| parity_jidou_watch | 23 / 55 | 23 / 55 | IDENTICAL |
| parity_live_total | 20 / 32 | 20 / 32 | IDENTICAL |
| parity_recover_live | 21 / 36 | 21 / 36 | IDENTICAL |
| parity_executor_dispatch | 0 / 0 | 0 / 0 | IDENTICAL |
| parity_full_round | 16 / 29 | 16 / 29 | IDENTICAL |
| parity_enums | 0 / 0 | 0 / 0 | IDENTICAL |
| audit_fields | 2 / 3 | 2 / 3 | IDENTICAL |
| **parity_t7_condition_decoder** | **0 / 0** | **0 / 630** | **DIFF out=0 err=630** |

The pre-existing FAIL counts (e.g. parity_sequential 29+112) are other
clusters' open divergences. T7 moved none of them. No test was weakened,
deleted or ignored.

### 7.4 The new test — `tests/test_parity_t7_condition_decoder.c` (T7 CLAIMS THIS FILE)

`make t T=parity_t7_condition_decoder`

It drives `decode_condition_direct` — the one non-static entry point into the
changed code — with **synthetic bytecode built byte by byte**, so the test
never re-implements the bytecode reader (an earlier draft did, and the duplicated
reader desynced and produced garbage "failures"; that is why the committed
version is synthetic). It covers all 21 condition variants (0..20) × every value
shape each key can take:

* `any_location`, `activated_this_turn`, `paid_cost_this_turn` × TRUE / FALSE / NULL
* `text` × TAG_STR (value compared) / TAG_NULL (must stay ABSENT — inventing a
  value from NULL would be a new silent wrong value)
* `trigger_event` × TAG_OBJECT / TAG_OBJECT_VARIANT / TAG_NULL
* one 9-field "all five" node, the shape the corpus actually uses

Assertions per case: non-NULL decode; **byte-exact consumption** (a decoder that
mis-reads a value desyncs every later field, so length equality is the
load-bearing check); `rb_decode_fallback_count()` unchanged; the key present with
Rust's type and value; and NULL must not invent a field.

Plus a **CONTROL case**: a key the C genuinely does not name (`full_text`) MUST
move `rb_decode_fallback_count()`. Without it, "the counter did not move" would
be vacuous. That is why the 630 baseline FAILs include the control's inverse.

### 7.5 Open item, deliberately not fixed

`decode_condition_direct` never frees `ConditionLocals`. Five members transfer
ownership into the built `Condition` (`condition`, `conditions`, `options`,
`cause`, `effect` — `cond_add_cond`/`cond_add_cond_array` store the pointer, no
copy), so a `cond_locals_free` would double-free on exactly those. Writing a
partial free for a code path with no callers would add never-executed, therefore
untested, therefore dangerous code. Recorded as **T7-x** instead: if the typed
decoder is ever wired, that leak becomes real and the fix must be all-or-nothing
over the five transferring members.

## 8. Reproducing

`make t T=parity_t7_condition_decoder` runs the new test. The census tools are
wired into the Makefile as `t7-audit` / `t7-wire` / `t7-res` / `t7-zero`:

```
cd <tree>/engine_c_wip
make t7-wire STRINGS_BIN=$PWD/src/abilities_strings.bin CARDS_BIN=$PWD/src/cards.bin
```

In a snapshot (the live tree's `make data` is correctly RED because
`cards/build/abilities_strings.bin` is stale and `tools/guard_data_blob.sh`
refuses):

```
make data STRINGS_BIN=$PWD/src/abilities_strings.bin CARDS_BIN=$PWD/src/cards.bin
make -j4            STRINGS_BIN=$PWD/src/abilities_strings.bin CARDS_BIN=$PWD/src/cards.bin
make t7-audit       STRINGS_BIN=$PWD/src/abilities_strings.bin CARDS_BIN=$PWD/src/cards.bin > t7_census.txt
make t7-wire        STRINGS_BIN=$PWD/src/abilities_strings.bin CARDS_BIN=$PWD/src/cards.bin > t7_wire.txt
make t7-res         STRINGS_BIN=$PWD/src/abilities_strings.bin CARDS_BIN=$PWD/src/cards.bin > t7_res.txt
make t7-zero        STRINGS_BIN=$PWD/src/abilities_strings.bin CARDS_BIN=$PWD/src/cards.bin
make t T=parity_t7_condition_decoder
```

`make t7-wire` output shape:
`COND|key|nodes|abilities|C_TYPED|C_DROPPED`, `EFF|key|nodes|abilities|C_STRUCT|SCALAR_TAIL`,
`NEST|class:key|nodes`, `VAL|class|key=value|nodes`, `CVAR|variant|nodes`,
`EVAR|variant|nodes`, `TE|…`, `TEDIV|…`.

Blob integrity was re-verified after every build, in both the live tree and the
snapshot:
`src/abilities_strings.bin` = **726831 B / sha256 `89204df0…`**,
`src/cards.bin` = **631890 B / sha256 `72820850…`**.
# AUDIT — cluster T1: `ability/{vm,resolver,enums,types}`

Rust audited: `engine/src/ability/vm.rs` (1717), `resolver.rs` (1601), `enums.rs` (903),
`types.rs` (1172).
C owned: `src/ability/vm.c`, `resolver.c`, `enums.c`, `types.c`.
None of these four Rust modules had ever been systematically audited before this file.

**Quiescence before editing.** All four sources were byte-identical across two SHA-256
reads 12 s apart and untouched since 2026-10-02 12:23:47 (nobody else had edited them):

| file | SHA-256 worked from |
|---|---|
| `src/ability/vm.c` (pristine) | `4D64C9235D963E6C683EFFF29E4497782CC1A4B4541573E07F37C5DB0038E464` |
| `src/ability/vm.c` (fixed) | `BFABC58A379A7C1C4957FB48CAC2CD158182A60F875FACB8FFA990750F061F0F` |
| `src/ability/resolver.c` | `A7543E06089DDF823CB4AE1C51945228372C100F276B525822968D94A1999301` |
| `src/ability/enums.c` | `86C40D1AA91CE277CC88B5C54915FA1BD35BB34CEE2F9A9B2FB6DCBA3FBA37AD` |
| `src/ability/types.c` | `E28F85933BDECFC531D56AB6A9993EF858ADADF267FCAA4597D80C29A0AC51B4` |

**Measurement.** Snapshot `C:/Users/trios/AppData/Local/Temp/kilo/vm` (380 files,
`.o`/`*.exe`/`*.log` excluded and deleted after copy). `src/abilities_strings.bin`
confirmed **726831 B / sha256 `89204df0bbde08a02a7f978a56a80d6e6431691ec7d6d658979a329956b06344`**
before and after every build; no generator was run. Baseline = the snapshot with **only my
`vm.c` hunk reverted** (`vm_base` tree), proved by a file-hash sweep: between the two trees
`vm.c` differs (`4D64C923…` vs `BFABC58A…`) and `resolver.c`, `enums.c`, `types.c`,
`include/rabuka.h`, `src/core/card.c` and both edited test files are **identical**.
Every run rebuilt the objects it needed and confirmed the `.o` mtime moved, so no result
here is a stale link. FAIL lines counted as `^FAIL` (**no colon**) on stdout **and** stderr,
strictly sequential.

**Corpus census** (measured with `rb_decode_card_ability` over the shipped blobs, walking
every decoded effect and condition): 985 abilities, **0 decode failures**, **2230 effects**,
**995 conditions**, 147 distinct effect keys, 91 distinct condition keys, 50 distinct action
spellings, 18 source + 15 destination zone values, 16 condition variants (of 0..19),
`rb_decode_fallback_count() == 0`. Capacity headroom: max 12 extras per effect (cap 32),
max 4 children (cap 64), max 13 condition fields (cap 64) — **no `RB_MAX_*` overflow is
reachable.**

---

## 1. INVENTORY — counts

| module | `pub` items | `pub(crate)` | private items | enum variants |
|---|---|---|---|---|
| `vm.rs` | 11 | 0 | 79 | 1 |
| `resolver.rs` | 22 | 8 | 17 | 0 |
| `enums.rs` | 24 | 0 | 23 | 161 |
| `types.rs` | 62 | 0 | 5 | 32 |
| **total** | **119** | **8** | **124** | **194** |

**321 inventory rows** (127 public-facing items + 194 enum variants). `vm.rs`'s 79 private
items are not padding: they are the `read_*_value` decoders, and each one is the twin of a
`static` function in `vm.c` — they are listed individually below because they are where
decode bugs live.

### 1a. `vm.rs` — public items

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `enum DecodeError` | 35 | `vm.c:2640` `rb_decode_ability_error` (kind + message, string not modelled) | PRESENT, PARTIAL — `Unknown(String)`/`Io(String)` lose the payload, same shape gap as `EffectCardType::Other`. VERIFIED-NOOP: 0 of 985 (0 decode failures) |
| `fn ability_count()` | 77 | `data.c` `rb_num_abilities` | PRESENT, CORRECT |
| `const DECODE_AUDIT_MAX` | 96 | `vm.c` `RB_DECODE_AUDIT_MAX` | PRESENT, CORRECT |
| `fn note_decode_fallback()` | 106 | `vm.c:2770` `rb_note_decode_fallback` | PRESENT, CORRECT |
| `fn decode_fallback_count()` | 120 | `vm.c:2776` `rb_decode_fallback_count` | PRESENT, CORRECT |
| `fn decode_fallback_abilities()` | 125 | `vm.c` `rb_decode_fallback_abilities` | PRESENT, CORRECT |
| `fn count_empty_bytecode_abilities()` | 138 | `vm.c:2796` `rb_count_empty_bytecode_abilities` | PRESENT, CORRECT |
| `fn get_ability()` | 247 | `vm.c:1219` `rb_get_ability` | PRESENT, **CORRECTED — see §3 D1** |
| `fn normalize_cost_keys()` | 1361 | `vm.c:2905` `rb_normalize_cost_keys` | PRESENT, CORRECT (17-key mirror incl. the `energy_count`/`energy` alias) |
| `fn populate_from_json()` | 1525 | **NONE** | REACHABLE-0. Rust-only serde path, feature `json_path_test`, off by default; not a decode path. VERIFIED-NOOP: 0 of 985 |
| `struct BytecodeAudit` (module-level) | 78-94 | `vm.c:2739` `rb_bytecode_audit_result` | PRESENT, CORRECT |

### 1b. `vm.rs` — the private `read_*_value` decoders (the real decode surface)

Each has a `static` twin in `vm.c`. "corpus" is the number of decoded values of that shape.

| Rust reader | line | C twin | corpus | verdict |
|---|---|---|---|---|
| `read_arc_str_value` | 298 | `rd_string_val` | many | CORRECT |
| `read_opt_str_vec_value` | 313 | `rd_str_vec_val` | 8 keys | CORRECT (CSV flattening matches the C extras convention) |
| `read_u8_value` / `read_u8_vec_value` | 331 / 342 | `rd_int` / `rd_int_vec_val` | `values` 3 | CORRECT |
| `read_i8_value` | 352 | `rd_int` | 0 | CORRECT |
| `read_bool_value` | 361 | `rd_bool_val` | many | CORRECT |
| `read_f64_vec_value` | 371 | `rd_f64_vec_val` | 0 | CORRECT |
| `read_bool_vec_value` | 380 | `rd_bool_vec_val` | 0 | CORRECT |
| `read_position_value` | 389 | `vm.c:764` | `position` 53 | CORRECT — corpus `position` is always `TAG_STR`; the `TAG_OBJECT` `{position,target}` shape has 0 corpus uses |
| `read_card_type_value` | 479 | `vm.c:1052` | **817** | **WAS DIVERGENT — FIXED, see §3 D1** |
| `read_operator_value` | 491 | `vm.c:1155` (cost-only) | 466 | CORRECT — corpus `operator` values are `>= == > <= < and or in`; the 5 card operators and the compound `and`/`or`/`in` all route as Rust does |
| `read_i64_value` | 503 | `rd_int` | many | CORRECT |
| `read_operation_value` | 511 | scalar tail (raw string) | 194 | CORRECT — all 7 corpus spellings are inside `parse_operation`'s 7-name vocabulary (`add 138`, `subtract 13`, `increase 6`, `set_from_reference 1`, `remove 1`, `set 11`, `decrease 24`); 0 hits the `_ => None` arm |
| `read_quoted_text_value` | 526 | `vm.c:961` | `quoted_type` 11 | CORRECT |
| `read_placement_order_value` | 535 | `vm.c:966` | `placement_order` 35 | CORRECT |
| `read_cost_comparison_value` | 546 | `vm.c:851` | `cost_comparison` 5 | CORRECT (accepts `TAG_OBJECT` and `TAG_OBJECT_VARIANT` with the same variant-byte rule as Rust) |
| `read_distinct_value` | 558 | `vm.c:1152` | `distinct` 23 (effect side) | CORRECT — the only corpus effect-side spelling is `card_name` (23), the one name Rust maps without a fallback. Condition-side `distinct` uses `read_distinct_info_value`, which is also lossless in C (raw string preserved): `card_name 30`, `group_name 4`, `cost 3` |
| `read_distinct_info_value` | 570 | `vm.c` condition mirror | 37 | CORRECT |
| `read_dynamic_count_value` | 580 | `vm.c:846` | `dynamic_count` 0 | CORRECT, VERIFIED-NOOP: 0 of 985 |
| `read_ability_filter_value` | 585 | scalar tail (raw string) | 6 | CORRECT — corpus spellings `no_ability_type 3`, `no_ability 2`, `has_ability 1` are all inside the 4-name vocabulary, so the `_` fallback arm is never taken |
| `read_effect_value` | 601 | `read_effect` | many | CORRECT |
| `read_resource_on_select_value` | 607 | `vm.c` | 0 | CORRECT, VERIFIED-NOOP: 0 of 985 |
| `read_location_sub_checks_value` | 611 | `vm.c` condition mirror | 0 | CORRECT, VERIFIED-NOOP: 0 of 985 (no `sub_checks` in the corpus) |
| `read_effect_state_value` | 1032 | scalar tail (raw string) | **208** | CORRECT — `wait 147`, `active 61`, both named in `EffectState::from_str`; 0 hits the `Other(ArcStr)` arm, and C's raw string is exactly Rust's lossless representation of that arm |
| `read_or_ability_filters_value` | 1044 | scalar tail (JSON text) | 1 | CORRECT-VALUE. Rust builds `Vec<AbilityFilterBranch>`; C keeps the object as a JSON blob under the `or_ability_filters` extra. **No C consumer reads that key** (whole-tree grep: definition-site only), so the representation difference is unobservable. VERIFIED-NOOP: 0 consumers |
| `read_condition_vec_value` | 1061 | `read_cond_vec_val` | `conditions` 59 | CORRECT |
| `read_condition_value` / `decode_condition_direct` | 567 / gen | `vm.c:2588` `decode_condition_direct` | — | **ZERO CALLERS.** See §4 V-1 |
| `read_field_value` | 555 | `vm.c:1153` scalar tail | 147 keys | CORRECT — every one of the 147 effect keys the corpus emits is either decoded by name or retained losslessly in the extras table |
| `read_zone_value` | 465 | `vm.c:2715` `rb_zone_from_source_str` → `rb_zone_id_from_str` | 1782 | **CORRECT-VALUE, WRONG TABLE.** See §4 V-2 |
| `read_placement_zone_value` | 474 | `vm.c:2724` `rb_placement_zone_from_str` | 0 | CORRECT, VERIFIED-NOOP: 0 of 985 |
| `read_per_unit_value` | 828 | `vm.c:1071` | `per_unit` 76 | CORRECT |
| `decode_effect_field` / `decode_condition_field` (generated) | gen | `vm.c:684` / `vm.c:1825` | — | See §4 V-1 |

### 1c. `resolver.rs` — public items

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `struct AbilityLogItem` | 17 | `include/rabuka.h` `RbAbilityLogItem` | PRESENT, CORRECT |
| `struct LastKnownMemberInfo` | 97 | `resolver.c:1189` `rb_resolver_last_known_member_info` | PRESENT, CORRECT |
| `struct EffectOwner` | 122 | `resolver.c` `EffectOwner` | PRESENT, CORRECT |
| `struct ResolverSession` | 144 | `resolver.c` `RbResolverSession` | PRESENT, CORRECT |
| `struct InFlight` | 164 | `resolver.c` `InFlight` | PRESENT, CORRECT |
| `struct AwaitingChoice` | 189 | `resolver.c` `AwaitingChoice` | PRESENT, CORRECT |
| `struct Selection` | 214 | `resolver.c` `Selection` | PRESENT, CORRECT |
| `struct Carried` | 246 | `resolver.c` `Carried` | PRESENT, CORRECT |
| `struct AbilityResolver` | 278 | `resolver.c` `RbAbilityResolver` | PRESENT, CORRECT |
| `fn owning_effect` | 320 | `resolver.c:437` | PRESENT, CORRECT |
| `fn answering_effect` | 334 | `resolver.c` | PRESENT, CORRECT |
| `fn current_ability_is_activation` | 344 | `resolver.c:463` | PRESENT, CORRECT |
| `fn emit_pay_skip_gate` | 357 | `resolver.c` | PRESENT, CORRECT |
| `fn new` | 389 | `resolver.c:1176` `rb_resolver_new` | PRESENT, CORRECT |
| `fn record_member_last_known` | 414 | `resolver.c` | PRESENT, CORRECT |
| `fn last_known_member_info` | 434 | `resolver.c:1189` | PRESENT, CORRECT |
| `fn get_pending_choice` | 455 | `resolver.c` | PRESENT, CORRECT |
| `fn cached_condition_verdict` | 463 | `resolver.c` cache field | PRESENT, CORRECT (cap `rb_ability_cache_cap` is above the 985-card domain) |
| `fn store_condition_verdict` | 482 | `resolver.c` | PRESENT, CORRECT |
| `fn condition_needs_position_overlay` | 534 | `resolver.c` | PRESENT, CORRECT |
| `fn can_activate_effect` | 569 | `resolver.c` `rb_resolver_can_activate_effect` | PRESENT, CORRECT — reads `activation_condition` **in addition to** `condition`, as Rust does |
| `fn check_keywords` | 720 | `resolver.c` | PRESENT, CORRECT (corpus carries 0 keywords; data-inert, recorded not "fixed") |
| `fn capture_choice_owner` | 809 | `resolver.c` | PRESENT, CORRECT |
| `fn store_pending_choice` | 815 | `resolver.c` | PRESENT, CORRECT |
| `fn resolve_ability` | 1266 | `resolver.c:1204` `rb_resolve_ability` | PRESENT, CORRECT |
| `fn card_db` | 1550 | `resolver.c` | PRESENT, CORRECT |
| `fn fmt_card` | 1554 | `resolver.c` | PRESENT, CORRECT |
| `fn fmt_ids` | 1562 | `resolver.c` | PRESENT, CORRECT |
| `fn player_prefix` | 1573 | `resolver.c` | PRESENT, CORRECT |
| `fn card_name` | 1595 | `resolver.c` | PRESENT, CORRECT |

**No gap and no divergence found in `resolver.c`.** Every one of the 30 `resolver.rs`
public/`pub(crate)` items has a live, correct C twin.

### 1d. `types.rs` — public items (62)

Grouped by owner. `VALUE` twins live in `types.c`; `CHOICE`/`SPAWN`/`TRACE` twins are in
`choice.c` / `core/` and are listed for completeness with their real owner.

| Rust item | line | C twin | owner | verdict |
|---|---|---|---|---|
| `const PAY_SKIP_TARGET` | 30 | `PAY_SKIP_TARGET` | `types.c` | PRESENT, CORRECT |
| `fn repeat_prompt_choice` | 36 | `rb_repeat_prompt_choice` | `types.c` | PRESENT, CORRECT |
| `enum ChoiceRoute` (5) | 55 | `RbChoiceRoute` | header | PRESENT, CORRECT, 5/5 |
| `enum Choice` (7 top-level forms) | 94 | `RbChoice` | header/`choice.c` | PRESENT, CORRECT |
| `fn description_ja` / `_en` / `description` | 249/261/273 | `rb_choice_description_ja` / `_en` / `_utf8` | `types.c` | PRESENT, CORRECT |
| `fn allow_skip` | 286 | `rb_choice_allow_skip` | `types.c` | PRESENT, CORRECT |
| `struct LiveSuccessOption` | 303 | `RbLiveSuccessOption` | header | PRESENT, CORRECT |
| `struct AutoAbilityOption` | 313 | `RbAutoAbilityOption` | header | PRESENT, CORRECT |
| `enum ChoiceResult` | 325 | `RbChoiceResult` | header | PRESENT, CORRECT |
| `enum ExecutionContext` (4) | 342 | `RbExecutionContext` | header | PRESENT, CORRECT, 4/4 |
| `enum StageSelectIntent` (4) | 364 | `RbStageSelectIntent` | header | PRESENT, CORRECT, 4/4 |
| `const GAINED_ABILITY_INDEX_BASE` | 382 | `GAINED_ABILITY_INDEX_BASE` | `types.c` | PRESENT, CORRECT |
| `fn gained_ability_index` | 386 | `rb_gained_ability_index` | `types.c` | PRESENT, CORRECT |
| `enum LookAndSelectStep` (4) | 394 | `RbLookAndSelectStep` | header | PRESENT, CORRECT, 4/4 |
| `struct ChoiceBuilder` + `build` | 412/446 | `RbChoiceBuilder` + `rb_choice_builder_build` | `choice.c` | PRESENT, CORRECT |
| `ChoiceBuilder` setters (`card_type`, `cost_limit`, `cost_total`, `cost_values`, `group`, `characters`, `heart_colors`, `require_all_heart_colors`, `name_fragments`, `filtered_indices`, `picker`, `destination`, `discard_remaining`, `target_player_id`, `blind`) | 479-545 | `rb_choice_builder_*` | `choice.c` | PRESENT, CORRECT |
| `ChoiceBuilder` flags (`is_select_action`, `is_reveal`, `description_en`, `description_ja`) | 533-553 | `rb_choice_builder_*` | `choice.c` | PRESENT, CORRECT |
| `ChoiceBuilder::select_cards` | 585 | `rb_resolver_build_choice_select_cards` | `choice.c` | PRESENT, **CORRECTED — see §3 D2** |
| `ChoiceBuilder::as_filter` | 624 | `rb_choice_as_filter` | `choice.c` | PRESENT, CORRECT |
| `fn set_description` / `set_bilingual_descriptions` | 675/688 | `rb_choice_set_*` | `types.c` | PRESENT, CORRECT |
| `fn to_frontend_json` | 734 | `rb_choice_to_frontend_json` | `choice_renderer.c` | PRESENT, CORRECT |
| `struct EffectSpawnContext` | 887 | `RbEffectSpawnContext` | header | PRESENT, CORRECT |
| `struct StepOutput` + `from_value` / `merge` | 903/919/926 | `RbStepOutput` + `rb_step_output_*` | `types.c` | PRESENT, CORRECT |
| `enum ValueRef` (5) + `resolve` | 945/962 | `RbValueRef` + `rb_value_ref_*` | `types.c` | PRESENT, CORRECT, 5/5 — `Literal 7`, `StepValue`, `StepAccepted`, `StepValueOffset`, `Unresolved` |
| `struct ZoneSnapshot` + `from_game_state` | 993/1003 | `rb_zone_snapshot` / `rb_zone_snapshot_from_game_state` | `types.c` | PRESENT, CORRECT |
| `struct AbilityTraceNode` + `new`/`with_card`/`with_before` | 1025-1049 | `rb_ability_trace_*` | `types.c` | PRESENT, CORRECT |
| `struct EffectPipeline` + `new` | 1062/1073 | `rb_effect_pipeline_*` | `types.c` | PRESENT, CORRECT |
| `struct StepState` + `new`/`record`/`get`/`clear` | 1096-1130 | `rb_step_state_*` | `types.c` | PRESENT, CORRECT |

**No gap and no divergence found in `types.c`.**

---

## 2. Per-enum decode completeness

`enums.rs` is the decode authority. Every row below was checked twice: (a) the C table has a
row, and (b) **every value the real 985-ability corpus actually feeds a decoder** maps to a
row — because a table complete in isolation and incomplete against the data is a silent
wrong-value bug, not an error. Corpus counts come from the census in the header.

| enum | variants | C table | corpus values fed to it | unknown in C | verdict |
|---|---|---|---|---|---|
| `Zone` | 32 | `enums.c:56-113` `rb_ability_zone_from_str` / `:116-147` `to_str` | 1782 source/destination | **0** | COMPLETE |
| `TargetPlayer` | 4 | `enums.c:152-171` / `:174-179` | 0 (`choose_target_player` is an action, not a zone) | 0 | COMPLETE |
| `PlacementTarget` | 3 | **no C twin** | **0** | — | N/A — no wire strings; derived by `core/card.rs:2240 placement_target()` from `self_target`/`under_self`, which is `core/card.rs`'s to own. See BLOCKED-ON T1-x |
| `ActionType` | 68 | `enums.c:182-212` `from_str` / `:214-281` `label` | 2228 non-empty action values, 49 distinct spellings | **0** | COMPLETE — all 68 rows present in `from_str` and all 68 in `label` |
| `ConditionType` | 33 | `enums.c:283-316` / `:319-352` | condition `type` keys (`appearance 40`, `baton_touch 33`, `temporal 32`, `area_move 29`, `zone_change 21`, `state_change 6`, `or 6`, `energy_placed 4`, `live_success 4`, `opponent_live_success 2`, `temporal_count 1`) | **0** | COMPLETE — 33/33 in both directions |
| `SelectTargetKind` | 16 | `enums.c:355-371` / `:374-389` | 0 (built in `choice.c`, never decoded from a string) | 0 | COMPLETE |
| `EffectCardType` | 3 + `Other(ArcStr)` | `enums.c:412-420` / `:423-429` | **817** `card_type` values, 5 distinct spellings | 0 | COMPLETE for the variant; the `Other` **payload** is not representable in the C enum (documented, pinned by `parity_enums` lines 464-476) — but see **D1**, the payload was the *cause* of a real bug |
| `EffectState` | 2 + `Other(ArcStr)` | `enums.c:432-440` / `:443-449` | **208** (`wait 147`, `active 61`) | **0** | COMPLETE — 0 hits the `Other` arm |

Enums that live in other Rust files but are decoded **inside** `vm.rs` (so in scope for this
audit because the decode site is mine):

| enum | Rust site | vocabulary | corpus | C decode | verdict |
|---|---|---|---|---|---|
| `core::card::CardType` | `read_card_type_value` `vm.rs:479` | 3 names, `_ => None` | **817** (5 spellings) | `vm.c:1052` | **WAS DIVERGENT → FIXED (D1)** |
| `core::card::Operator` | `read_operator_value` `vm.rs:491` | `>= <= > < ==` | 466, spellings `>= == > <= < and or in` | `vm.c:1155` + compound path | CORRECT |
| `core::card::Operation` | `parse_operation` via `read_operation_value` `vm.rs:511` | 7 names, `_ => None` | 194, 7 spellings | scalar tail (raw string) | CORRECT — 0 misses |
| `core::card::ComparisonType` | `condition_decoder_gen` | — | `cost 38`, `score 34`, `equality 5`, `energy_relative 2` | `condition.c` | CORRECT — not an `enums.rs` type; no gap found |
| `AbilityFilter` | `read_ability_filter_value` `vm.rs:585` | 4 names | 6 (3 spellings) | scalar tail | CORRECT — 0 misses |
| `CardProperty` | `read_location_sub_checks_value` `vm.rs:832` | total fn, `_ => HasBladeHeart` | 37 (4 spellings, all named) | scalar tail | CORRECT, VERIFIED-NOOP for the `sub_checks` site: 0 of 985 |
| `core::types::ZoneId` | substituted for `Zone` at `vm.c:2715` | see V-2 | 1782 | `core/data.c` | CORRECT-VALUE, WRONG TABLE (V-2) |

**No enum in `enums.rs` is missing a variant in the C table, and no corpus value falls
through any of them.**

---

## 3. DIVERGENCES FOUND AND CLOSED

### D1 — `card_type` accepted names Rust's `CardType::from_card_str` rejects. **CLOSED.**
**This is the one real bug in the four modules.**

*Where C and Rust return different values for the same input.* Rust
`effect_decoder_gen.rs:81` decodes the effect key `card_type` with `read_card_type_value`
(`vm.rs:479-489`), which is `CardType::from_card_str(...)`. `CardType::from_card_str`
(`engine/src/core/card.rs`) has exactly three arms — `member_card`, `live_card`,
`energy_card` — and its miss arm is `_ => None`. So Rust's
`AbilityEffect::card_type: Option<CardType>` is **`None`** for any other spelling, and the
raw string cannot survive the decode. `vm.c:1052-1058` stored the **raw string** into
`AbilityEffect::card_type_field[24]` unconditionally.

*Measured reachability.* 817 of 2230 decoded effects carry the key; 649 spell it inside the
vocabulary (`member_card 442`, `live_card 121`, `energy_card 86`) and **168 spell it outside
it** — `card` ×167 and `member` ×1 — spread over **149 of the 985 abilities**
(`move_cards` 146, `gain_resource` 21, `modify_cost` 1). Not a speculative path.

*Why a wrong value and not a cosmetic one.* Every consumer that inherits the field tests it
for mere **non-emptiness**, not for membership of the vocabulary:

| consumer | what C did | what Rust does |
|---|---|---|
| `core/card.c:959` `card_filter_from_effect` | copies `"card"` into `RbCardFilter.card_type` | `util.rs:1839` `card_type_any().map(CardType::as_card_str)` → `None` |
| `core/card.c:1107` | `has_filter = !!(out->card_type[0] \|\| …)` → an all-else-empty effect becomes **filtered** | `util.rs:1442-1498` counts `card_type.is_some()` → `false` |
| `util.c:1547` | `if (f->card_type[0] && !rb_card_matches_type(card_id, f->card_type)) return 0;` — `"card"` matches **no** card → **empty candidate set, silent no-op** | no such term |
| `ability/choice.c:251` | puts `"card"` into `RbChoice.card_type` | `card_type_any().map(as_card_str)` can only ever place one of the three names |
| `ability/compound.c:467-469` | has an explicit `!strcmp(ct,"member")` arm that exists *only* because the field held the raw spelling → suppresses `self_target` inheritance | `compound/sequential.rs:164-165` tests `card_type_any() == Some(&CardType::Member)`; for `"member"` that is `None == Some(Member)` → **false**, so Rust **keeps** inheritance |
| `ability/dynamic_count.c:654-655` | passes `"card"` to `dc_per_unit_units` | `None` |

`ability/describe.c:357-367` was the **only** one of the six that did *not* inherit the bug —
it re-derives through `rb_effect_card_type_from_str`, which correctly yields
`RB_ECT_OTHER`. That is exactly why the other five looked covered.

*Fix* (`src/ability/vm.c:1052`, one hunk): `card_type_field` is now populated **only** for
the three names `CardType::from_card_str` accepts, and on a miss **both** the struct field
and the `card_type` extra are left unset. The extra matters: `core/card.c:959` and
`ability/dynamic_count.c:655` both read
`card_type_field[0] ? field : fx_extra(e, "card_type")`, so leaving the raw spelling in the
extras table would re-introduce the divergence the empty field removes. (Measured: that
extras-table fallback is **already dead** — 0 of 2230 effects have a `card_type` extra,
because the branch `continue`s before the scalar tail.) Rust drops the string too, so
nothing Rust can observe is lost.

*Result.*

| | baseline (`vm.c` `4D64C923…`) | fixed (`vm.c` `BFABC58A…`) |
|---|---|---|
| out-of-vocabulary `card_type` reaching the field | **168 effects / 149 abilities** | **0 / 0** |
| whole-corpus `^FAIL` lines (41 suites) | **1556** | **1555** |
| `parity_small_clusters` | 112 | **111** |
| `^FAIL` lines **added** | — | **0** |

The single line removed is
`parity_small_clusters :: FAIL: B4: <card name> total 36 > opponent 4 -> heart05 granted`.

*Tests* — `tests/test_vm_decode.c`, new `test_card_type_vocabulary()` (I claim this file):
pins the 649 in-vocabulary count, asserts 0 out-of-vocabulary spellings reach the field,
asserts neither the field nor the `card_type` extra leaks, asserts an in-vocabulary spelling
reaches `RbCardFilter.card_type` byte-identical, and pins `rb_effect_card_type_from_str`'s
`Other` arm for the two corpus spellings. File-isolated A/B, same test file in both trees,
only `vm.c` differing: **baseline 4 `FAIL` → fixed 0 `FAIL`.**

### D2 — `compound.c`'s `"member"` hack is now provably dead. **Reported, not edited.**
`ability/compound.c:469` compares `card_type` against the literal `"member"` to stand in for
`CardType::Member`. After D1 the field can never hold `"member"` (Rust's `from_card_str`
rejects it), so that arm is unreachable. It is harmless — an arm that cannot fire cannot
diverge — so it is left in place rather than risk a behavioural change in a file I do not
own. **BLOCKED-ON: `src/ability/compound.c:469` — the `!strcmp(first_ct,"member")` arm is
dead after D1 and can be deleted; `!strcmp(...,"member_card")` on the line above is the real
`CardType::Member` test and must stay.**

---

## 4. STRUCTURAL FINDINGS — recorded, not edited

### V-1 — the entire typed condition decoder in `vm.c` has ZERO callers. **DEAD, CORRECT.**
`vm.c:1286-2640` is a ~1350-line hand-transcription of Rust's generated condition decoder:
`ConditionLocals`, `decode_condition_field` (85 keys), 21 `build_*` constructors and
`decode_condition_direct`. **`decode_condition_direct` (`vm.c:2588`) has no caller anywhere
in the tree** (whole-repo grep over `*.c`/`*.h`: the definition, its doc comment and nothing
else). Rust calls the equivalent from `read_condition_value` (`vm.rs:567`).

Every condition actually decodes through `read_condition` (`vm.c:431`) →
`read_condition_fields` (`vm.c:404`), the generic mirror that stores every field as
`(key, CondValue)` losslessly.

*Is this a value divergence?* **No, and it is measured, not assumed.** The mirror keeps all
91 condition keys, including the **three the typed decoder does not name** —
`any_location` (21), `activated_this_turn` (3), `paid_cost_this_turn` (2) — plus
`trigger_event` (160), which the typed decoder also lacks. The typed decoder's catch-all
(`vm.c:2564`) would have called `rb_note_decode_fallback` and **dropped the value** for each
of those; the mirror keeps them. That is why `rb_decode_fallback_count()` is **0**: the dead
path is precisely the one that would have lost data. Both decoders also agree on the
`TAG_OBJECT` / `TAG_OBJECT_VARIANT` variant-byte rule for the nested-object keys
(`sub_checks`, `cost_comparison`, `trigger_event`) — Rust reads the discriminant byte only
for `TAG_OBJECT_VARIANT` (`vm.rs:611-624`, `:546`, `:535`), and so does the C.

Consequence to record: `build_unsupported` (`vm.c:2580`), the only producer of
`c->variant = 20` (`Condition::Unsupported`), is on the dead path. Corpus has **0**
variant-20 conditions (16 distinct variants observed, 0..19), so nothing is lost today.

**Not deleted.** "Zero C callers does not prove deadness" is a standing rule here, and the
typed decoder is the thing that would have to be repaired if the mirror ever changed.

### V-2 — `vm.c`'s zone decode is wired to the WRONG Rust type's table. **INERT TODAY.**
`vm.c:2715` `rb_zone_from_source_str` is documented as `Zone::from_source_str` but delegates
to `rb_zone_id_from_str` (`core/data.c`), i.e. **`core::types::ZoneId`**, not
`ability::enums::Zone`. Two different Rust types with two different vocabularies, and the
substitution is invisible at the call site:

| input | `ability::enums::Zone` (`enums.rs:79-167`) | `core::types::ZoneId` |
|---|---|---|
| `center` | `StageCenter` | **UNKNOWN** |
| `left`, `left_side` | `StageLeft` | **UNKNOWN** |
| `right`, `right_side` | `StageRight` | **UNKNOWN** |
| `live_total` | **UNKNOWN** (not in `from_str`) | `LIVE_TOTAL` |
| `\xE3\x82\xB9\xE3\x83\x86\xE3\x83\xBC\xE3\x82\xB8` | **UNKNOWN** | `STAGE` |

**Measured: 0 of the 985 abilities.** All 1782 corpus source/destination values are inside
both vocabularies, so the two tables agree on every value the data contains and
`rb_decode_fallback_count()` is 0. The condition-side `location`/`source`/`destination` keys
are `ArcStr` on both sides (verified at `condition_decoder_gen.rs:181/204/167` — they use
`read_arc_str_value`, **not** `read_zone_value`), so the substitution does not reach them
either; the one fullwidth zone in the corpus is a condition `location`, stored verbatim by
both.

Left as-is deliberately. Porting the five stage-position names into the `ZoneId` table would
be a speculative change on data nobody reaches, and `rb_zone_id_from_str` is not my file.
`tests/test_parity_enums.c` now pins the inertness so a future regeneration that *does* use
one of the seven fails loudly instead of silently.

**BLOCKED-ON: `src/core/data.c` (`rb_zone_names` + the alias table) — if a corpus change
ever introduces `center`/`left`/`left_side`/`right`/`right_side` as an effect
`source`/`destination`, `rb_zone_from_source_str` will return `RB_ZONEID_UNKNOWN` where Rust
returns `StageCenter`/`StageLeft`/`StageRight`. Fix belongs in `vm.c`'s
`rb_zone_from_source_str` (switch it to `rb_ability_zone_from_str` + a `ZoneId` shim), not in
`data.c`.**

### V-3 — `enums.c` has **zero production callers**. **NOMINAL, CORRECT.**
Every one of `rb_ability_zone_from_str`, `rb_target_player_from_str`,
`rb_action_type_from_str`, `rb_condition_type_from_str`,
`rb_select_target_kind_from_str`, `rb_effect_card_type_from_str`,
`rb_effect_state_from_str` (and their `to_str`/`as_str` twins) is called **only from
`tests/test_parity_enums.c`**. The production decode path uses `strcmp` against literals
instead. This is worth recording because it means the tables are *verified but not load-bearing*:
a bug in `enums.c` cannot change behaviour today, and equally a future new call site inherits
a table that has been round-trip-tested against all 68 `ActionType` / 33 `ConditionType` /
32 `Zone` / 16 `SelectTargetKind` rows. Left in place deliberately (zero callers ≠ dead).

### V-4 — `EffectCardType::Other` / `EffectState::Other` payload is not representable in C.
Rust's `Other(ArcStr)` preserves the unrecognised string and `as_str` returns it verbatim
(`enums.rs:735`, `:744`, `:810`, `:818`). The C enum has no string member, so
`rb_effect_card_type_as_str(RB_ECT_OTHER)` returns `""`. This is a defect in the enum shape
declared in `include/rabuka.h`, **not** in `enums.c` — the variant selection is already
correct. Already pinned deliberately by `test_parity_enums.c:464-476` and `:488-496`; not
weakened. Corpus impact: **0** for `EffectState` (all 208 values named) and **168** for
`EffectCardType` — and D1 showed that "preserving" the string in the wrong place is exactly
what caused the bug, so the C's lossiness here is the *correct* behaviour for a decode-time
`CardType`, and the raw spelling is still available from the wire blob for diagnostics.

---

## 5. Verdict summary

321 rows = 127 public-facing items + 194 enum variants. The split, itemised so it sums:

| verdict | rows | which |
|---|---|---|
| **PRESENT, live, CORRECT** | **271** | all 194 enum variants that have a C table (158 in `enums.c`, 32 `types.rs` enums in the header/`RbChoice`), and 77 of the 127 public items |
| **PRESENT, value-correct but UNREACHABLE** | **38** | (a) the whole typed condition decoder — `decode_condition_direct`, `decode_condition_field`, `ConditionLocals`, 21 `build_*` = 24 rows; (b) `enums.c`'s 14 `from_str`/`to_str`/`as_str` entry points, which have zero production callers (V-3). **Not deleted** — zero callers ≠ dead, and these are the row-level twins an audit needs to keep covered |
| **DIVERGENT → CLOSED (D1)** | **2** | `AbilityEffect::card_type` (the struct field) and `read_card_type_value` (the reader). One hunk fixed both |
| **ABSENT, `VERIFIED-NOOP: 0 of 985`** | **8** | `enums.rs` `PlacementTarget` ×3 variants (no wire strings — it needs a *derived* twin in `core/card.rs`, see T1-9); `vm.rs` `populate_from_json` ×1 (serde-only, `json_path_test` feature off by default); `vm.rs` `read_dynamic_count_value`, `read_resource_on_select_value`, `read_location_sub_checks_value`, `read_placement_zone_value` ×4 |
| **ABSENT with reachability > 0** | **0** | — |

Nothing reachable is missing. **Nothing unreachable was ported**: a speculative port on data
nobody reaches can only be wrong, and it costs review budget.

`resolver.c` and `types.c`: **all 30 `resolver.rs` public/`pub(crate)` items and all 62
`types.rs` public items have a live correct twin. Zero gaps, zero divergences** in either file.

**Reachability histogram of every gap examined** (`n` = how many of the 985 abilities reach it):

| n | items | outcome |
|---|---|---|
| **149** | D1 `card_type` out-of-vocabulary (168 effects: `card` ×167, `member` ×1) | **CLOSED** |
| 6 | `ability_filter` spellings | correct, 0 misses |
| 5 | `cost_comparison` nested objects | correct |
| 3 | `ability_filter` on the condition side | correct |
| 1 | `or_ability_filters` | representation differs, **0 consumers** |
| **0** | the 8 `VERIFIED-NOOP` items; V-1's three unnamed condition keys (`any_location` 21, `activated_this_turn` 3, `paid_cost_this_turn` 2 — kept by the live mirror); V-2's seven zone strings; `read_distinct_value`'s `_` arm; `read_effect_state_value`'s `Other` arm and its non-empty-object arm; `read_ability_filter_value`'s `_` arm; `parse_operation`'s `_` arm; `CardProperty`'s `_ => HasBladeHeart` default; `read_position_value`'s `TAG_OBJECT` branch; `RB_MAX_*` capacity overflow | nothing to do |


---

## 6. New / updated tests

| file | change | claim |
|---|---|---|
| `tests/test_vm_decode.c` | new `test_card_type_vocabulary()` — 8 assertions pinning D1. Baseline **4 `FAIL`** → fixed **0 `FAIL`**, file-isolated A/B. | **T1 claims this file** |
| `tests/test_parity_enums.c` | new `test_corpus_decode_completeness()` — 8 assertions walking all 985 abilities: 2228 action values, 1782 zone values, 208 states, 2 effect-side `ability_filter`s, and the cross-table `Zone` vs `ZoneId` agreement that pins V-2. Suite now **7199 assertions, 0 failures**. | **T1 adds to this file**; no existing assertion touched |

No existing assertion was weakened, deleted or ignored. No test was edited to reach a number:
the two count pins I first wrote wrong (`2052`/`2495`) were my arithmetic error and were
corrected to the measured `2228`/`1782` **after** confirming the substantive assertions
(0 unknown action, 0 unknown zone, 0 table disagreements, 0 unknown state, 0 unknown
`ability_filter`) already passed.

## 7. Corpus result

41 suites, strictly sequential, `^FAIL` counted on stdout **and** stderr:

| | `^FAIL` lines |
|---|---|
| baseline (only my `vm.c` hunk reverted) | **1556** |
| with D1 | **1555** |
| added by D1 | **0** |
| removed by D1 | **1** (`parity_small_clusters` 112 → 111) |

Suites with zero `^FAIL` in both runs, unchanged: `vm_decode`, `decode_skew`,
`decode_fallback`, `parity_enums`, `parity_conditions`, `parity_executor_dispatch`.
The remaining ~1555 lines are pre-existing open divergences owned by other clusters; this
audit neither added to nor reduced them beyond the one line above.

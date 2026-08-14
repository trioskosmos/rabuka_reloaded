# Ability pipeline: current architecture, failure points, and plan

Last reviewed: 2026-07-29

This document is an engineering map of the ability system. It is intentionally
about data flow and ownership rather than a complete card-ability reference.
The schema and card-facing semantics live in
[`cards/ABILITY_DOCUMENTATION.md`](cards/ABILITY_DOCUMENTATION.md).

## Executive summary

The system is functional, but the contract between the parser, serialized
ability data, Rust models, and effect handlers is distributed across several
manual transformations. The queue/resolver work is further along than the
data model: choices can pause and resume an ability, but the representation of
an effect still has two overlapping forms and a large hand-written conversion
bridge.

The highest-value work is not an immediate rewrite of every effect handler. It
is to make the existing pipeline observable and mechanically validated first:

1. Make generation reproducible and record exactly which inputs produced each
   generated artifact.
2. Add strict validation at the parser, bytecode, and runtime boundaries.
3. Reduce the number of places that know how an action is represented.
4. Refactor the effect model and decoder only after the compatibility tests are
   strong enough to make that change safe.

## What is true in the current code

| Area | Current implementation | Important implication |
| --- | --- | --- |
| Authoring input | `cards/cards.json` contains card text and metadata | The Japanese card text is the upstream source for extracted abilities. |
| Parsing | `cards/ability_extraction/extract_card_abilities.py` calls the large parser in `cards/ability_extraction/parser.py` | `abilities.json` is generated data, not the original source. |
| Ability data | `cards/abilities.json` contains 800 grouped `unique_abilities` entries in this checkout | One entry can be referenced by many cards. |
| Build format | `cards/compile_abilities.py` writes a compact tagged binary-JSON representation and generates `abilities_gen.rs` | This is serialization of the JSON tree, not a typed instruction VM. |
| Embedded artifact | `engine/src/ability/abilities_gen.rs` contains the bytecode, offsets, interned strings, and card-to-ability pairs | The Rust binary does not parse `abilities.json` at runtime. |
| Card loading | `engine/src/core/card_loader.rs` attaches `AbilityRef(u16)` values to cards | Cards keep lightweight indices rather than decoded `Ability` objects. |
| Decode | `engine/src/ability/ability_store.rs` calls `vm::get_ability()` on demand | There is currently no decoded-ability cache; each `resolve()` decodes a fresh `Arc<Ability>`. |
| Typed model | `engine/src/core/card.rs` deserializes `AbilityEffect`, then `populate_from_json()` builds `EffectKind` recursively | The flat fields and typed `kind` must remain consistent. |
| Triggering | `engine/src/core/game_state/abilities.rs` collects and enqueues triggered abilities | Trigger snapshots are stored on queue entries so later resolution can see the event that caused the trigger. |
| Resolution | `AbilityResolver` owns cost/effect execution and choice state | The resolver is persisted in the queue entry across choice round-trips. |
| Dispatch | `engine/src/ability/effects/mod.rs::execute_effect()` routes `ActionType` to domain handlers | The central match is still large, but the actual work is split by domain. |

## The real pipeline

```text
cards/cards.json
      |
      v
extract_card_abilities.py
      |
      v
parser.py: parse text, infer triggers/cost/effect/conditions, normalize tree
      |
      v
cards/abilities.json
      |  unique_abilities + card references
      v
compile_abilities.py
      |
      +--> cards/build/abilities_gen.rs       (build artifact, when retained)
      +--> engine/src/ability/abilities_gen.rs (checked-in embedded artifact)
      |       BYTECODE + OFFSETS + STRINGS + CARD_ABILITY_PAIRS
      v
CardLoader::attach_abilities()
      |
      v
Card.abilities: Vec<AbilityRef>        (u16 unique-ability index)
      |
      v
AbilityRef::resolve() -> vm::get_ability()
      |
      +--> read tagged binary-JSON tree
      +--> direct top-level decoder for Ability
      +--> serde_json::from_value for nested/cost/effect structures
      +--> AbilityEffect::populate_from_json()
      +--> draw-count normalization
      v
Ability { cost, effect, triggers, ... }
      |
      v
trigger_auto_ability*() / player activation
      |
      v
AbilityQueue: enqueue -> order -> process
      |
      +--> AbilityResolver::resolve_ability()
      |       +--> validate and pay cost
      |       +--> pause for Choice if needed
      |       +--> execute effect
      |       +--> enqueue newly triggered abilities
      |
      +--> resume_with_choice() through turn/actions.rs
      v
game state, rule log, structured resolution log, frontend actions
```

### Build-time stages

`extract_card_abilities.py` performs more than extraction. It groups identical
ability text, parses costs and effects, applies parser post-processing, writes
`abilities.json`, and then invokes `compile_abilities.py` to regenerate the
embedded Rust artifact. This means a parser change can affect both the JSON
contract and the generated Rust source in one run.

`compile_abilities.py` currently encodes each ability as a tagged tree:

- primitive values have tags for null, booleans, integers, floats, and strings;
- object keys and string values are interned in `STRINGS`;
- `OFFSETS` locates each ability slice in `BYTECODE`;
- `CARD_ABILITY_PAIRS` maps card numbers to unique-ability indices;
- loader-only fields such as the `cards` mapping are excluded from each encoded
  ability body.

The artifact is compact and data-driven, but it is not a schema-aware opcode
stream. The action names and field names remain JSON names.

### Runtime decode stages

`vm.rs` has two decode paths with shared post-processing:

1. The direct path reads the tagged object and decodes the top-level `Ability`.
2. Nested effects and costs are collected into `serde_json::Map` values and
   deserialized through the normal Rust `serde` model.
3. `populate_from_json()` reconstructs `EffectKind` and recursively populates
   nested effects and conditions.
4. A draw-count normalization pass mirrors the text-loader behavior.
5. If the direct path fails, `get_ability()` logs an error and falls back to
   reconstructing a JSON value and calling `decode_like_json()`.

Therefore the current bytecode path does reduce the shipped asset and avoids
loading the complete `abilities.json` file, but it does **not** provide a
serde-free, allocation-free, typed decoder. The fallback is also only logged;
`AbilityRef::resolve()` converts a failed decode into `Ability::default()`, which
can turn corrupt data into a silent no-op.

## Runtime resolution model

### Trigger collection and ordering

`engine/src/core/game_state/abilities.rs` is the orchestration layer.

- `ability_matches_trigger()` maps engine trigger types to the trigger text on
  an ability.
- `trigger_auto_ability()` resolves a card number and ability text/index, then
  enqueues an `AbilityQueueEntry`.
- `trigger_auto_ability_by_index()` is the lower-allocation path for callers
  that already know the numeric ability index.
- Entries snapshot movement data and the triggering member where required by
  conditions such as `those_cards`.
- Gained abilities are resolved from `gained_card_abilities` and can also be
  enqueued.
- `process_pending_auto_abilities()` resolves the active player's entries first,
  then the non-active player's entries.
- Multiple simultaneous auto abilities create a `SelectAutoAbility` choice.
- Newly triggered entries are drained depth-first before stale queued entries.
  Recursion and drain iteration caps are safety nets against runaway triggers.

The ordering logic is game-rule logic, not merely a FIFO queue. Any future queue
refactor must preserve active/non-active ordering, simultaneous-ability choice,
depth-first re-entry, and event snapshots.

### Queue state machine

`engine/src/ability_queue.rs` owns the resumable execution boundary:

```text
Idle
  |
  +--> PayingCost --------------------------+
  |                                         |
  +--> WaitingForChoice <-------------------+
  |          ^                              |
  |          | resume_with_choice           |
  +--> ExecutingEffect ---------------------+
             |
             v
        Completed -> Idle

Idle + multiple pending auto abilities
  -> WaitingForAutoAbilityChoice -> Idle
```

An entry contains more than the ability itself: cost/effect progress flags,
pending sequential actions, choice routing, movement snapshots, condition
cache data, and the persistent `AbilityResolver`. This is why recreating a
resolver after every choice is unsafe; its state includes selected cards,
revealed cards, repeat state, formation plans, and other cross-step context.

`engine/src/turn/actions.rs::resume_with_choice()` is the external resumption
boundary. It validates the pending choice, applies the selected result, returns
the queue to execution, and continues until the resolver either asks for
another choice or completes.

### Cost/effect execution

`AbilityResolver::resolve_ability()` coordinates the phases:

1. Validate the ability's activation condition and cost.
2. Pay the cost in `cost.rs`. Sequential, optional, energy, movement, and
   choice-based costs can all suspend execution.
3. Execute the effect through `effects/mod.rs::execute_effect()`.
4. Route compound effects through `compound.rs`, look/select behavior through
   `look.rs`, movement through `move_cards.rs`, and choice continuation through
   `choice.rs`.
5. Apply replacement effects, target routing, and non-stackable checks as part
   of effect execution.
6. Complete the queue entry or store the resolver and pending choice.

The action-to-domain map is currently:

| Concern | Main implementation |
| --- | --- |
| Central action dispatch | `engine/src/ability/effects/mod.rs` |
| Costs and payment | `engine/src/ability/cost.rs` |
| Choice application/resumption | `engine/src/ability/choice.rs` |
| Sequential/conditional/repeat effects | `engine/src/ability/compound.rs` |
| Look/reveal/select flows | `engine/src/ability/look.rs` |
| Card movement | `engine/src/ability/move_cards.rs` |
| Score and heart changes | `engine/src/ability/effects/score.rs` |
| State/resource changes | `engine/src/ability/effects/state.rs`, `draw.rs` |
| Ability gain/invalidation/activation | `engine/src/ability/effects/ability_effects.rs` |
| Miscellaneous and special actions | `engine/src/ability/effects/misc.rs` |
| Conditions | `engine/src/ability/condition.rs` and `condition/` |

## Why a one-ability fix is still expensive

The original document correctly identified the symptom, but some of its
explanations were out of date. The current causes are:

### 1. The schema is implicit

There is no single schema file that defines the relationship between parser
output, Rust fields, action names, condition names, and handler behavior. That
contract is spread across:

- parser registries and normalization code;
- `ActionType::from_str()` in `enums.rs`;
- `AbilityEffect::kind_from_action()` in `card.rs`;
- custom `AbilityCost` deserialization;
- `EffectKind` variant fields;
- `populate_from_json()` recursion;
- handler assumptions in the effect modules.

Adding or changing a field can therefore require coordinated edits in several
places even when the ability JSON looks simple.

### 2. `AbilityEffect` has two representations

`AbilityEffect` stores common flat fields such as `action`, `source`,
`destination`, `count`, and `target`, plus a typed `kind: Option<EkBox>`.
Handlers use both styles. `populate_from_json()` must keep the typed variant
aligned with the flat representation, including nested effects inside
compound actions and conditions.

This is the highest-risk data-model seam: a field may deserialize successfully
but still be absent from the typed variant that a handler reads.

### 3. The decoder is generic, but the post-processing is manual

The binary codec itself does not need a new opcode for every action, which is a
real maintenance improvement. However, correctness still depends on the
manual `populate_from_json()` bridge and cost-key normalization. A new nested
field can survive the binary round trip yet be dropped by typed deserialization
or never reach `EffectKind`.

### 4. Dispatch knowledge is distributed

The central match gives one entry point, but handler-specific semantics are
spread across cost, compound, choice, movement, condition, and effect modules.
There is no generated registry containing action name, accepted fields,
handler, choice behavior, and test coverage.

### 5. Runtime errors are too easy to hide

Unknown action strings, malformed bytecode, missing fields, and failed effect
preconditions often become an empty/default effect, a skipped effect, or a log
message. That is useful for keeping a game running, but dangerous while
authoring or migrating abilities.

### 6. The queue has real state, but the transitions are not centralized

The queue is substantially better than a collection of ad-hoc pending fields:
it owns the resolver and explicit states. Still, many callers mutate entry
flags, pending actions, resolver storage, choice routing, and completion state
directly. The state machine's invariants are consequently implicit.

Also, `AbilityId` is deterministic (`card + ability index + trigger`) and its
duplicate-check helper is not the mechanism that drives queue ordering. It
should not be treated as a globally unique execution-instance ID when the same
ability can trigger more than once.

## Recommended work

### Regeneration

After any parser, card, or engine field change, run:

```bash
python cards/ability_extraction/extract_card_abilities.py
```

This is the single entry point. It extracts abilities from `cards/cards.json`,
compiles the bytecode + Rust artifact (`abilities_gen.rs`), regenerates both
Rust decoders (`effect_decoder_gen.rs`, `condition_decoder_gen.rs` from
`core/card.rs`), and runs bytecode validation. All generated outputs are updated
in one run — there is no separate compile/decoder step to remember.

### Phase 0: make the current system explainable and fail loudly

Do this before a broad refactor.

- [x] Add a generation manifest containing parser version/hash, input hashes,
  unique-ability count, generated artifact hash, and engine commit.
- [x] Use repository-relative source paths in generated metadata; the current
  `source_file` value can contain a machine-specific absolute path.
- [x] Add one documented regeneration command and make it update all generated
  outputs atomically. (`python cards/ability_extraction/extract_card_abilities.py`)
- [ ] Decide whether `cards/build/abilities.bin` is a retained artifact or a
  temporary intermediate. The runtime currently embeds `BYTECODE` from the
  generated Rust file, so the build should not imply that the loose `.bin`
  file is the runtime source of truth.
- [x] Add parser validation that rejects unknown action/condition names, impossible
  field combinations, and effects that parse to an empty action list unless
  explicitly marked as null/custom.
- [x] Change runtime decode APIs to return `Result<Ability, DecodeError>` with the
  ability index and byte range. Do not convert malformed data to a default
  ability without an explicit compatibility mode.
- [x] Keep `bytecode_deep_compare_test` as a required regeneration gate and add a
  test that every card reference resolves to a valid ability index.

### Dead code removed

- [x] Deleted `engine/src/ability/vm_gen.rs` (deprecated, no longer compiled)
- [x] Deleted `cards/gen_vm_decoder.py` (deprecated, never used at runtime)
- [x] Deleted `cards/ability_extraction/parser copy.py` (outdated backup)
- [x] Removed `scan_abilities()`, `_parse_list_fields()`, and `infer_type()` from
  `compile_abilities.py` (diagnostic-only, no longer drives the decoder)
- [x] Merged duplicate `_propagate()` and `_propagate_if_missing()` into one
  function with `skip_existing` parameter

### Phase 1: create a machine-readable contract

- [x] Introduce a schema or registry (`cards/ability_schema.json`) that describes, for each action and condition:
  - canonical JSON name and aliases;
  - required and optional fields;
  - nested effect/condition fields;
  - target and choice behavior;
  - Rust action/variant mapping;
  - handler module;
  - whether the action is authoring-only, runtime-only, or both.

The first version does not need to generate all Rust. It should generate a
validation report and the action-to-handler reference. This gives the project
a single place to answer "what does this field mean?" without forcing a risky
rewrite.

The registry should also distinguish parser normalization from game semantics.
For example, a parser may normalize `look_and_select` into effect steps, while
the runtime still needs to preserve the original rule-level meaning for logs
and debugging.

### Phase 2: remove the dual effect representation incrementally

- [x] Schema cross-reference validator (`cards/validate_schema.py`) catches drift
  between schema, compiler opcodes, Rust enum variants, and handler coverage.
- [ ] Define typed fields and conversion tests for one domain, such as compound
  effects.
- [ ] Make handlers read the canonical typed form.
- [ ] Keep a compatibility conversion at the boundary while old data is still
  present.
- [ ] Remove the corresponding `populate_from_json()` branch only after all
  generated and runtime tests pass.
- [ ] Repeat for costs, movement, choices, and state effects.

Do not start by flattening every field into one large struct. That would remove
some enum boilerplate but preserve the ambiguity about which fields are valid
for which action.

### Phase 3: improve decoding only after the contract is stable

The current generic binary-JSON format is a reasonable compatibility layer. A
typed decoder can be considered after Phase 1 and Phase 2, provided it has:

- a schema-derived or otherwise single-source field mapping;
- exact deep comparison against the compatibility decoder;
- malformed-input tests and versioning;
- parity tests for every supported platform feature;
- a clear answer for unknown fields and forward compatibility.

`cards/gen_vm_decoder.py` currently describes a historical generated decoder
and is marked deprecated; it is not the current runtime path. It should either
be removed or rewritten as part of a real, tested code-generation design so it
does not mislead future maintainers.

### Phase 4: make queue transitions explicit

Once data loading is stable, reduce queue coupling by exposing transition
operations instead of allowing callers to mutate entry internals freely. The
queue API should make these states and invariants explicit:

- only one active entry can be resolving;
- a waiting choice identifies the entry and the player who must answer;
- a resolver exists whenever a resumable execution is paused;
- cost completion cannot be confused with effect completion;
- pending sequential actions preserve order across a choice;
- completed entries cannot be selected again;
- trigger snapshots belong to the entry that observed the event.

Add model-based tests for sequences such as:

```text
trigger two auto abilities
  -> choose order
  -> first ability asks for a target
  -> resume
  -> first ability triggers another auto ability
  -> drain the new ability depth-first
  -> resolve the second original ability
```

This is more valuable than adding another large integration test for a single
card because it tests the execution protocol shared by many cards.

## Test and debugging strategy

The existing tests provide a good base and should become explicit gates:

| Test or helper | Purpose |
| --- | --- |
| `engine/tests/test_modules/bytecode_deep_compare_test.rs` | Bytecode decode must equal the JSON compatibility path for every ability. |
| `engine/tests/test_modules/bytecode_validation_test.rs` | Every generated ability decodes and basic action/cost expectations hold. |
| `engine/src/core/card_loader.rs` tests | Card-to-ability references are present and in range. |
| `engine/tests/test_modules/ability_engine_fixes_test.rs` | Choice, cost, targeting, and resolver regressions. |
| `engine/tests/test_modules/auto_system_stress_test.rs` | Queue and generated-action behavior under repeated auto triggers. |
| `engine/tests/helpers/mod.rs` | Test-level choice/resume utilities and ability trace inspection. |

Add the following missing gates:

- parser output validation for every unique ability, not just decode success;
- round-trip tests from card text to `abilities.json` to generated artifact;
- unknown-action and unknown-condition failure tests;
- malformed/truncated bytecode tests;
- choice-player routing tests for self, opponent, and both targets;
- queue invariant/property tests around completion, re-entry, and duplicate
  triggers;
- a per-action coverage report that identifies actions with no handler test;
- snapshot tests for structured ability-resolution logs.

When debugging one ability, capture these identifiers first:

```text
card number
unique_ability index
ability index on the card
trigger type
queue entry state
action and EffectKind variant
pending choice, if any
```

Then follow the narrow path:

```text
cards/abilities.json
  -> CARD_ABILITY_PAIRS / AbilityRef index
  -> vm::get_ability(index)
  -> AbilityEffect::populate_from_json()
  -> trigger_auto_ability*()
  -> AbilityQueueEntry
  -> AbilityResolver::resolve_ability()
  -> execute_effect()
  -> domain handler
```

This is the practical replacement for reading the whole effect subsystem.

## Decisions to keep explicit

The following choices should be recorded in this document or in an ADR when
made:

- Is `abilities.json` a generated artifact that must never be hand-edited, or
  is it a supported patch layer after extraction?
- Is the generated Rust blob checked in for all platforms, or generated during
  every build?
- Is the compatibility `serde_json` decoder retained for shipped builds, or
  only for tests and development?
- Should decoded abilities be cached in a bounded pool, or decoded afresh as
  they are now? A cache saves CPU but requires an eviction policy and platform
  memory budget.
- What is the supported behavior for unknown fields/actions: reject the build,
  preserve them as custom data, or log and skip them?
- What is the stable identity of a queued execution instance when one ability
  triggers repeatedly?

## Bottom line

The current system has two meaningful strengths: card loading is already lazy
with respect to decoded abilities, and the queue now preserves resolver state
across interactive choices. The remaining difficulty is contract drift, not a
single slow handler.

The sensible sequence is therefore:

```text
reproducible generation
  -> strict validation and diagnostics
  -> machine-readable action/condition contract
  -> canonical typed effect model
  -> typed decoder, if still justified
  -> explicit queue transitions and property tests
```

That order reduces risk while improving the day-to-day task of fixing one
ability. It also prevents a decoder rewrite from masking parser or rule-model
bugs, which are the more fundamental source of ambiguity today.

## Appendix A: Action type → handler lookup

When debugging a single ability, start by identifying the `action` string in
`abilities.json`, then jump to the handler below. All paths are relative to
`engine/src/ability/`.

| ActionType | Handler function | File | Line |
| --- | --- | --- | --- |
| `Sequential` | `execute_sequential_effect` | `compound.rs` | 43 |
| `ConditionalAlternative` | `execute_conditional_alternative` | `compound.rs` | 638 |
| `ConditionalOnResult` | `execute_conditional_on_result` | `compound.rs` | 776 |
| `ConditionalOnOptional` | `execute_conditional_on_optional` | `compound.rs` | 854 |
| `RepeatProcedure` | `execute_repeat_procedure` | `compound.rs` | 759 |
| `LookAndSelect` | `execute_look_and_select` | `look.rs` | — |
| `SelectCards` | `execute_select_cards` | `look.rs` | — |
| `LookAt` | `execute_look_at` | `look.rs` | — |
| `MoveCards` | `execute_move_cards` | `move_cards.rs` | — |
| `DiscardCard` | `execute_move_cards` | `move_cards.rs` | — |
| `DrawCard` | `execute_draw_wrapper` | `effects/draw.rs` | 208 |
| `DrawUntilCount` | `execute_draw_until_count` | `effects/draw.rs` | — |
| `Select` | `execute_select_effect` | `effects/draw.rs` | 264 |
| `SelectNumber` | `execute_select_number` | `effects/draw.rs` | 568 |
| `GainAbility` | `execute_gain_ability_effect` | `effects/ability_effects.rs` | 13 |
| `GainAbilityFromSource` | `execute_gain_ability_from_source` | `effects/ability_effects.rs` | 319 |
| `InvalidateAbility` | `execute_invalidate_ability` | `effects/ability_effects.rs` | 143 |
| `SuppressAbilityTrigger` | `execute_suppress_ability_trigger` | `effects/ability_effects.rs` | 196 |
| `ActivateAbility` | `execute_activate_ability` | `effects/ability_effects.rs` | 61 |
| `ChangeState` | `execute_change_state` | `effects/state.rs` | 18 |
| `SetCost` | `execute_set_cost` | `effects/state.rs` | 769 |
| `SetBladeType` | `execute_set_blade_type` | `effects/state.rs` | 821 |
| `SetHeartType` | `execute_set_heart_type` | `effects/state.rs` | 903 |
| `SetCardIdentity` | `execute_set_card_identity` | `effects/state.rs` | 997 |
| `SetBladeCount` | `execute_set_blade_count` | `effects/state.rs` | 1025 |
| `ActivationCost` | `execute_activation_cost` | `effects/state.rs` | 963 |
| `ModifyCost` | `execute_modify_cost` | `effects/state.rs` | 1192 |
| `SetCostToUse` | `execute_set_cost_to_use` | `effects/state.rs` | 1151 |
| `AllBladeTiming` | `execute_all_blade_timing` | `effects/state.rs` | 1170 |
| `SpecifyHeartColor` | `execute_specify_heart_color` | `effects/state.rs` | 1092 |
| `ReduceLiveCardSetLimit` | `execute_reduce_live_card_set_limit` | `effects/state.rs` | 1011 |
| `PlaceEnergyUnderMember` | `execute_place_energy_under_member` | `effects/state.rs` | — |
| `PositionChange` | `execute_position_change` | `effects/state.rs` | — |
| `Rotation` | `execute_rotation` | `effects/state.rs` | — |
| `ModifyScore` | `execute_modify_score` | `effects/score.rs` | 14 |
| `ModifyRequiredHearts` | `execute_modify_required_hearts` | `effects/score.rs` | 248 |
| `ModifyRequiredHeartsSuccess` | `execute_modify_required_hearts_success` | `effects/score.rs` | 587 |
| `ModifyYellCount` | `execute_modify_yell_count` | `effects/score.rs` | 529 |
| `ModifyLimit` | `execute_modify_limit` | `effects/score.rs` | 558 |
| `Reveal` | `execute_reveal_effect` | `effects/misc.rs` | 21 |
| `RevealUntilLiveCard` | `execute_reveal_until_live_card` | `effects/misc.rs` | — |
| `RevealUntilChosenCard` | `execute_reveal_until_chosen_card` | `effects/misc.rs` | 138 |
| `RevealPerGroup` | `execute_reveal_per_group` | `effects/misc.rs` | — |
| `Custom` | `execute_custom` | `effects/misc.rs` | 88 |
| `Choice` | `execute_choice` | `effects/misc.rs` | 3224 |
| `PayEnergy` | `execute_pay_energy` | `effects/misc.rs` | 3357 |
| `DiscardUntilCount` | `execute_discard_until_count` | `effects/misc.rs` | 3377 |
| `Restriction` | `execute_restriction` | `effects/misc.rs` | 3416 |
| `ReYell` | `execute_re_yell` | `effects/misc.rs` | 3468 |
| `ActivationRestriction` | `execute_activation_restriction` | `effects/misc.rs` | 3506 |
| `ChooseRequiredHearts` | `execute_choose_required_hearts` | `effects/misc.rs` | 3520 |
| `ChooseTargetPlayer` | `execute_choose_target_player` | `effects/misc.rs` | 3533 |
| `Shuffle` | `execute_shuffle` | `effects/misc.rs` | 3557 |
| `PerformYell` | `execute_perform_yell` | `effects/misc.rs` | 3615 |
| `PlayBatonTouch` | `execute_play_baton_touch` | `effects/misc.rs` | 1772 |
| `GainResource` | `execute_gain_resource` | `effects/mod.rs` | 303 |
| `DoNothing` | *(no-op)* | `effects/mod.rs` | 787 |

### Quick-debug workflow

1. Find the ability in `cards/abilities.json` — note the `action` string and
   any nested effects (look for `gained_effect`, `primary_effect`,
   `alternative_effect`, `options`, `look_action`, `select_action`).
2. Look up the `action` in the table above — go to that handler.
3. If the ability has a `condition`, check `ability/condition/card.rs` for the
   condition evaluator and `ability/condition.rs` for the entry point.
4. If the ability has a `cost`, check `ability/cost.rs`.
5. If the ability is a compound (sequential/choice/conditional), check
   `ability/compound.rs`.
6. If the ability involves player choice, check `ability/choice.rs`.
7. If the ability involves card movement, check `ability/move_cards.rs`.
8. If the ability involves look/reveal/select, check `ability/look.rs`.

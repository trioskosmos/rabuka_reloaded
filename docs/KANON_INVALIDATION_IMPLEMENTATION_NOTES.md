# Kanon Invalidation Implementation Notes

## Scope

Read-only analysis of the invalidation path for:

- Kanon `PL!SP-bp2-001-R＋` — optional LiveStart invalidation of one stage
  Liella! member until live end, followed by a Liella! discard-to-hand
  recovery when invalidation actually occurs.
- Genki Zenkai `PL!S-pb1-019-L` — condition-gated self-invalidation of the
  card's LiveSuccess ability.

No Cargo or generated-artifact command was run during this analysis.

## Confirmed runtime gaps

- `engine/src/ability/effects/ability_effects.rs:229-279` auto-selects the
  first valid target instead of offering a choice.
- The current Kanon effect excludes the activating card unconditionally,
  although the printed text does not say that self is excluded.
- `GameState.negated_abilities` stores only a card ID, so one invalidation
  suppresses every trigger on the card.
- `engine/src/core/game_state/abilities.rs:1647-1733` checks the unscoped
  list when resolving an ability.
- `engine/src/turn/triggers.rs:160-312` and `:320-490` skip cards using the
  unscoped list during LiveStart and LiveSuccess scans.
- No state cleanup removes the unscoped list at live end.
- `action_success_condition` is currently decoded as `Condition::AlwaysTrue`
  in `engine/src/core/card.rs:3270-3281`, so the follow-up cannot distinguish
  success from a skipped or ineffective invalidation.
- Only two invalidation effects exist in the current corpus: Kanon and Genki
  Zenkai. Only Kanon uses `action_reference: "invalidate_ability"`.

## Parser finding

`target_trigger` is already supported by the Rust effect model and decoder,
but the parser's generic `無効に` action rule did not populate it.

The required rule is to take the last recognized trigger icon before the first
`能力` token. This avoids choosing the earlier heart icon in Genki's text:

```text
{{heart_02.png|heart02}}{{live_success.png|ライブ成功時}}能力
```

Expected values:

- Kanon: `ライブ開始時`
- Genki Zenkai: `ライブ成功時`

The parser rule and schema now include this field, and parser tests cover both
cases.

## Scoped state design

Add a serializable runtime record:

```rust
pub struct AbilityInvalidation {
    pub card_id: i16,
    pub trigger: AbilityTrigger,
    pub duration: Duration,
    pub created_turn: u8,
}
```

Keep `negated_abilities` as a legacy all-trigger fallback for older serialized
states. New invalidations should be written only to the scoped list.

Add state helpers:

- `is_ability_invalidated(card_id, trigger)`
- `card_has_ability_trigger(card_id, trigger)`
- `try_add_ability_invalidation(card_id, trigger, duration)`

`try_add_ability_invalidation` must return true only when the target has the
requested trigger and the invalidation was not already present. This is what
makes a re-pick of an already-invalid target fail to trigger the recovery.

## Target-choice continuation

The Kanon primary should:

1. Build eligible stage positions using the effect's card/group filters.
2. Include self when the printed filter permits it.
3. Offer one `SelectCard` choice with `allow_skip` from `effect.optional`.
4. Store the primary effect in pending actions.
5. On answer, re-run the primary with `selected_cards` populated.
6. Add the scoped invalidation and record the actual action result.
7. Let the existing conditional-on-result continuation execute its follow-up.

The existing `conditional_on_result` continuation already saves a finish
command after a pending choice. The finish command must retain its
`result_condition`; the finish currently removes only the primary and outer
condition.

A resolver-local result can avoid changing every executor signature:

```rust
last_action_result: Option<(ActionType, bool)>
```

The invalidation handler sets it to `(InvalidateAbility, actual_success)`.
The conditional-on-result evaluator consults it only when the condition's
`action_reference` matches, leaving the other result-condition abilities on
their existing condition path.

## Scoped trigger checks

Replace raw `negated_abilities.contains(card_id)` checks with the scoped
helper at:

- Debut scan: `engine/src/turn/triggers.rs:11-134`
- LiveStart scan: `engine/src/turn/triggers.rs:160-312`
- LiveSuccess scan: `engine/src/turn/triggers.rs:320-490`
- Queue enqueue by text/index:
  `engine/src/core/game_state/abilities.rs:983-1152`
- Resolution defense:
  `engine/src/core/game_state/abilities.rs:1606-1758`

Do not add a constant-ability invalidation check. Kanon's target is LiveStart;
its constant abilities must remain active. This should be asserted with a
constant card such as Wien.

## Expiry

Extend `GameState::check_expired_effects` to remove scoped records using the
existing duration rules:

- `LiveEnd` expires when the turn is no longer `TurnPhase::Live`.
- `ThisTurn` expires when the turn number advances.
- `Permanent` remains.

The normal live phase sequence already reaches victory determination and then
normal-turn phases, so no new phase transition is needed. This also preserves
Q171: an effect with a live-end duration expires after the live sequence even
when no live card was set.

## Regression coverage

Expand
`engine/tests/test_modules/effects/conditional/kanon_invalidate_test.rs` with:

1. Exact target selection with multiple Liella! members.
2. Decline branch with no invalidation or recovery.
3. Self/no-applicable-ability branch.
4. Q106: selecting an already-invalid target produces no second recovery.
5. LiveStart blocked while another trigger, such as LiveSuccess, remains active.
6. Constant ability remains active and recalculates normally.
7. Live-end expiry, including the no-live-card sequence.

Also repair existing Kanon tests that auto-answer the old flow or use a
baton-touched target without a LiveStart ability:

- `engine/tests/test_modules/characterization/zero_tested_action_types_test.rs`
- `engine/tests/test_modules/characterization/decode_audit_behavior_pins_test.rs`
- `engine/tests/test_modules/integration/parser_issues_e2e_test.rs`
- `engine/tests/test_modules/integration/parser_issues_e2e_part2_test.rs`
- `engine/tests/test_modules/effects/ability_mod/genki_zenkai_test.rs`

## Verification

- Run the parser test after the parser edit.
- Regenerate `cards/abilities.json` and bytecode after the parser change.
- Run focused Kanon tests with debug logging and full output.
- Run the full Rust integration suite.
- Run `python cards/test_inventory.py --check` after test-ledger changes.

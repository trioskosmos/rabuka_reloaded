# Abilities Architecture

## Data Flow

```
Card text (Japanese)
  │
  ▼
parser.py: parse_ability()
  ├─ parse_cost()      → cost dict (pay_energy, move_cards, optional)
  ├─ parse_effect()    → 37 handlers in priority cascade
  │   ├─ _try_per_unit       "メンバー1人につき" → per_unit fields
  │   ├─ _try_conditional    "場合、" → condition + action
  │   ├─ _try_choice         "以下から1つを選ぶ" → options array
  │   ├─ _try_sequential     "その後、" → action array
  │   └─ fallback → parse_action() → dispatch table (80+ rules)
  └─ fields flow into flat dict
  │
  ▼
abilities.json (flat JSON, ~600 abilities)
  │
  ▼
engine: from_ability_effect()
  ├─ extract_modifiers() → ActionModifiers { target, count, card_type, ... }
  ├─ map action string → Effect::Variant
  │   Effect::MoveCards { source, destination, ... modifiers }
  │   Effect::GainResource { resource, ... modifiers }
  │   Effect::Draw { source, destination, modifiers }
  │   ...40+ variants
  │
  ▼
execute_effect()
  ├─ replacement effects check
  ├─ execute_effect_enum()  ← dispatch match
  │   ├─ Effect::MoveCards → execute_move_cards()
  │   ├─ Effect::Draw      → execute_draw()
  │   └─ ...matches each variant to executor
  │
  ▼
Executor functions (effects.rs, move_cards.rs)
  ├─ resolve target → player
  ├─ resolve count → static / per_unit / dynamic
  ├─ find matching cards → prompt if ambiguous
  └─ apply state change
```

## Field Hierarchy

Every effect has two kinds of fields:

```
┌──────────────────────────────────┐
│ ActionModifiers (shared)         │  ← Same fields for ALL action types
│   target: "self"/"opponent"/"both"│
│   count: u32                     │
│   card_type: "member"/"live"/... │
│   group_name / group_names       │
│   cost_limit: u32               │
│   optional / max / duration      │
│   per_unit / per_unit_count      │
│   state_change: "wait"/"active"  │
│   dynamic_count / location       │
│   source / destination           │
│   heart_colors / position        │
│   exclude_self / multiple_targets│
└──────────────────────────────────┘

┌──────────────────────────────────┐
│ Action-specific fields           │  ← Unique to each action type
│   move_cards: self_cost,         │
│     placement_order, distinct,    │
│     name_constraint              │
│   gain_resource: resource,       │
│     heart_color, icon_count      │
│   modify_score: operation, value │
│   change_state: state_change     │
│   draw: (source, dest already    │
│     in modifiers)                │
│   appear: (same)                 │
│   restriction: restriction_type  │
│   position_change: target_member │
│   ...etc                         │
└──────────────────────────────────┘
```

**Rule of thumb**: if the field appears in 3+ action types → ActionModifiers. If 1-2 → action-specific.

## Action Type Reference

Each primitive action is listed with: what it does, what the parser must produce, what the engine expects, and how to test it.

### move_cards
**Abilities:** 96 | **Engine:** execute_move_cards (move_cards.rs, 258 lines)
**What it does:** Moves cards between zones. Uses `take_n!` macro for matching+prompting.
**Parser must produce:** `source`, `destination`, `count`, `card_type` (optional), `group` (optional), `cost_limit` (optional)
**Engine uses:** source zone → find matching indices → prompt → remove → place in destination zone
**Engine ignores:** `activation_condition` (handled by execute_effect), `multiple_targets` (dead field), `group_names` (only `group_name` used)
**Sub-fields used:** `placement_order` ("any_order"), `self_cost`, `exclude_self`, `max`
**To test:** Play card with `source=X destination=Y` ability, set up cards in X, activate, make choices, verify cards in Y
**Example test:** `ruby_activation_search_live_from_discard` — self_cost stage→discard, then discard→hand with card_type filter

### gain_resource
**Abilities:** 110 | **Engine:** execute_gain_resource (effects.rs, ~120 lines)
**What it does:** Adds blade/heart to members. Handles per-unit scaling.
**Parser must produce:** `resource` ("blade"/"heart"), `count` (can be from icon count), `heart_color` (for heart), `card_type`, `group_name`, `per_unit*`
**Engine uses:** Find targets on stage matching filters → add modifier (blade/heart) → record temporary effect
**Sub-fields used:** `heart_colors` (choose-and-replace), `duration` (temporary), `per_unit*` (count scaling)
**To test:** Play card with gain_resource, set up targets on stage, activate, verify blade/heart modifier applied
**Test needed:** ✅ (blade gain), ❌ (heart gain), ❌ (per-unit gain)

### modify_score
**Abilities:** 67 | **Engine:** execute_modify_score (effects.rs, ~60 lines)
**What it does:** Adds/removes score from a live card. Per-unit scales by matching cards.
**Parser must produce:** `operation` ("add"/"remove"), `value`, `card_type` (optional), `group_name` (optional)
**Engine uses:** Find matching live cards → add score modifier → record temporary effect if duration set
**To test:** Play card with modify_score, set up live cards, activate, verify score changed

### draw_card
**Abilities:** 37 | **Engine:** execute_draw (effects.rs, ~30 lines)
**What it does:** Draws cards from deck to hand (or other destination).
**Parser must produce:** `source` (default "deck"), `destination` (default "hand"), `count`, `card_type` (optional), `per_unit*`
**Engine uses:** Draw from deck matching card_type → place in destination. Per-unit counts members on stage.
**To test:** Play card with draw, activate, verify hand increased by count, deck decreased

### change_state
**Abilities:** 38 | **Engine:** 3 helpers (effects.rs, ~80 lines)
**What it does:** Sets wait/active state on energy or members.
**Parser must produce:** `state_change` ("wait"/"active"), `card_type` (for member/energy), `count`, `cost_limit`
**Engine uses:** 3 paths: energy deck→zone (draw), member stage (orientation modifier), energy zone (active count)
**To test:** Play card with change_state, set up targets, activate, verify state changed

### restriction
**Abilities:** 21 | **Engine:** execute_restriction (effects.rs, ~15 lines)
**What it does:** Adds a prohibition to the game state (e.g., "cannot activate by effect").
**Parser must produce:** `restriction_type`, `card_type` (optional), `duration` (optional)
**Engine uses:** Push to prohibition_effects list. Checked during game operations.
**To test:** Play card with restriction, attempt restricted action, verify it fails

### position_change
**Abilities:** 13 | **Engine:** execute_position_change (effects.rs, ~40 lines)
**What it does:** Moves members between stage areas.
**Parser must produce:** `position`, `card_type`, `group`, `multiple_targets`
**Engine uses:** Find member at position → move to new area → swap if occupied
**To test:** Play card with position_change, place member at position, activate, verify member moved

### look_and_select
**Abilities:** 57 | **Engine:** execute_look_and_select (structural)
**What it does:** Looks at cards (look_action) then player selects from them (select_action).
**Parser must produce:** `look_action` (effect dict), `select_action` (effect dict, typically sequential)
**Engine uses:** peek cards from deck → show to player → player picks → remaining to discard
**Sub-actions are themselves effects** — look_action is typically look_at, select_action is typically sequential move_cards
**To test:** Set up deck with known cards, activate, verify looked-at cards match, select, verify moved to correct zone

### conditional_alternative
**Abilities:** 7 | **Engine:** execute_conditional_alternative (structural)
**What it does:** If condition is met, do primary; else do alternative.
**Parser must produce:** `condition` dict, `primary_effect`, `alternative_effect`
**Engine uses:** Evaluate condition → prompt player for choice → execute chosen effect
**To test:** Set up game with condition met/not met, activate, verify correct effect runs

### choice
**Abilities:** 10 | **Engine:** execute_choice (structural)
**What it does:** Player picks from a list of options.
**Parser must produce:** `options` array (each is an effect dict), `choice_type`, `choice_modifier`
**Engine uses:** Show options to player → player picks → execute chosen option
**To test:** Activate, verify prompt shows correct options, select, verify state changed

## Japanese Text → Engine Pipeline

When debugging "why does this card work/not work?", trace through:

1. **Is the text parsed correctly?** → Run `python cards/ability_extraction/parser.py` and check abilities.json
2. **Does from_ability_effect handle the action?** → Check `effect.rs` for the action string match arm
3. **Does the executor use all needed fields?** → Check the executor function for `modifiers.xxx` or `effect.xxx` reads
4. **Does the game state change match?** → Write a gameplay test

### Common Parsing Issues

| Symptom | Root Cause | Fix Location |
|---------|-----------|--------------|
| Field in parser output but engine ignores it | Executor doesn't read that field | Check executor function body |
| Field missing in parser output | Handler doesn't set it | Check `_try_*` function or dispatch table |
| Wrong action type | Dispatch table priority issue | Check `_R` rule order in `parse_action` |
| Condition not evaluated | `parse_condition` handler missing | Check early-return handlers or `_extract_generic_fields` |
| Choice not showing | `_try_choice` not matching | Check CHOICE_MARKER + structural handlers |

## Writing a New Test

### Quick start (copy this template)
```rust
#[test]
fn card_name_what_it_does() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let my_card = game.id("PL!-xxx-XXX-X");
    let target_card = game.id("PL!-sd1-XXX-SD");  // filler

    game.add_to_hand(my_card);
    game.add_to_discard(target_card);
    game.give_energy(5);  // card cost + 1
    game.play_to_stage(my_card, MemberArea::Center);
    game.activate_ability(my_card);

    // If choice needed:
    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    // Assert expected state:
    assert!(game.state.player1.hand.cards.contains(&target_card));
}
```

### Debug checklist when a test fails
1. `play_to_stage failed: Could not pay N energy` → increase `give_energy()`
2. `No pending choice` → auto-resolved (only 1 valid target). Add more valid targets.
3. Wrong card filtered → check `card_type`, `cost_limit`, `group` in parsed JSON
4. Index out of bounds in `select_indices` → print the zone contents first
5. Wrong effect executed → check `from_ability_effect` match arms
6. State didn't change → check executor reads the right fields

### Test coverage map
```
✅ TESTED:
  move_cards: self_cost stage→discard + discard→hand with card_type filter
  move_cards: self_cost stage→discard + discard→hand with member_card filter
  move_cards: discard→hand with group filter (虹ヶ咲)
  draw_card: sequential (draw→discard, draw 2→discard 1, draw 2→discard 2)
  look_and_select: deck_top 3→looked→select 1→hand, rest→discard
  change_state: wait on opponent cost≤4 member
  change_state: energy deck→zone in wait state

❌ NEED TESTS:
  gain_resource: blade gain (with per-unit)
  gain_resource: heart gain (with choose-and-replace)
  modify_score: add score (with condition)
  restriction: "cannot activate by effect"
  position_change: swap members
  change_state: activate energy in zone
  conditional_alternative: score mod choice
  choice: answer-based (エマパンチ, アイ♡スクリーム)
  pay_energy: as cost (non-optional)
```

## File Map

| Concern | File | Lines |
|---------|------|-------|
| Parser dispatch table | `cards/ability_extraction/parser.py` | 1720-1833 |
| Effect handler cascade | `parser.py` | 2920-2956 |
| Condition parser | `parser.py` | 1107-1151 |
| normalize_action (inlined) | `parser.py` | 1732-1793 |
| Engine field extraction | `engine/src/card.rs` | 620-644 |
| Effect enum | `engine/src/effect.rs` | 4-267 |
| from_ability_effect | `engine/src/effect.rs` | 270-575 |
| execute_effect dispatch | `engine/src/ability/effects.rs` | 69-147 |
| Executor functions | `engine/src/ability/effects.rs` | 400-1480 |
| move_cards executor | `engine/src/ability/move_cards.rs` | 18-258 |
| take_n! macro | `engine/src/ability/move_cards.rs` | 9-25 |
| match_cards_in_zone helper | `engine/src/ability/resolver.rs` | 52-75 |
| GameState (player resolution) | `engine/src/game_state.rs` | 996-1005 |
| Test infrastructure | `engine/tests/helpers.rs` | 1-205 |
| Gameplay tests | `engine/tests/gameplay_test.rs` | 1-527 |

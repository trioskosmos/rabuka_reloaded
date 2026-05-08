"""Cross-reference parsed abilities against engine capabilities.

Finds real problems: unknown action types, missing required fields,
type mismatches, invalid values, etc.
"""
import json
from collections import defaultdict

ENGINE_ACTIONS = [
    'none', 'sequential', 'conditional_alternative', 'look_and_select',
    'draw', 'draw_card', 'draw_until_count', 'discard_card', 'move_cards',
    'gain_resource', 'change_state', 'modify_score', 'modify_required_hearts',
    'set_cost', 'set_blade_type', 'set_heart_type', 'activate_ability',
    'invalidate_ability', 'gain_ability', 'play_baton_touch', 'reveal',
    'select', 'look_at', 'modify_required_hearts_global', 'modify_yell_count',
    'place_energy_under_member', 'activation_cost', 'position_change',
    'formation_change', 'appear', 'choice', 'pay_energy', 'set_card_identity',
    'repeat_procedure', 'discard_until_count', 'restriction',
    're_yell', 'activation_restriction', 'choose_required_hearts',
    'modify_limit', 'set_blade_count', 'do_nothing',
    'set_required_hearts', 'set_score', 'specify_heart_color',
    'modify_required_hearts_success', 'set_cost_to_use', 'all_blade_timing',
    'set_card_identity_all_regions', 'shuffle', 'reveal_per_group',
    'conditional_on_result', 'conditional_on_optional', 'modify_cost',
    'reveal_until_live_card', 'custom',
]

ENGINE_CONDITION_TYPES = [
    'compound', 'comparison_condition', 'location_condition',
    'card_count_condition', 'group_condition', 'position_condition',
    'appearance_condition', 'temporal_condition', 'state_condition',
    'energy_state_condition', 'movement_condition', 'ability_negation_condition',
    'or_condition', 'any_of_condition', 'score_threshold_condition',
    'choice_condition', 'position_change_condition', 'state_change_condition',
    'opponent_choice_condition', 'opponent_live_success',
    'complex_condition', 'no_excess_heart',
]

ENGINE_COST_TYPES = [
    'move_cards', 'pay_energy', 'sequential_cost', 'change_state',
    'reveal', 'choice_condition', 'energy_condition',
    'place_energy_under_member', 'custom',
]

VALID_STATES = {'wait', 'active'}
VALID_OPERATIONS = {'add', 'remove', 'set', 'subtract', 'increase', 'decrease'}


def check():
    data = json.load(open('cards/abilities.json', encoding='utf-8'))
    entries = data['unique_abilities']
    problems = []

    for i, entry in enumerate(entries):
        t = entry.get('triggerless_text', '')
        eff = entry.get('effect') or {}
        cost = entry.get('cost') or {}
        if not t:
            continue

        _check_effect(eff, i, t[:50], problems)
        _check_cost(cost, i, t[:50], problems)

    # Report
    by_cat = defaultdict(list)
    for idx, cat, detail, ctx in problems:
        by_cat[cat].append((idx, detail, ctx))

    print(f'Total problems found: {len(problems)}')
    print()
    for cat, items in sorted(by_cat.items(), key=lambda x: -len(x[1])):
        print(f'  {cat}: {len(items)}')
        for idx, detail, ctx in items[:5]:
            print(f'    #{idx}: {detail}')
            print(f'      {ctx}')
        if len(items) > 5:
            print(f'    ... and {len(items)-5} more')
        print()


def _check_effect(eff, idx, ctx, problems):
    if not isinstance(eff, dict):
        return
    action = eff.get('action', '')

    # Unknown action
    if action and action not in ENGINE_ACTIONS:
        problems.append((idx, 'UNKNOWN_ACTION', 'action=' + str(action), ctx))

    # Sequential without actions
    if action == 'sequential' and not eff.get('actions'):
        problems.append((idx, 'EMPTY_SEQUENTIAL', 'no actions', ctx))

    # gain_resource without resource
    if action == 'gain_resource' and 'resource' not in eff:
        problems.append((idx, 'GAIN_NO_RESOURCE', 'no resource', ctx))

    # change_state with invalid state
    if action == 'change_state':
        sc = eff.get('state_change', '')
        if sc and sc not in VALID_STATES:
            problems.append((idx, 'BAD_STATE', 'state=' + str(sc), ctx))

    # per_unit without count
    if eff.get('per_unit') and not eff.get('per_unit_count'):
        problems.append((idx, 'PER_UNIT_NO_COUNT', 'per_unit with no per_unit_count', ctx))

    # cost_limit without operator
    cl = eff.get('cost_limit')
    clo = eff.get('cost_limit_operator')
    if cl is not None and clo is None:
        problems.append((idx, 'MISSING_COST_OP', 'cost_limit=' + str(cl) + ' no operator', ctx))

    # Check condition
    cond = eff.get('condition')
    if isinstance(cond, dict):
        _check_condition(cond, idx, ctx, problems)

    # Check duration
    dur = eff.get('duration')
    if dur and dur not in ('live_end', 'this_turn', 'turn_end', 'this_live',
                           'as_long_as', 'permanent', 'unless'):
        problems.append((idx, 'BAD_DURATION', 'duration=' + str(dur), ctx))

    # Recurse into sub-actions
    for key in ('actions', 'options'):
        for sub in eff.get(key, []):
            if isinstance(sub, dict):
                _check_effect(sub, idx, ctx, problems)
    for key in ('primary_effect', 'followup_action', 'optional_action',
                'conditional_action', 'look_action', 'select_action',
                'opponent_action', 'alternative_effect'):
        sub = eff.get(key)
        if isinstance(sub, dict):
            _check_effect(sub, idx, ctx, problems)


def _check_condition(cond, idx, ctx, problems):
    if not isinstance(cond, dict):
        return
    ct = cond.get('type', '')
    # Skip sub-condition types that are handled inside temporal_condition or by the engine
    TEMPORAL_SUB_TYPES = {'not_moved', 'has_moved', 'no_excess_heart', 'opponent_live_success', 'otherwise_condition'}
    if ct and ct not in ENGINE_CONDITION_TYPES and ct not in TEMPORAL_SUB_TYPES:
        problems.append((idx, 'UNKNOWN_COND_TYPE', 'type=' + str(ct), ctx))

    # Condition with count but no operator
    if ct in ('card_count_condition', 'comparison_condition'):
        if 'count' in cond and 'operator' not in cond and 'values' not in cond:
            problems.append((idx, 'COND_NO_OP', ct + ' has count but no operator', ctx))

    # Condition with operator but no count (skip comparison_condition with comparison_target — self vs opponent)
    if ct in ('card_count_condition', 'comparison_condition'):
        if 'operator' in cond and 'count' not in cond:
            if ct == 'comparison_condition' and cond.get('comparison_target'):
                pass  # self vs opponent, no count needed
            else:
                problems.append((idx, 'COND_NO_COUNT', ct + ' has operator but no count', ctx))

    # Location condition without location (check both singular and plural)
    if ct == 'location_condition' and 'location' not in cond and 'locations' not in cond:
        problems.append((idx, 'COND_NO_LOC', 'location_condition without location', ctx))

    # Recurse into sub-conditions
    for sub in cond.get('conditions', []):
        _check_condition(sub, idx, ctx, problems)
    for key in ('cause', 'condition', 'effect'):
        sub = cond.get(key)
        if isinstance(sub, dict):
            _check_condition(sub, idx, ctx, problems)


def _check_cost(cost, idx, ctx, problems):
    if not isinstance(cost, dict):
        return
    ct = cost.get('type', '')
    if ct and ct not in ENGINE_COST_TYPES:
        problems.append((idx, 'UNKNOWN_COST_TYPE', 'type=' + str(ct), ctx))

    if ct == 'sequential_cost' and not cost.get('costs'):
        problems.append((idx, 'EMPTY_SEQ_COST', 'sequential_cost with no costs', ctx))

    for sub in cost.get('costs', []):
        if isinstance(sub, dict):
            _check_cost(sub, idx, ctx, problems)


if __name__ == '__main__':
    check()

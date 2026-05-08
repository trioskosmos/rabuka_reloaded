#!/usr/bin/env python3
"""Cross-reference validator for abilities.json.

For each ability entry, checks parsed fields against what the engine expects.
Reports mismatches, missing required fields, and suspicious values.
"""
import json, re, sys
from pathlib import Path
from collections import defaultdict

ABILITIES = Path(__file__).parent / 'abilities.json'

# === Action-type schemas: {action: {required: [...], optional: {...}, valid_values: {...}}} ===
SCHEMAS = {
    'move_cards': {
        'required': ['source', 'destination', 'count'],
        'valid_sources': ['hand', 'discard', 'deck', 'deck_top', 'deck_bottom', 'stage',
                          'energy_deck', 'energy_zone', 'live_card_zone', 'success_live_zone',
                          'revealed_cards', 'revealed_remaining', 'revealed_card',
                          'under_member', 'selected_cards'],
        'valid_dests': ['hand', 'discard', 'deck', 'deck_top', 'deck_bottom',
                         'stage', 'energy_zone', 'live_card_zone', 'success_live_zone',
                         'empty_area', 'same_area', 'under_member'],
    },
    'gain_resource': {
        'required': ['resource', 'count'],
        'valid_resources': ['blade', 'heart', 'heart01', 'heart02', 'heart03',
                            'heart04', 'heart05', 'heart06', 'energy', 'generic'],
    },
    'draw_card': {
        'required': ['count'],
    },
    'modify_score': {
        'required': ['operation', 'value'],
        'valid_operations': ['add', 'remove', 'set'],
    },
    'modify_required_hearts': {
        'required': ['heart_color', 'count'],
    },
    'change_state': {
        'required': ['state_change'],
        'valid_states': ['wait', 'active'],
    },
    'look_and_select': {
        'required': ['look_action', 'select_action'],
    },
    'conditional_on_result': {
        'required': ['result_condition', 'followup_action'],
    },
    'conditional_on_optional': {
        'required': ['optional_action', 'conditional_action'],
    },
    'gain_ability': {
        'required': [],
    },
    'restriction': {
        'required': ['restriction_type'],
    },
    'sequential': {
        'required': ['actions'],
    },
    'position_change': {},
    'appear': {},
    'look_at': {'required': ['count']},
    'reveal': {'required': ['count']},
    'select': {'required': ['count']},
    'modify_cost': {},
    'set_blade_count': {'required': ['count']},
    'set_blade_type': {},
    'play_baton_touch': {},
    're_yell': {},
    'invalidate_ability': {},
    'activate_ability': {},
    'formation_change': {},
    'set_card_identity': {},
    'set_score': {},
    'do_nothing': {},
    'shuffle': {},
    'custom': {},
}

CONDITION_TYPES = [
    'compound', 'card_count_condition', 'location_condition', 'comparison_condition',
    'group_condition', 'appearance_condition', 'temporal_condition',
    'position_condition', 'movement_condition', 'state_condition',
    'ability_negation_condition', 'or_condition', 'state_change_condition',
    'energy_state_condition', 'score_threshold_condition', 'custom',
]

COST_TYPES = [
    'move_cards', 'pay_energy', 'sequential_cost', 'change_state',
    'reveal', 'choice_condition', 'energy_condition',
    'place_energy_under_member', 'custom',
]


def load():
    return json.load(open(ABILITIES, encoding='utf-8'))


def validate():
    data = load()
    entries = data['unique_abilities']
    issues = defaultdict(list)

    for i, entry in enumerate(entries):
        t = entry.get('triggerless_text', '')
        full = entry.get('full_text', '')
        if not t:
            continue

        # Check 1: full_text vs triggerless_text consistency
        if full and not full.endswith(t) and not full.startswith(t):
            issues['text_mismatch'].append((i, full[:60], t[:60]))

        # Check 2: null abilities that shouldn't be
        if entry.get('is_null') and t.strip() and ')' not in t and '(' not in t:
            issues['false_null'].append((i, t[:60]))

        effect = entry.get('effect') or {}
        cost = entry.get('cost') or {}

        # Check 3: effect fields
        _check_effect(effect, i, t[:40], issues)

        # Check 4: cost fields
        _check_cost(cost, i, t[:40], issues)

    # Print report
    _print_report(issues, len(entries))


def _check_effect(eff, idx, context, issues):
    action = eff.get('action', '')
    if not action:
        return

    schema = SCHEMAS.get(action)
    if not schema:
        issues['unknown_action'].append((idx, action, context))
        return

    # Check required fields
    for field in schema.get('required', []):
        if field not in eff or eff[field] is None:
            issues['missing_required'].append((idx, f'{action}.{field}', context))
        elif isinstance(eff[field], (list, dict)) and not eff[field]:
            issues['empty_required'].append((idx, f'{action}.{field}', context))

    # Check valid values
    if action == 'move_cards':
        src = eff.get('source')
        if src and src not in schema['valid_sources']:
            issues['invalid_source'].append((idx, f'source={src}', context))
        dst = eff.get('destination')
        if dst and dst not in schema['valid_dests']:
            issues['invalid_dest'].append((idx, f'dest={dst}', context))

    if action == 'gain_resource':
        res = eff.get('resource')
        if res and res not in schema['valid_resources']:
            issues['invalid_resource'].append((idx, f'resource={res}', context))

    if action == 'change_state':
        sc = eff.get('state_change')
        if sc and sc not in schema['valid_states']:
            issues['invalid_state'].append((idx, f'state={sc}', context))

    if action == 'modify_score':
        op = eff.get('operation')
        if op and op not in schema['valid_operations']:
            issues['invalid_operation'].append((idx, f'op={op}', context))

    # Check condition sub-fields
    cond = eff.get('condition')
    if cond and isinstance(cond, dict):
        ct = cond.get('type')
        if ct and ct not in CONDITION_TYPES:
            issues['invalid_condition_type'].append((idx, f'cond_type={ct}', context))
        if ct and ct == 'card_count_condition':
            if 'count' not in cond:
                issues['cond_missing_count'].append((idx, '', context))
        if ct and ct == 'location_condition':
            if 'location' not in cond:
                issues['cond_missing_location'].append((idx, '', context))

    # Recurse into sub-actions
    for sub_key in ('actions', 'options', 'primary_effect', 'followup_action',
                    'optional_action', 'conditional_action',
                    'look_action', 'select_action', 'opponent_action'):
        sub = eff.get(sub_key)
        if isinstance(sub, dict):
            _check_effect(sub, idx, context, issues)
        elif isinstance(sub, list):
            for item in sub:
                if isinstance(item, dict):
                    _check_effect(item, idx, context, issues)


def _check_cost(cost, idx, context, issues):
    ct = cost.get('type', '')
    if not ct:
        return
    if ct and ct not in COST_TYPES:
        issues['unknown_cost_type'].append((idx, ct, context))
    # Check sequential_cost sub-costs
    if ct == 'sequential_cost':
        costs = cost.get('costs', [])
        if not costs:
            issues['empty_sequential_cost'].append((idx, '', context))
        for sub in costs:
            if isinstance(sub, dict):
                _check_cost(sub, idx, context, issues)


def _print_report(issues, total):
    print(f'Ability Validation Report ({total} entries)\n')

    all_issues = [(k, v) for k, v in issues.items() if v]
    all_issues.sort(key=lambda x: -len(x[1]))

    for category, items in all_issues:
        print(f'  {category} ({len(items)}):')
        for idx, detail, ctx in items[:5]:
            print(f'    #{idx} {detail}')
            if ctx:
                print(f'        {ctx}')
        if len(items) > 5:
            print(f'    ... and {len(items) - 5} more')
        print()

    total_issues = sum(len(v) for v in issues.values())
    print(f'Total issues found: {total_issues}')


if __name__ == '__main__':
    validate()

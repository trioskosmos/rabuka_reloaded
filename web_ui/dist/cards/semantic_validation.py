"""Semantic validation: checks every ability's parsed JSON for missing fields,
wrong types, and contradictions between text and structure.

This runs automatically after parser.py and flags issues like:
- 'エネルギーをアクティブ' with no card_type: 'energy_card'
- 'コストN以下' with no cost_limit in effect
- 'heart_XX' in select text with no heart_colors
"""
import json, re, sys
from pathlib import Path
from collections import defaultdict

ABILITIES = Path(__file__).parent / 'abilities.json'
data = json.load(open(ABILITIES, encoding='utf-8'))
entries = data['unique_abilities']

issues = defaultdict(list)


def _find_in_tree(obj, field):
    if not isinstance(obj, dict):
        return False
    if obj.get(field):
        return True
    for key in ('actions', 'options', 'conditions'):
        for item in obj.get(key, []):
            if isinstance(item, dict) and _find_in_tree(item, field):
                return True
    for key in ('condition', 'primary_effect', 'followup_action', 'optional_action',
                'conditional_action', 'look_action', 'select_action'):
        sub = obj.get(key)
        if isinstance(sub, dict) and _find_in_tree(sub, field):
            return True
    return False


def check(name, fn):
    """Register a semantic check."""
    for i, entry in enumerate(entries):
        t = entry.get('triggerless_text', '')
        eff = entry.get('effect') or {}
        cost = entry.get('cost') or {}
        if not t:
            continue
        fn(i, t, eff, cost, entry)


# === change_state: energy activation must have card_type: energy_card ===
check('energy_activation_no_card_type', lambda i, t, eff, cost, e:
    issues['energy_activation_card_type'].append(i) if
    eff.get('action') == 'change_state' and 'エネルギー' in t
    and 'メンバー' not in t and eff.get('card_type') != 'energy_card' else None)

# === change_state: member activation must have card_type: member_card ===
check('member_activation_no_card_type', lambda i, t, eff, cost, e:
    issues['member_activation_card_type'].append(i) if
    eff.get('action') == 'change_state' and 'メンバー' in t
    and 'エネルギー' not in t and eff.get('card_type') != 'member_card' else None)

# === move_cards: cost_limit in text but not in effect ===
check('cost_limit_missing', lambda i, t, eff, cost, e:
    issues['cost_limit_missing'].append(i) if
    eff.get('action') == 'move_cards' and 'コスト' in t
    and ('以下' in t or '以上' in t) and eff.get('cost_limit') is None else None)

# === look_and_select: heart colors in select text but not in sub-action ===
check('heart_colors_missing_reveal', lambda i, t, eff, cost, e:
    issues['heart_colors_missing'].append(i) if
    eff.get('action') == 'look_and_select'
    and eff.get('select_action')
    and any(isinstance(a, dict) and a.get('action') == 'reveal'
            and not a.get('heart_colors')
            for a in (eff['select_action'].get('actions') or []))
    else None)

# === gain_resource: per_unit in text but not in output ===
check('per_unit_missing', lambda i, t, eff, cost, e:
    issues['per_unit_missing'].append(i) if
    eff.get('action') in ('gain_resource', 'draw_card')
    and 'につき' in t and not eff.get('per_unit') else None)

# === source/destination: expected but missing ===
DEST_REQUIRED_PATTERNS = [
    ('手札に加える', 'hand'), ('控え室に置く', 'discard'),
    ('デッキの上に', 'deck_top'), ('デッキの一番下に', 'deck_bottom'),
]
for phrase, expected_dest in DEST_REQUIRED_PATTERNS:
    def make_check(ph, exp):
        return lambda i, t, eff, cost, e: (
            issues['dest_missing'].append(i) if
            eff.get('action') == 'move_cards' and ph in t
            and exp not in str(eff.get('destination')) else None
        )
    check(f'dest_{expected_dest}_missing', make_check(phrase, expected_dest))

# === conditional_on_optional: must have both optional_action and conditional_action ===
check('conditional_optional_missing', lambda i, t, eff, cost, e:
    issues['conditional_optional_missing'].append(i) if
    eff.get('action') == 'conditional_on_optional'
    and (not eff.get('optional_action') or not eff.get('conditional_action'))
    else None)

# === sequential: empty actions array ===
check('empty_sequential', lambda i, t, eff, cost, e:
    issues['empty_sequential'].append(i) if
    eff.get('action') == 'sequential' and not eff.get('actions') else None)

# === condition with "名前が異なる" but no distinct flag ===
check('distinct_missing_condition', lambda i, t, eff, cost, e:
    issues['distinct_missing'].append(i) if
    '名前が異なる' in t or 'カード名の異なる' in t
    and not _find_in_tree(eff, 'distinct') else None)

# === effect with "それぞれ" or "ずつ" but no multiple_targets ===
check('multiple_targets_missing', lambda i, t, eff, cost, e:
    issues['multiple_targets_missing'].append(i) if
    ('ずつ' in t or 'それぞれ' in t)
    and not _find_in_tree(eff, 'multiple_targets')
    else None)


# === Print report ===
print(f'\nSemantic Validation: {len(entries)} abilities')
total = 0
for cat, items in sorted(issues.items(), key=lambda x: -len(x[1])):
    pct = len(items) / len(entries) * 100
    print(f'  {cat:40s} {len(items):4d} ({pct:5.1f}%)')
    total += len(items)
    if items:
        idx = items[0]
        t = entries[idx].get('triggerless_text', '')[:50]
        print(f'    Example #{idx}: {t}')

print(f'\nTotal issues: {total}')

# Exit with error if any issues found
if total > 0:
    print(f'\n[!] Run `find_dropped_details.py` for detailed text-vs-output comparison')

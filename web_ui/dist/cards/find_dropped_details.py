#!/usr/bin/env python3
"""Find silently dropped details in parsed abilities.

Compares raw ability text against parsed JSON output to find
keywords and patterns present in the text but missing from the output.
"""
import json, re, sys
from pathlib import Path
from collections import defaultdict

ABILITIES = Path(__file__).parent / 'abilities.json'
data = json.load(open(ABILITIES, encoding='utf-8'))
entries = data['unique_abilities']

# Patterns that should generate structured fields but often get dropped
CHECKS = []

def check(name, text_pattern, field_check_fn):
    CHECKS.append((name, text_pattern, field_check_fn))

# Heart color filters in select — text has heart_04 but output has no heart_colors
check('heart_filter_dropped',
      lambda t: re.search(r'ハートに\{\{heart_\d+', t) is not None,
      lambda eff: eff.get('heart_colors') or eff.get('heart_color'))

# "元々" (original value) pattern
check('original_value_dropped',
      lambda t: '元々' in t,
      lambda eff: eff.get('original_value'))

# Per-unit modifier in text but not in output
check('per_unit_dropped',
      lambda t: 'につき' in t or 'ごとに' in t,
      lambda eff: eff.get('per_unit'))

# Heart selection (player chooses heart color)
check('heart_selection_dropped',
      lambda t: 'ハートの色を' in t or 'ハートを指定' in t,
      lambda eff: eff.get('heart_selection'))

# "それぞれ" / "ずつ" (each / respectively) — multiple targets
check('multiple_targets_dropped',
      lambda t: 'ずつ' in t or ('それぞれ' in t and 'それぞれ異なる' not in t),
      lambda eff: eff.get('multiple_targets'))

# Position requirements
check('position_dropped',
      lambda t: any(p in t for p in ['センター', '左サイド', '右サイド']),
      lambda eff: eff.get('position') or eff.get('activation_position') or eff.get('source_position'))

# "好きな枚数" (any number) pattern
check('any_number_dropped',
      lambda t: '好きな枚数' in t,
      lambda eff: eff.get('any_number'))

# Cost limit operator ("以下"/"以上") present but cost_limit missing
check('cost_limit_dropped',
      lambda t: re.search(r'コスト\d+', t) is not None,
      lambda eff: eff.get('cost_limit') is not None)

# "まで" (up to N) = max flag
check('max_dropped',
      lambda t: ('枚まで' in t or '人まで' in t or 'つまで' in t),
      lambda eff: eff.get('max'))

# "このメンバー以外"/"ほかの" = exclude_self
check('exclude_self_dropped',
      lambda t: 'このメンバー以外' in t or 'ほかのメンバー' in t,
      lambda eff: eff.get('exclude_self'))

# OR card types (AかB) present but not parsed
check('or_card_types_dropped',
      lambda t: re.search(r'(メンバーカード|ライブカード|エネルギーカード).+(?:か|又は|または).+(メンバーカード|ライブカード|エネルギーカード)', t) is not None,
      lambda eff: eff.get('or_card_types'))

# Multiple named characters in text
check('characters_dropped',
      lambda t: len(re.findall(r'「([^」]+)」', t)) >= 2,
      lambda eff: eff.get('characters'))

# "chosen" / "選んだカード" — source should be selected_cards
check('selected_cards_source_dropped',
      lambda t: '選んだカード' in t or 'これにより選ばれた' in t,
      lambda eff: eff.get('source') == 'selected_cards')

# Distinct name condition
check('distinct_dropped',
      lambda t: '名前が異なる' in t or '名前の異なる' in t or 'カード名の異なる' in t,
      lambda eff: eff.get('distinct'))

# Timing condition (appeared this turn, moved this turn)
check('timing_condition_dropped',
      lambda t: 'このターン' in t and ('登場' in t or '移動' in t),
      lambda eff: eff.get('timing_condition'))

# Original value modifier
check('original_count_dropped',
      lambda t: '元々の' in t and ('ブレード' in t or 'ハート' in t),
      lambda eff: eff.get('original_count'))

def _check_recursive(eff, field_check):
    if not isinstance(eff, dict):
        return False
    if field_check(eff):
        return True
    # Check condition sub-tree — this is where many fields end up
    cond = eff.get('condition')
    if isinstance(cond, dict):
        if field_check(cond):
            return True
        # Also recurse into compound conditions
        for sub_cond in cond.get('conditions', []):
            if isinstance(sub_cond, dict) and _check_recursive(sub_cond, field_check):
                return True
    # Check sub-actions
    for sub_key in ('actions', 'options', 'primary_effect', 'followup_action',
                    'optional_action', 'conditional_action',
                    'look_action', 'select_action', 'opponent_action'):
        sub = eff.get(sub_key)
        if isinstance(sub, dict):
            if _check_recursive(sub, field_check):
                return True
        elif isinstance(sub, list):
            for item in sub:
                if isinstance(item, dict) and _check_recursive(item, field_check):
                    return True
    return False


print('Finding silently dropped details in parsed abilities...')
print()

results = defaultdict(list)
total_affected = 0

for i, entry in enumerate(entries):
    t = entry.get('triggerless_text', '')
    if not t:
        continue
    full = entry.get('full_text', '')
    eff = entry.get('effect') or {}

    for name, text_pred, field_check in CHECKS:
        if text_pred(t):
            # Text has this pattern — check if it's in the parsed output
            try:
                present = field_check(eff)
            except:
                present = False
            if not present:
                # Check recursively in sub-actions
                present = _check_recursive(eff, field_check)
            if not present:
                results[name].append((i, t[:70]))

# Print report
print(f"{'Pattern':30s} {'Dropped':>7s} {'Total':>7s} {'%':>6s}")
print('-' * 52)
# Build lookup: name -> pred
_name_to_pred = {n: p for n, p, _ in CHECKS}
for name, checks_list in sorted(results.items(), key=lambda x: -len(x[1])):
    pred = _name_to_pred.get(name)
    total = sum(1 for e in entries if pred and pred(e.get('triggerless_text', ''))) if pred else 0
    dropped = len(checks_list)
    pct = dropped / total * 100 if total else 0
    print(f"{name:30s} {dropped:7d} {total:7d} {pct:5.1f}%")
    total_affected += dropped

print()
print(f"Total dropped details across all abilities: {total_affected}")

print()
print('Examples of dropped details:')
for name, checks_list in sorted(results.items(), key=lambda x: -len(x[1])):
    if checks_list:
        idx, txt = checks_list[0]
        print(f'\n  {name}:')
        print(f'    #{idx}: {txt}')

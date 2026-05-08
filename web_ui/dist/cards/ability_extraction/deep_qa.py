"""Deep QA analysis of abilities.json - refined checks."""
import json, os, re
from collections import defaultdict

abilities_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'abilities.json')
with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

def collect_all_effects(effect):
    if not effect:
        return []
    result = [effect]
    action = effect.get('action', '')
    if action in ('sequential',):
        for sub in effect.get('actions', []):
            result.extend(collect_all_effects(sub))
    elif action == 'choice':
        for sub in effect.get('options', []):
            result.extend(collect_all_effects(sub))
    elif action == 'look_and_select':
        result.extend(collect_all_effects(effect.get('look_action')))
        result.extend(collect_all_effects(effect.get('select_action')))
    elif action == 'conditional_alternative':
        result.extend(collect_all_effects(effect.get('primary_effect')))
        result.extend(collect_all_effects(effect.get('alternative_effect')))
    return result

def describe(ab):
    idx = abilities.index(ab)
    cards = ab.get('cards', [])
    card_str = cards[0] if cards else '?'
    ft = ab.get('full_text', '')[:200]
    return f"[{idx}] card={card_str}"

# Unicode normalization helper
def normalize(s):
    """Normalize unicode for comparison"""
    import unicodedata
    return unicodedata.normalize('NFKC', s)

results = {str(i): [] for i in range(1, 13)}

# ============================================================
# 1. Wrong source/destination (REFINED)
# ============================================================
print("=" * 80)
print("CATEGORY 1: WRONG SOURCE/DESTINATION")
print("=" * 80)

# Known valid source→keyword mappings (what the parser should produce for given JP text)
# These are cases where the JP text clearly indicates one source but parser says another
CLEAR_SOURCE_INDICATORS = {
    'deck': ['デッキ', '山札'],
    'deck_top': ['デッキの上', '山札の上'],
    'hand': ['手札'],
    'discard': ['控え室'],
    'energy_zone': ['エネルギーゾーン', 'エネルギーエリア'],
    'stage': ['ステージ', 'スチEージ'],
}
CLEAR_DEST_INDICATORS = {
    'hand': ['手札に加え', '手札に戻す'],
    'discard': ['控え室に置く'],
    'deck_top': ['デッキの上に置く', '山札の上に置く'],
    'deck_bottom': ['デッキの下に置く'],
    'energy_zone': ['エリアに置く', 'エネルギーゾーンに置く'],
    'empty_area': ['空いているエリア', '空いてるエリア'],
    'active': ['アクティブ'],
    'wait': ['ウェイト'],
}

for ab in abilities:
    ft = ab.get('full_text', '')
    for eff in collect_all_effects(ab.get('effect')):
        action = eff.get('action', '')
        src = eff.get('source')
        dst = eff.get('destination')
        text = eff.get('text', '')
        if not text:
            continue
        
        n_text = normalize(text)
        
        # Check: if source is "deck" but text clearly says "hand" (手札) without mentioning deck
        if src == 'deck' and '手札' in n_text and 'デッキ' not in n_text and '山札' not in n_text:
            results['1'].append((describe(ab), f"source=deck but text says 手札 (hand) and not デッキ text=\"{text[:100]}\""))
        
        # Check: if source is "hand" but action is draw_card (draw from deck, not hand)
        if action == 'draw_card' and src is not None and src != 'deck':
            results['1'].append((describe(ab), f"draw_card with source={src} (should probably be deck) text=\"{text[:100]}\""))
        
        # Check: destination=null for move_cards
        if action == 'move_cards' and dst is None:
            results['1'].append((describe(ab), f"move_cards with destination=null text=\"{text[:100]}\""))
        
        # Check: destination=discard for 手札に加える (should be hand)
        if dst == 'discard' and '手札に加え' in n_text:
            results['1'].append((describe(ab), f"destination=discard but text says 手札に加える (should be hand) text=\"{text[:100]}\""))

print(f"  Total category 1 issues: {len(results['1'])}")
for item in results['1']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['1']:
    print("  (none found)")

# ============================================================
# 2. Missing count on move_cards/draw_card
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 2: MISSING COUNT ON MOVE_CARDS/DRAW_CARD")
print("=" * 80)
for ab in abilities:
    for eff in collect_all_effects(ab.get('effect')):
        if eff.get('action') in ('move_cards', 'draw_card'):
            if 'count' not in eff or eff.get('count') is None:
                text = eff.get('text', '')[:100]
                results['2'].append((describe(ab), f"action={eff['action']} missing count text=\"{text}\""))
for item in results['2']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['2']:
    print("  (none found)")

# ============================================================
# 3. Wrong card_type
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 3: WRONG CARD_TYPE")
print("=" * 80)
for ab in abilities:
    ft = ab.get('full_text', '')
    for eff in collect_all_effects(ab.get('effect')):
        ct = eff.get('card_type')
        text = eff.get('text', '')
        if not text or not ct:
            continue
        if ct == 'card':
            continue
        n_text = normalize(text)
        if 'エネルギーカード' in n_text and ct != 'energy_card':
            results['3'].append((describe(ab), f"card_type={ct} but text says エネルギーカード text=\"{text[:100]}\""))
        if 'メンバーカード' in n_text and ct not in ('member_card',):
            results['3'].append((describe(ab), f"card_type={ct} but text says メンバーカード text=\"{text[:100]}\""))
        if 'ライブカード' in n_text and ct not in ('live_card',):
            results['3'].append((describe(ab), f"card_type={ct} but text says ライブカード text=\"{text[:100]}\""))
for item in results['3']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['3']:
    print("  (none found)")

# ============================================================
# 4. Missing state_change on change_state
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 4: MISSING STATE_CHANGE ON CHANGE_STATE")
print("=" * 80)
for ab in abilities:
    for eff in collect_all_effects(ab.get('effect')):
        if eff.get('action') == 'change_state':
            if 'state_change' not in eff or eff.get('state_change') is None:
                text = eff.get('text', '')[:100]
                results['4'].append((describe(ab), f"action=change_state missing state_change text=\"{text}\""))
for item in results['4']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['4']:
    print("  (none found)")

# ============================================================
# 5. per_unit without per_unit_count
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 5: PER_UNIT WITHOUT PER_UNIT_COUNT")
print("=" * 80)
for ab in abilities:
    for eff in collect_all_effects(ab.get('effect')):
        if eff.get('per_unit') and ('per_unit_count' not in eff or eff.get('per_unit_count') is None):
            text = eff.get('text', '')[:100]
            results['5'].append((describe(ab), f"per_unit=true but no per_unit_count text=\"{text}\""))
for item in results['5']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['5']:
    print("  (none found)")

# ============================================================
# 6. Missing all flag (すべての but all not true)
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 6: MISSING ALL FLAG")
print("=" * 80)
for ab in abilities:
    ft = ab.get('full_text', '')
    for eff in collect_all_effects(ab.get('effect')):
        text = eff.get('text', '')
        if not text:
            continue
        n_text = normalize(text)
        if 'すべての' in n_text or '全ての' in n_text or 'すべて' in n_text:
            if not eff.get('all'):
                results['6'].append((describe(ab), f"text has すべての/全ての/すべて but all not set text=\"{text[:100]}\""))
for item in results['6']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['6']:
    print("  (none found)")

# ============================================================
# 7. Missing multiple_targets (それぞれ/ずつ)
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 7: MISSING MULTIPLE_TARGETS")
print("=" * 80)
for ab in abilities:
    ft = ab.get('full_text', '')
    for eff in collect_all_effects(ab.get('effect')):
        text = eff.get('text', '')
        if not text:
            continue
        n_text = normalize(text)
        if 'それぞれ' in n_text or 'ずつ' in n_text:
            if not eff.get('multiple_targets'):
                results['7'].append((describe(ab), f"text has それぞれ/ずつ but multiple_targets not set text=\"{text[:100]}\""))
for item in results['7']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['7']:
    print("  (none found)")

# ============================================================
# 8. Condition with missing fields
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 8: CONDITIONS WITH MISSING FIELDS")
print("=" * 80)
def check_condition(cond, context):
    if not cond:
        return
    t = cond.get('type', '')
    if t == 'location_condition':
        if 'location' not in cond or cond.get('location') is None:
            results['8'].append((context, f"location_condition missing location field"))
        if 'target' not in cond or cond.get('target') is None:
            results['8'].append((context, f"location_condition missing target field"))
    if t == 'compound':
        for sub in cond.get('conditions', []):
            check_condition(sub, context)
    if t == 'group_condition':
        if 'group_names' not in cond or not cond.get('group_names'):
            results['8'].append((context, f"group_condition missing group_names field"))

for ab in abilities:
    desc = describe(ab)
    eff = ab.get('effect')
    if eff:
        check_condition(eff.get('condition'), desc)
        for sub_eff in collect_all_effects(eff):
            check_condition(sub_eff.get('condition'), desc)
# Deduplicate
seen = set()
unique_8 = []
for item in results['8']:
    key = f"{item[0]}|{item[1]}"
    if key not in seen:
        seen.add(key)
        unique_8.append(item)
results['8'] = unique_8

for item in results['8']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['8']:
    print("  (none found)")

# ============================================================
# 9. Bare do_nothing actions
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 9: BARE DO_NOTHING ACTIONS")
print("=" * 80)
for ab in abilities:
    for eff in collect_all_effects(ab.get('effect')):
        if eff.get('action') == 'do_nothing':
            results['9'].append((describe(ab), f"bare do_nothing action found text=\"{eff.get('text','')[:100]}\""))
for item in results['9']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['9']:
    print("  (none found)")

# ============================================================
# 10. Missing/empty action field
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 10: MISSING/EMPTY ACTION FIELD")
print("=" * 80)
for ab in abilities:
    for eff in collect_all_effects(ab.get('effect')):
        action = eff.get('action')
        if not action or action == '':
            text = eff.get('text', '')[:100]
            results['10'].append((describe(ab), f"empty/missing action field text=\"{text}\""))
        elif action == 'custom':
            text = eff.get('text', '')[:100]
            results['10'].append((describe(ab), f"action=custom (should be resolved) text=\"{text}\""))
for item in results['10']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['10']:
    print("  (none found)")

# ============================================================
# 11. Sequential with single action
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 11: SEQUENTIAL WITH SINGLE ACTION")
print("=" * 80)
for ab in abilities:
    eff = ab.get('effect')
    if not eff:
        continue
    if eff.get('action') == 'sequential':
        actions = eff.get('actions', [])
        if len(actions) <= 1:
            results['11'].append((describe(ab), f"sequential with {len(actions)} action(s) text=\"{eff.get('text','')[:100]}\""))
    if eff.get('action') == 'look_and_select':
        sa = eff.get('select_action')
        if sa and sa.get('action') == 'sequential':
            actions = sa.get('actions', [])
            if len(actions) <= 1:
                results['11'].append((describe(ab), f"select_action sequential with {len(actions)} action(s) text=\"{eff.get('text','')[:100]}\""))
for item in results['11']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['11']:
    print("  (none found)")

# ============================================================
# 12. Missing operator on comparison conditions
# ============================================================
print("\n" + "=" * 80)
print("CATEGORY 12: MISSING OPERATOR ON COMPARISON CONDITIONS")
print("=" * 80)
for ab in abilities:
    desc = describe(ab)
    eff = ab.get('effect')
    if eff:
        def check_op(cond):
            if not cond:
                return
            t = cond.get('type', '')
            if t in ('comparison_condition', 'card_count_condition') and 'operator' not in cond:
                results['12'].append((desc, f"{t} missing operator field"))
            if t == 'compound':
                for sub in cond.get('conditions', []):
                    check_op(sub)
        check_op(eff.get('condition'))
# Deduplicate
seen = set()
unique_12 = []
for item in results['12']:
    key = f"{item[0]}|{item[1]}"
    if key not in seen:
        seen.add(key)
        unique_12.append(item)
results['12'] = unique_12

for item in results['12']:
    print(f"  {item[0]}")
    print(f"    -> {item[1]}")
if not results['12']:
    print("  (none found)")

# ============================================================
# SUMMARY
# ============================================================
print("\n" + "=" * 80)
print("SUMMARY")
print("=" * 80)
for cat in sorted(results.keys()):
    count = len(results[cat])
    print(f"  Category {cat}: {count} issues{' (PASS)' if count == 0 else ''}")
total = sum(len(v) for v in results.values())
print(f"\n  TOTAL: {total} issues found")

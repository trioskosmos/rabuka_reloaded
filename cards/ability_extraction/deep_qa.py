"""Deep QA analysis of abilities.json - checks 12 categories of issues."""
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
    return f"[{idx}] card={card_str} text=\"{ft}\""

results = {str(i): [] for i in range(1, 13)}

# ============================================================
# 1. Wrong source/destination
# ============================================================
print("=" * 80)
print("CATEGORY 1: WRONG SOURCE/DESTINATION")
print("=" * 80)
keywords_src = {
    'deck': ['デッキ', '山札', 'チE��キ', 'チEーキ'],
    'deck_top': ['チE��キの上', 'チEーキの上', '山札の上'],
    'hand': ['手札'],
    'discard': ['控え室'],
    'energy_zone': ['エネルギーゾーン', 'エリア'],
    'stage': ['スチEージ', 'ステージ'],
    'wait_room': ['ウェイトルーム'],
    'memory': ['メモリー'],
}
keywords_dst = {
    'hand': ['手札に加える', '手札に戻す', '手札に置く'],
    'discard': ['控え室に置く', '控え室に送る'],
    'deck_top': ['チE��キの上に置く', 'デッキの上に置く'],
    'deck_bottom': ['チE��キの下に置く', 'デッキの下に置く'],
    'energy_zone': ['エネルギーゾーンに置く', 'エリアに置く', 'アクチE��ブにする'],
    'stage': ['スチEージに登場', 'ステージに登場'],
    'empty_area': ['空いてるエリア', 'ぁE��ぁE��リア'],
}
for ab in abilities:
    ft = ab.get('full_text', '')
    for eff in collect_all_effects(ab.get('effect')):
        src = eff.get('source')
        dst = eff.get('destination')
        text = eff.get('text', '')
        if not text:
            continue
        # Check source
        if src == 'deck' and '手札' in text and 'デッキ' not in text:
            results['1'].append((describe(ab), f"source=deck but text has 手札 (should be hand?) text={text[:80]}"))
        if src == 'hand' and 'デッキ' in text and '手札' not in text:
            results['1'].append((describe(ab), f"source=hand but text has デッキ (should be deck?) text={text[:80]}"))
        # Check destination
        if dst and 'move_cards' in eff.get('action','') or eff.get('action') == 'move_cards':
            if dst == 'hand' and '控え室' in text and '手札' not in text and '加え' not in text:
                results['1'].append((describe(ab), f"destination=hand but text={text[:80]}"))
            if dst == 'discard' and '手札' not in text and '控え室' in text and '引' not in text:
                pass  # this might be ok

# More thorough check: compare source/destination keywords
src_jp_to_en = {
    'デッキ': 'deck', '山札': 'deck', 'チE��キ': 'deck', 'チEーキ': 'deck',
    '手札': 'hand', '控え室': 'discard', 'エネルギー': 'energy_zone',
    'スチEージ': 'stage', 'ステージ': 'stage', 'メモリー': 'memory',
    'ウェイト': 'wait_room',
}
dst_jp_to_en = {
    '手札に': 'hand', '控え室に': 'discard', 'チE��キの上に': 'deck_top',
    'チEーキの上に': 'deck_top', 'デッキの上に': 'deck_top',
    'チE��キの下に': 'deck_bottom', 'デッキの下に': 'deck_bottom',
    'エネルギー': 'energy_zone', 'アクチE��ブ': 'active',
    'ウェイト': 'wait', '登場': 'stage', '空い': 'empty_area',
}

for ab in abilities:
    ft = ab.get('full_text', '')
    for eff in collect_all_effects(ab.get('effect')):
        src = eff.get('source')
        dst = eff.get('destination')
        text = eff.get('text', '')
        if not text:
            continue
        if eff.get('action') in ('move_cards', 'draw_card', 'look_at'):
            # Check if source contradicts text
            if src:
                # Find JP keywords in text
                found_jp_src = None
                for jp, en in src_jp_to_en.items():
                    if jp in text:
                        found_jp_src = en
                        break
                if found_jp_src and found_jp_src != src:
                    results['1'].append((describe(ab), f"WRONG SOURCE: src={src} but text contains '{jp}'→{found_jp_src} text=\"{text[:100]}\""))
            # Check if destination contradicts text
            if dst:
                found_jp_dst = None
                for jp, en in dst_jp_to_en.items():
                    if jp in text:
                        found_jp_dst = en
                        break
                if found_jp_dst and found_jp_dst != dst:
                    # But some are ok: e.g. destination=discard while text says 控え室に置く is fine
                    # and destination=hand while text says 手札に加える is fine
                    # Skip the obvious "move to X" where X matches
                    if dst == 'discard' and found_jp_dst == 'discard':
                        continue
                    if dst == 'hand' and found_jp_dst == 'hand':
                        continue
                    if dst == 'energy_zone' and found_jp_dst == 'energy_zone':
                        continue
                    results['1'].append((describe(ab), f"WRONG DEST: dst={dst} but text contains '{jp}'→{found_jp_dst} text=\"{text[:100]}\""))

# Check specific case: destination=null when it should have a value
for ab in abilities:
    for eff in collect_all_effects(ab.get('effect')):
        if eff.get('action') == 'move_cards':
            if eff.get('destination') is None:
                text = eff.get('text', '')
                if text:
                    results['1'].append((describe(ab), f"destination=null for move_cards text=\"{text[:100]}\""))

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
card_type_jp = {
    'energy_card': ['エネルギーカード', 'エネルギー'],
    'member_card': ['メンバーカード', 'メンバ�E'],
    'live_card': ['ライブカード', 'ライブ'],
    'card': ['カード'],
}
for ab in abilities:
    ft = ab.get('full_text', '')
    for eff in collect_all_effects(ab.get('effect')):
        ct = eff.get('card_type')
        text = eff.get('text', '')
        if not text or not ct:
            continue
        if ct == 'card':
            continue  # card is the generic fallback, OK
        # Check if text has energy keywords but card_type is not energy_card
        if 'エネルギーカード' in text and ct != 'energy_card' and 'エネルギーチE��キ' not in text:
            results['3'].append((describe(ab), f"card_type={ct} but text says エネルギーカード text=\"{text[:100]}\""))
        if 'メンバーカード' in text and ct not in ('member_card', 'card'):
            results['3'].append((describe(ab), f"card_type={ct} but text says メンバーカード text=\"{text[:100]}\""))
        if 'ライブカード' in text and ct not in ('live_card', 'card'):
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
    for eff in collect_all_effects(ab.get('effect')):
        text = eff.get('text', '')
        if not text:
            continue
        if 'すべての' in text or '全ての' in text:
            if not eff.get('all'):
                results['6'].append((describe(ab), f"text has すべての but all not set text=\"{text[:100]}\""))
        # Also check for "全部" or "すべて"
        if 'すべて' in text and 'すべての' not in text:
            pass  # might be "すべて" meaning "everything"
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
    for eff in collect_all_effects(ab.get('effect')):
        text = eff.get('text', '')
        if not text:
            continue
        if 'それぞれ' in text or 'ずつ' in text:
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
    if t in ('comparison_condition', 'card_count_condition'):
        if 'operator' not in cond or cond.get('operator') is None:
            results['8'].append((context, f"{t} missing operator field"))
    if t == 'compound':
        for sub in cond.get('conditions', []):
            check_condition(sub, context)
    if t == 'group_condition':
        if 'group' not in cond or cond.get('group') is None:
            results['8'].append((context, f"group_condition missing group field"))

for ab in abilities:
    desc = describe(ab)
    eff = ab.get('effect')
    if eff:
        check_condition(eff.get('condition'), desc)
        # Also check conditions inside sequential/choice/etc
        for sub_eff in collect_all_effects(eff):
            check_condition(sub_eff.get('condition'), desc)
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
    # Also check inside look_and_select select_action
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
        # Check top-level and nested conditions
        cond = eff.get('condition')
        if cond:
            # Recursive check
            def check_op(cond):
                if not cond:
                    return
                t = cond.get('type', '')
                if t in ('comparison_condition', 'card_count_condition') and 'operator' not in cond:
                    results['12'].append((desc, f"{t} missing operator field"))
                if t == 'compound':
                    for sub in cond.get('conditions', []):
                        check_op(sub)
            check_op(cond)
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

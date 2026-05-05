"""
Structural template analyzer for ability texts.

Instead of exact text matching, this clusters abilities by their
STRUCTURAL SHAPE — the pattern of features they share, regardless
of specific parameter values (card names, counts, zone names, etc).

The goal: find the minimum number of distinct "template shapes"
that cover the maximum number of abilities.
"""
import json
import re
from collections import defaultdict, Counter
from typing import Any, Dict, List, Tuple

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
print(f"Total unique abilities: {len(abilities)}")

# =========================================================================
# Step 1: Extract structural fingerprints
# =========================================================================
# A fingerprint captures the SHAPE of an ability, replacing specific
# values with type markers. For example:
#   "カードを{INT}枚引き、手札から{INT}枚を控え室に置く"
#   → "draw_{INT}_cards_then_discard_{INT}_from_hand"
# =========================================================================

def fingerprint_text(text: str) -> str:
    """Replace specific values with type markers."""
    # Replace specific counts
    text = re.sub(r'\d+枚', '{N}枚', text)
    text = re.sub(r'\d+人', '{N}人', text)
    text = re.sub(r'\d+つ', '{N}つ', text)
    text = re.sub(r'\d+回', '{N}回', text)
    text = re.sub(r'コスト(\d+)', 'コスト{V}', text)
    text = re.sub(r'スコアを([+＋]?\d+)', 'スコアを{V}', text)
    text = re.sub(r'[+＋]\d+', '+{V}', text)
    # Replace specific card names (「...」 patterns)
    text = re.sub(r'「[^」]+」', '{NAME}', text)
    # Replace specific group names (『...』 patterns)  
    text = re.sub(r'『[^』]+』', '{GROUP}', text)
    # Replace heart/blade icon references
    text = re.sub(r'{{heart_\d+\.png\|heart\d+}}', '{HEART}', text)
    text = re.sub(r'{{icon_blade\.png\|ブレード}}', '{BLADE}', text)
    text = re.sub(r'{{icon_energy\.png\|E}}', '{ENERGY}', text)
    text = re.sub(r'{{icon_all\.png\|ハート}}', '{ALLHEART}', text)
    return text.strip()

# =========================================================================
# Step 2: Identify structural markers
# =========================================================================

def get_structure_signature(text: str) -> Tuple[str, ...]:
    """Identify the high-level structure shape."""
    markers = []
    
    # Cost-effect structure
    if '：' in text:
        markers.append('COLON')
    
    # Condition markers
    for m in ['場合、', 'とき、', 'なら、']:
        if m in text:
            markers.append('COND')
            break
    
    # Sequential markers
    if 'その後、' in text:
        markers.append('SEQ_THEN')
    if 'さらに' in text:
        markers.append('SEQ_FURTHER')
    
    # Te-form sequential (comma-separated actions)
    if '、' in text and not any(m in text for m in ['場合、', 'とき、', 'なら、']):
        parts = text.split('、')
        if len(parts) >= 2:
            # Check if first part ends with continuative verb form
            first = parts[0].strip()
            if any(first.endswith(e) for e in ['き', 'ぎ', 'し', 'じ', 'ち', 'び', 'み', 'り', 'い', 'え']):
                markers.append('SEQ_TE')
    
    # Special structures
    if 'その中から' in text:
        markers.append('LOOK_SELECT')
    if 'につき' in text or 'ごとに' in text:
        markers.append('PER_UNIT')
    if 'かぎり' in text:
        markers.append('DURATION')
    if '以下から1つを選ぶ' in text:
        markers.append('CHOICE')
    if '代わりに' in text:
        markers.append('ALT')
    if 'たび' in text:
        markers.append('EACH_TIME')
    if '何もしない' in text:
        markers.append('DO_NOTHING')
    if 'そうした場合' in text:
        markers.append('SOU_SHITA')
    if '、' in text and 'し' in text and not markers:
        first_comma = text.split('、')[0].strip()
        if first_comma.endswith('し'):
            markers.append('SEQUENTIAL')
    if 'これにより' in text:
        markers.append('KORE_NIYORI')
    
    # Period-separated multi-sentence
    if '。' in text:
        sentences = [s.strip() for s in text.split('。') if s.strip()]
        if len(sentences) >= 2:
            markers.append('MULTI_SENT')
    
    return tuple(sorted(set(markers)))

# =========================================================================
# Step 3: Effect action patterns (based on parsed output)
# =========================================================================

def get_effect_shape(effect: Dict[str, Any]) -> str:
    """Classify an effect into a shape family."""
    if not effect or not isinstance(effect, dict):
        return "unknown"
    
    action = effect.get('action', '')
    if action == 'sequential':
        actions_list = effect.get('actions', [])
        sub_actions = [a.get('action', '') for a in actions_list if isinstance(a, dict)]
        if sub_actions:
            return 'seq[' + '+'.join(sub_actions) + ']'
        return 'sequential'
    
    if action == 'move_cards':
        src = effect.get('source', '?')
        dst = effect.get('destination', '?')
        ct = effect.get('card_type', '')
        tag = f"move_{src}→{dst}"
        if ct:
            tag += f"({ct})"
        return tag
    
    if action == 'gain_resource':
        res = effect.get('resource', '?')
        dur = 'temp' if effect.get('duration') else 'perm'
        tag = f"gain_{res}({dur})"
        if effect.get('group'):
            tag += "[group]"
        return tag
    
    if action == 'look_and_select':
        return 'look_and_select'
    
    if action == 'change_state':
        sc = effect.get('state_change', '?')
        return f"change_state({sc})"
    
    if action == 'modify_score':
        return 'modify_score'
    
    if action == 'draw_card':
        tgt = effect.get('target', 'self')
        return f'draw({tgt})'
    
    if action.startswith('custom'):
        return 'custom'
    
    return action

# =========================================================================
# Step 4: Cost shape
# =========================================================================

def get_cost_shape(cost: Dict[str, Any]) -> str:
    """Classify cost into a shape."""
    if not cost or not isinstance(cost, dict):
        return "no_cost"
    ct = cost.get('type', '')
    if ct == 'move_cards':
        src = cost.get('source', '?')
        dst = cost.get('destination', '?')
        return f"cost_move_{src}→{dst}"
    if ct == 'pay_energy':
        return "cost_energy"
    if ct == 'change_state':
        return "cost_change_state"
    if ct == 'sequential_cost':
        return "cost_sequential"
    if ct == 'reveal':
        return "cost_reveal"
    return f"cost_{ct}"

# =========================================================================
# Step 5: Build structural template families
# =========================================================================

# A template family = (trigger_type, structure_signature, cost_shape, effect_shape)
template_families = defaultdict(list)

for ab in abilities:
    trigger = ab.get('triggers', '') or ''
    text = ab.get('triggerless_text', '')
    fp = fingerprint_text(text)
    sig = get_structure_signature(text)
    
    effect = ab.get('effect', {})
    cost = ab.get('cost', {})
    
    eff_shape = get_effect_shape(effect)
    cost_shape = get_cost_shape(cost)
    
    # Create template family key
    family_key = (trigger, sig, eff_shape, cost_shape)
    template_families[family_key].append({
        'text': text[:80],
        'fp': fp[:100],
    })

print(f"\nTotal structure families: {len(template_families)}")

# Sort by frequency
sorted_families = sorted(template_families.items(), key=lambda x: -len(x[1]))

print(f"\nTop 30 most common families:")
print(f"{'#':>3} {'Count':>5}  {'Trigger':<20} {'Structure':<40} {'Effect':<30} {'Cost':<20}")
print("-"*120)
for i, ((trigger, sig, eff_shape, cost_shape), items) in enumerate(sorted_families[:30]):
    print(f"{i+1:>3} {len(items):>5}  {trigger:<20} {str(sig):<40} {eff_shape:<30} {cost_shape:<20}")

print(f"\n--- Coverage analysis ---")
cumulative = 0
for i, (_, items) in enumerate(sorted_families, 1):
    cumulative += len(items)
    if cumulative >= len(abilities) // 2:
        print(f"{i} families cover 50% ({cumulative}/{len(abilities)})")
        break
cumulative = 0
for i, (_, items) in enumerate(sorted_families, 1):
    cumulative += len(items)
    if cumulative >= len(abilities) * 8 // 10:
        print(f"{i} families cover 80% ({cumulative}/{len(abilities)})")
        break
cumulative = 0
for i, (_, items) in enumerate(sorted_families, 1):
    cumulative += len(items)
    if cumulative >= len(abilities) * 9 // 10:
        print(f"{i} families cover 90% ({cumulative}/{len(abilities)})")
        break
cumulative = 0
for i, (_, items) in enumerate(sorted_families, 1):
    cumulative += len(items)
    if cumulative >= len(abilities):
        break
print(f"{i} families cover 100% ({cumulative}/{len(abilities)})")

# Singleton families
singletons = sum(1 for v in template_families.values() if len(v) == 1)
print(f"\nSingleton families (1 ability only): {singletons}")
print(f"Non-singleton families: {len(template_families) - singletons}")

# =========================================================================
# Step 6: Collapse by effect shape only (ignoring trigger and cost)
# =========================================================================

print(f"\n--- Collapsed by effect shape only ---")
eff_families = defaultdict(list)
for ab in abilities:
    eff = get_effect_shape(ab.get('effect', {}))
    eff_families[eff].append(ab.get('triggerless_text', '')[:60])

sorted_eff = sorted(eff_families.items(), key=lambda x: -len(x[1]))
print(f"Distinct effect shapes: {len(eff_families)}")
for eff, items in sorted_eff[:15]:
    print(f"  {eff:40s} x{len(items):3d}  e.g. {items[0]}")

# =========================================================================
# Step 7: Collapse by structure signature only (no trigger/cost/effect)
# =========================================================================

print(f"\n--- Collapsed by structure signature only ---")
sig_families = defaultdict(int)
for ab in abilities:
    text = ab.get('triggerless_text', '')
    sig = get_structure_signature(text)
    sig_families[sig] += 1

sorted_sigs = sorted(sig_families.items(), key=lambda x: -x[1])
print(f"Distinct structure signatures: {len(sig_families)}")
for sig, count in sorted_sigs[:15]:
    print(f"  {str(sig):50s} x{count:3d}")

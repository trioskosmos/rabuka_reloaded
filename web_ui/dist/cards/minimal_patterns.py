"""
Find the MINIMUM number of core patterns in 602 abilities.

The question: if you group by WHAT the ability fundamentally DOES
(not exact text, not trigger type, not specific zone names),
how many distinct patterns exist?
"""
import json
import re
from collections import defaultdict, Counter

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

# =====================================================================
# Level 1: What structure does the text have?
# =====================================================================
# A text is fundamentally one of:
#   cost_action  — "do X to pay: then do Y"
#   conditional  — "if X, then do Y"  
#   sequential   — "do A, then do B" (te-form or further)
#   choice       — "pick one of these"
#   duration     — "as long as X, do Y"
#   per_unit     — "for each X, do Y"
#   simple       — just "do Y"
#   look_select  — "look at X cards, pick one"

def core_structure(text):
    if '：' in text and not '場合' in text:
        return 'cost_action'
    if any(m in text for m in ['場合、', 'とき、', 'なら、']):
        return 'conditional'
    if '以下から1つを選ぶ' in text:
        return 'choice'
    if 'その中から' in text:
        return 'look_select'
    if 'かぎり' in text:
        return 'duration'
    if 'につき' in text or 'ごとに' in text:
        return 'per_unit'
    if '代わりに' in text:
        return 'conditional_alt'
    if 'さらに' in text:
        return 'sequential'
    if 'その後' in text:
        return 'sequential'
    if '。' in text:
        parts = [p.strip() for p in text.split('。') if p.strip()]
        if len(parts) >= 2:
            return 'sequential'
    if '、' in text:
        first = text.split('、')[0].strip()
        if any(first.endswith(e) for e in ['き','ぎ','し','じ','ち','び','み','り','い','え']):
            return 'sequential'
    return 'simple'

# =====================================================================
# Level 2: What is the EFFECT's action type?
# =====================================================================
# This comes from the parsed effect.action field

def effect_type(ab):
    effect = ab.get('effect', {})
    if not isinstance(effect, dict):
        return 'unknown'
    action = effect.get('action', '')
    if action == 'sequential':
        subs = effect.get('actions', [])
        sub_actions = '+'.join(sorted(set(a.get('action','') for a in subs if isinstance(a, dict))))
        if sub_actions:
            return f'seq({sub_actions})'
        return 'sequential'
    if action == 'move_cards':
        src = effect.get('source', '?')
        dst = effect.get('destination', '?')
        return f'move({src}->{dst})'
    if action == 'gain_resource':
        res = effect.get('resource', '?')
        dur = 'temp' if effect.get('duration') else 'perm'
        return f'gain_{res}({dur})'
    if action == 'draw_card':
        tgt = effect.get('target', 'self')
        return f'draw({tgt})'
    return action if action else 'unknown'

# =====================================================================
# Level 3: What TRIGGER fires it?
# =====================================================================

def trigger_type(ab):
    t = ab.get('triggers', '') or ''
    if '登場' in t: return 'debut'
    if 'ライブ開始' in t: return 'live_start'
    if '起動' in t: return 'activate'
    if 'ライブ成功' in t: return 'live_success'
    if '常時' in t: return 'constant'
    return 'none'

# =====================================================================
# Analyze core patterns (structure + effect)
# =====================================================================

core_patterns = defaultdict(list)

for ab in abilities:
    text = ab.get('triggerless_text', '')
    cs = core_structure(text)
    et = effect_type(ab)
    tt = trigger_type(ab)
    key = (cs, et)
    core_patterns[key].append((tt, text[:50]))

print("=== CORE PATTERNS (structure + effect_type) ===")
print(f"Total distinct core patterns: {len(core_patterns)}")
print()

sorted_core = sorted(core_patterns.items(), key=lambda x: -len(x[1]))
print(f"{'#':>3} {'Count':>5}  {'Structure':<18} {'Effect':<28} {'Example trigger'}")
print("-" * 80)
for i, ((cs, et), items) in enumerate(sorted_core):
    example_triggers = list(set(it[0] for it in items))
    print(f"{i+1:>3} {len(items):>5}  {cs:<18} {et:<28} {example_triggers[0]}")

print()

# Coverage
cum = 0
for i, (_, items) in enumerate(sorted_core, 1):
    cum += len(items)
    if cum >= 602 * 0.5:
        print(f"{i} patterns covers 50%")
        break
cum = 0
for i, (_, items) in enumerate(sorted_core, 1):
    cum += len(items)
    if cum >= 602 * 0.8:
        print(f"{i} patterns covers 80%")
        break  
cum = 0
for i, (_, items) in enumerate(sorted_core, 1):
    cum += len(items)
    if cum >= 602 * 0.9:
        print(f"{i} patterns covers 90%")
        break

print()

# =====================================================================
# Collapse further: just the CORE STRUCTURE (ignore effect details)
# =====================================================================

print("=== COLLAPSED BY CORE STRUCTURE ONLY ===")
structure_counts = Counter()
for ab in abilities:
    text = ab.get('triggerless_text', '')
    cs = core_structure(text)
    structure_counts[cs] += 1

for cs, count in structure_counts.most_common():
    print(f"  {cs:<20} x{count:4d}")

print()

# =====================================================================
# Show examples for each core structure
# =====================================================================
print("=== EXAMPLES PER CORE STRUCTURE ===")
seen_cs = set()
for ab in abilities:
    text = ab.get('triggerless_text', '')
    cs = core_structure(text)
    if cs not in seen_cs:
        # Find what abilities.json shows as the parsed action for this
        effect = ab.get('effect', {})
        action = effect.get('action', '?') if isinstance(effect, dict) else '?'
        print(f"\n  {cs} (action={action}):")
        print(f"    {text[:100]}")
        seen_cs.add(cs)

# =====================================================================
# How many patterns if we collapse triggers?
# =====================================================================
print()
print("=== WITHOUT TRIGGER ===")
no_trigger = defaultdict(int)
for ab in abilities:
    text = ab.get('triggerless_text', '')
    cs = core_structure(text)
    et = effect_type(ab)
    no_trigger[(cs, et)] += 1

print(f"Patterns without trigger: {len(no_trigger)}")
cum = 0
for i, (_, count) in enumerate(sorted(no_trigger.items(), key=lambda x: -x[1]), 1):
    cum += count
    if cum >= 602 * 0.8:
        print(f"  {i} patterns cover 80%")
        break

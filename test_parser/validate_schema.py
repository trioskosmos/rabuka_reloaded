"""Validate the schema-driven parser against all 602 real abilities."""

import json, os, sys, re, time
sys.path.insert(0, os.path.dirname(__file__))

from schema import extract_all, ACTION_FIELD_SIGNATURES, FIELDS
from infer import infer_action, parse_action

ABILITIES_PATH = os.path.join(os.path.dirname(__file__), '..', 'cards', 'abilities.json')
with open(ABILITIES_PATH, encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
print(f"Loaded {len(abilities)} unique abilities")

# ------------------------------------------------------------------
# Helper: extract effect text for parsing
# ------------------------------------------------------------------

def extract_text(ab):
    t = (ab.get('triggerless_text') or ab.get('full_text', ''))
    t = re.sub(r'\{\{[^}]+?\}\}', '', t).strip()
    if '：' in t:
        t = t.split('：', 1)[1].strip()
    t = t.rstrip('。')
    return t

def ref_action(ab):
    eff = ab.get('effect')
    return eff.get('action', '?') if eff else '(none)'

# ------------------------------------------------------------------
# Phase 1: Action type distribution
# ------------------------------------------------------------------

print("\n--- Phase 1: Action type distribution ---")
old_stats = {}
new_stats = {}
custom_count = 0
unmatched = []

for i, ab in enumerate(abilities):
    old = ref_action(ab)
    old_stats[old] = old_stats.get(old, 0) + 1

    text = extract_text(ab)
    try:
        state = parse_action(text)
        new_action = state.get('action', 'custom')
    except Exception as e:
        new_action = 'ERROR'
    
    new_stats[new_action] = new_stats.get(new_action, 0) + 1
    if new_action == 'custom':
        custom_count += 1
        unmatched.append((i, text[:50]))

all_actions = sorted(set(list(old_stats.keys()) + list(new_stats.keys())))
print(f"{'Action':30s} {'Old':>6s} {'New':>6s}")
print("-" * 45)
for a in all_actions:
    o = old_stats.get(a, 0)
    n = new_stats.get(a, 0)
    print(f"{a:30s} {o:6d} {n:6d}")

print(f"\nUnmatched (custom): {custom_count}/{len(abilities)} ({100*custom_count/len(abilities):.1f}%)")
if unmatched:
    print(f"First 10:")
    for idx, txt in unmatched[:10]:
        print(f"  [{idx}] {txt}")

# ------------------------------------------------------------------
# Phase 2: Field extraction coverage
# ------------------------------------------------------------------

print("\n\n--- Phase 2: Field extraction coverage ---")
field_counts = {}
for ab in abilities:
    text = extract_text(ab)
    fields = extract_all(text)
    for fname in fields:
        field_counts[fname] = field_counts.get(fname, 0) + 1

print("Fields extracted (sorted by frequency):")
for fname in sorted(field_counts, key=lambda f: -field_counts[f]):
    print(f"  {fname:25s} {field_counts[fname]:6d}/{len(abilities)}")

# Which fields were NEVER extracted?
all_field_names = [f.name for f in FIELDS]
never_extracted = [f for f in all_field_names if f not in field_counts]
if never_extracted:
    print(f"\nNever extracted: {never_extracted}")

# ------------------------------------------------------------------
# Phase 3: Performance
# ------------------------------------------------------------------

print("\n\n--- Phase 3: Performance ---")
sample = abilities[:500]
start = time.perf_counter()
for ab in sample:
    text = extract_text(ab)
    parse_action(text)
elapsed = time.perf_counter() - start
print(f"500 texts in {elapsed*1000:.1f}ms ({elapsed/500*1000:.3f}ms each)")

# ------------------------------------------------------------------
# Phase 4: Field extraction accuracy spot-check
# ------------------------------------------------------------------

print("\n\n--- Phase 4: Spot check key abilities ---")
tests = [
    ("discard-recover",
     "このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。",
     {'source': 'discard', 'destination': 'hand', 'count': 1, 'action': 'move_cards'}),
    ("draw",
     "カードを2枚引く",
     {'action': 'draw_card', 'source': 'deck', 'destination': 'hand', 'count': 2}),
    ("change_state wait",
     "相手のステージにいるコスト4以下のメンバー1人をウェイトにする",
     {'action': 'change_state', 'state_change': 'wait', 'count': 1}),
    ("gain_resource blade",
     "{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
     {'action': 'gain_resource', 'resource': 'blade', 'count': 2}),
    ("pay_energy",
     "{{icon_energy.png|E}}支払ってもよい",
     {'action': 'pay_energy', 'count': 1, 'optional': True}),
]

passed = 0
for name, text, expected in tests:
    state = parse_action(extract_text({'triggerless_text': text}))
    for k, v in expected.items():
        if state.get(k) != v:
            print(f"  FAIL {name}: {k}={state.get(k)!r}, expected {v!r}")
            break
    else:
        passed += 1
        print(f"  OK {name}")

print(f"\nSpot checks: {passed}/{len(tests)} passed")

# ------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------

print(f"\n{'='*60}")
print(f"SCHEMA-DRIVEN PARSER SUMMARY")
print(f"{'='*60}")
print(f"Total abilities:         {len(abilities)}")
print(f"Fields defined:          {len(FIELDS)}")
print(f"Action signatures:       {len(ACTION_FIELD_SIGNATURES)}")
print(f"Unmatched (custom):      {custom_count}/{len(abilities)} ({100*custom_count/len(abilities):.1f}%)")
print(f"Field extraction hits:   {len(field_counts)}/{len(FIELDS)} fields found in data")
print(f"Parse time:              {elapsed*1000:.1f}ms for 500 texts")

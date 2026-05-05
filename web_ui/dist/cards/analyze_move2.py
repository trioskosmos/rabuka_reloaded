"""
Deep dive: how many distinct move_cards CAPABILITIES exist?
Not just source→dest pairs, but the full semantic shape.

A move_cards capability is: (source, dest, card_type, count_mode, optional?, target?, group_filter?, cost_limit?, state_change?, placement?)
"""
import json
from collections import defaultdict

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

# Collect ALL effects
def walk_effects(eff, collector, path=""):
    if not isinstance(eff, dict):
        return
    a = eff.get('action', '')
    if a == 'move_cards':
        collector.append(eff)
    elif a == 'sequential':
        for i, sub in enumerate(eff.get('actions', [])):
            walk_effects(sub, collector, f"{path}[{i}]")
    elif a == 'look_and_select':
        for k in ['select_action', 'look_action']:
            walk_effects(eff.get(k, {}), collector, f"{path}.{k}")

all_moves = []
for ab in abilities:
    walk_effects(ab.get('effect', {}), all_moves)

def move_key(m):
    """Semantic key for a move_cards action."""
    count = 'dynamic' if 'dynamic_count' in m else ('fixed' if 'count' in m else 'unknown')
    group = m.get('group', {})
    group_name = group.get('name', '') if isinstance(group, dict) else str(group)
    gn = m.get('group_names')
    gn_str = str(gn) if gn else ''
    po = m.get('placement_order', '?')
    po_str = str(po) if not isinstance(po, str) else po
    pos = m.get('position', '?')
    pos_str = str(pos) if not isinstance(pos, str) else pos
    cl = 'custom' if m.get('cost_limit') else 'no'
    return (
        str(m.get('source', '?')),
        str(m.get('destination', '?')),
        str(m.get('card_type', '?')),
        count,
        bool(m.get('optional')),
        str(m.get('target', '?')),
        group_name or gn_str or 'no',
        str(m.get('state_change', '?')),
        cl,
        po_str,
        'all' if m.get('all') else 'no',
        pos_str,
    )

# Group by semantic key
groups = defaultdict(list)
for m in all_moves:
    groups[move_key(m)].append(m)

print(f"Total move_cards occurrences: {len(all_moves)}")
print(f"Distinct semantic signatures: {len(groups)}")

# Show most common ones
sorted_g = sorted(groups.items(), key=lambda x: -len(x[1]))
print("\n=== TOP 30 move_cards SEMANTIC SHAPES ===")
print(f"{'#':>3} {'Count':>4}  Shape")
print("-"*100)
for i, (key, items) in enumerate(sorted_g[:30]):
    s, d, ct, cnt, opt, tgt, grp, sc, cl, po, al, pos = key
    opt_s = 'opt' if opt else ''
    grp_s = f'[grp]' if grp == 'yes' else ''
    tgt_s = f'<{tgt}>' if tgt != '?' else ''
    sc_s = f'+{sc}' if sc != '?' else ''
    cl_s = f'$lim' if cl != 'no' else ''
    po_s = f'_{po}' if po != '?' else ''
    al_s = '_ALL' if al == 'yes' else ''
    pos_s = f'@{pos}' if pos != '?' else ''
    print(f"{i+1:>3} {len(items):>4}  {s:>20}-{d:<20} [{ct:<15}] {cnt} {opt_s}{grp_s}{tgt_s}{sc_s}{cl_s}{po_s}{al_s}{pos_s}")

# Collapse to see how many just source→dest→card_type
print("\n=== COLLAPSED: source→dest→card_type ===")
basic = defaultdict(int)
for m in all_moves:
    key = (m.get('source','?'), m.get('destination','?'), m.get('card_type','?'))
    basic[key] += 1
for (s,d,ct), c in sorted(basic.items(), key=lambda x: -x[1]):
    s2 = s if s else '?'
    d2 = d if d else '?'
    ct2 = ct if ct else '?'
    print(f"  {s2:>20} -> {d2:<20} [{ct2:<15}] x{c:>3}")
print(f"Total: {len(basic)}")

# What about: just source→dest (no card_type)?
print("\n=== COLLAPSED: source→dest (all card_types combined) ===")
pair = defaultdict(int)
for m in all_moves:
    key = (m.get('source','?'), m.get('destination','?'))
    pair[key] += 1
for (s,d), c in sorted(pair.items(), key=lambda x: -x[1]):
    print(f"  {s:>20} -> {d:<20} x{c:>3}")
print(f"Total: {len(pair)}")

# Show rare ones (count=1)
rare = [(k,v) for k,v in groups.items() if len(v)==1]
print(f"\n=== SINGLETON move_cards patterns: {len(rare)} ===")
for key, items in rare:
    print(f"  {move_key(items[0])}")

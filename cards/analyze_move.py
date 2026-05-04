"""
Analyze ALL move_cards actions across all 602 abilities.
What source/destination pairs exist?
What subfields are used (source, destination, count, card_type, optional, state_change, etc)?
How many unique parameter signatures exist?
"""
import json
from collections import defaultdict

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']

# Collect every effect and cost that is a move_cards
move_cards_effects = []
move_cards_costs = []

for ab in abilities:
    # Check cost
    cost = ab.get('cost', {})
    if isinstance(cost, dict) and cost.get('type') == 'move_cards':
        move_cards_costs.append(cost)
    
    # Check effect (including nested in sequential)
    effect = ab.get('effect', {})
    if not isinstance(effect, dict):
        continue
    
    def extract_moves(eff, path=""):
        if not isinstance(eff, dict):
            return
        a = eff.get('action', '')
        if a == 'move_cards':
            move_cards_effects.append(eff)
        elif a == 'sequential':
            for i, sub in enumerate(eff.get('actions', [])):
                extract_moves(sub, f"{path}[{i}]")
        elif a == 'look_and_select':
            sa = eff.get('select_action', {})
            if isinstance(sa, dict):
                extract_moves(sa, f"{path}.select")
    
    extract_moves(effect)

print(f"Total move_cards effects: {len(move_cards_effects)}")
print(f"Total move_cards costs: {len(move_cards_costs)}")

print("\n=== EFFECT move_cards: source→destination pairs ===")
eff_pairs = defaultdict(int)
for m in move_cards_effects:
    key = f"{m.get('source','?')} -> {m.get('destination','?')}"
    eff_pairs[key] += 1
for pair, c in sorted(eff_pairs.items(), key=lambda x: -x[1]):
    print(f"  {pair:<30} x{c}")

print("\n=== COST move_cards: source→destination pairs ===")
cost_pairs = defaultdict(int)
for m in move_cards_costs:
    key = f"{m.get('source','?')} -> {m.get('destination','?')}"
    cost_pairs[key] += 1
for pair, c in sorted(cost_pairs.items(), key=lambda x: -x[1]):
    print(f"  {pair:<30} x{c}")

print("\n=== EFFECT move_cards: ALL parameter fields ===")
field_counts_eff = defaultdict(int)
for m in move_cards_effects:
    for k in m:
        if k != 'text':
            field_counts_eff[k] += 1
for f, c in sorted(field_counts_eff.items(), key=lambda x: -x[1]):
    print(f"  {f:<20} x{c}")

print("\n=== COST move_cards: ALL parameter fields ===")
field_counts_cost = defaultdict(int)
for m in move_cards_costs:
    for k in m:
        if k != 'text':
            field_counts_cost[k] += 1
for f, c in sorted(field_counts_cost.items(), key=lambda x: -x[1]):
    print(f"  {f:<20} x{c}")

# =====================================================================
# Unique parameter signatures (which fields appear together)
# =====================================================================
def signature(m):
    """A signature of which parameter fields are present."""
    fields = set()
    for k in ['source', 'destination', 'count', 'card_type', 'optional',
              'state_change', 'self_target', 'self_cost', 'placement_order',
              'shuffle', 'exclude_self', 'cost_limit', 'all', 'position',
              'target']:
        if k in m:
            fields.add(k)
    return tuple(sorted(fields))

eff_sigs = defaultdict(int)
for m in move_cards_effects:
    eff_sigs[signature(m)] += 1
print(f"\n=== EFFECT: Unique field signatures ===")
for sig, c in sorted(eff_sigs.items(), key=lambda x: -x[1]):
    print(f"  {str(sig):<50} x{c}")

cost_sigs = defaultdict(int)
for m in move_cards_costs:
    cost_sigs[signature(m)] += 1
print(f"\n=== COST: Unique field signatures ===")
for sig, c in sorted(cost_sigs.items(), key=lambda x: -x[1]):
    print(f"  {str(sig):<50} x{c}")

# =====================================================================
# Show all unique source→destination→card_type combinations (effects)
# =====================================================================
print("\n=== EFFECT move_cards: unique (source, dest, card_type) combos ===")
sdct = defaultdict(int)
for m in move_cards_effects:
    key = (m.get('source','?'), m.get('destination','?'), m.get('card_type',''))
    sdct[key] += 1
for (s,d,ct), c in sorted(sdct.items(), key=lambda x: -x[1]):
    print(f"  {s:<15} -> {d:<20} ({ct:<15}) x{c}")

print("\n=== COST move_cards: unique (source, dest, card_type) combos ===")
sdct_cost = defaultdict(int)
for m in move_cards_costs:
    key = (m.get('source','?'), m.get('destination','?'), m.get('card_type',''))
    sdct_cost[key] += 1
for (s,d,ct), c in sorted(sdct_cost.items(), key=lambda x: -x[1]):
    print(f"  {s:<15} -> {d:<20} ({ct:<15}) x{c}")

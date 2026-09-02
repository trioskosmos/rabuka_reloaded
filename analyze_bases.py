import json
data = json.load(open('cards/cards.json', encoding='utf-8'))

# Analyze the structure of card numbers
bases = set()
for k in data:
    cn = data[k].get('card_no', '')
    parts = cn.split('-')
    if len(parts) >= 4:
        # Last part is rarity, rest is base
        base = '-'.join(parts[:-1])
        rarity = parts[-1]
        bases.add(base)
    else:
        print(f'SHORT: {cn}')

print(f'Total unique bases: {len(bases)}')

# Check how many cards per base
from collections import defaultdict
cards_per_base = defaultdict(list)
for k in data:
    cn = data[k].get('card_no', '')
    parts = cn.split('-')
    if len(parts) >= 4:
        base = '-'.join(parts[:-1])
        rarity = parts[-1]
        cards_per_base[base].append((cn, data[k].get('rare')))

print(f'Total bases: {len(cards_per_base)}')
# Find bases with multiple rarities
multi = {b: v for b, v in cards_per_base.items() if len(v) > 1}
print(f'Bases with multiple rarities: {len(multi)}')
for b, v in sorted(multi.items())[:10]:
    print(f'{b}:')
    for cn, rare in v:
        print(f'  {cn} -> {rare}')
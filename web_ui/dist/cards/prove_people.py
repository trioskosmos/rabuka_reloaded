"""Find all conditions that use '人' (people counter) and show their locations."""
import json

data = json.load(open('cards/abilities.json', encoding='utf-8'))

results = []
for ab in data['unique_abilities']:
    effect = ab.get('effect', {})
    if not isinstance(effect, dict):
        continue
    cond = effect.get('condition', {})
    if not isinstance(cond, dict):
        continue
    ctext = cond.get('text', '')
    if '人' not in ctext:
        continue
    loc = cond.get('location', '')
    ct = cond.get('card_type', '')
    ctype = cond.get('type', '')
    results.append({
        'type': ctype, 'location': loc, 'card_type': ct,
        'text': ctext[:60]
    })

# Also check nested conditions (compound sub-conditions)
for ab in data['unique_abilities']:
    effect = ab.get('effect', {})
    if not isinstance(effect, dict):
        continue
    cond = effect.get('condition', {})
    if not isinstance(cond, dict):
        continue
    if cond.get('type') == 'compound':
        for sub in cond.get('conditions', []):
            if isinstance(sub, dict) and '人' in sub.get('text', ''):
                loc = sub.get('location', '')
                ct = sub.get('card_type', '')
                ctype = sub.get('type', '')
                results.append({
                    'type': ctype, 'location': loc, 'card_type': ct,
                    'text': sub.get('text', '')[:60]
                })

print(f"Total conditions with '人': {len(results)}")

has_loc = sum(1 for r in results if r['location'])
no_loc = sum(1 for r in results if not r['location'])
print(f"With location: {has_loc}")
print(f"Without location: {no_loc}")

for r in results:
    loc_str = r['location'] if r['location'] else '(empty)'
    ct_str = r['card_type'] if r['card_type'] else '(empty)'
    print(f"  type={r['type']:<25} loc={loc_str:<15} ct={ct_str:<15} text={r['text']}")

print()

# Verify: does ANY '人' condition refer to a non-stage location?
non_stage = [r for r in results if r['location'] and r['location'] != 'stage']
if non_stage:
    print(f"NON-STAGE locations with '人': {len(non_stage)}")
    for r in non_stage:
        print(f"  loc={r['location']} text={r['text']}")
else:
    print("ALL '人' conditions with a location refer to 'stage'. Zero counterexamples.")

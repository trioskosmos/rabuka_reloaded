"""Fix remaining cross-reference issues."""
import json, sys
sys.path.insert(0, '.')

data = json.load(open('cards/abilities.json', encoding='utf-8'))

print('=== COND_NO_LOC entries ===')
for i, entry in enumerate(data['unique_abilities']):
    t = entry.get('triggerless_text', '')
    eff = entry.get('effect')
    if not eff: continue
    cond = eff.get('condition', {})
    if cond.get('type') == 'location_condition' and 'location' not in cond:
        print('Entry #' + str(i) + ': ' + t[:60])
        print('  condition: ' + json.dumps(cond, ensure_ascii=False)[:200])
        print()

print()
print('=== otherwise_condition entries ===')
for i, entry in enumerate(data['unique_abilities']):
    eff = entry.get('effect', {})
    # Search recursively for otherwise_condition
    def find_otherwise(obj, depth=0):
        if not isinstance(obj, dict):
            return None
        if obj.get('type') == 'otherwise_condition':
            return obj
        for k, v in obj.items():
            if isinstance(v, dict):
                r = find_otherwise(v, depth+1)
                if r: return r
            elif isinstance(v, list):
                for item in v:
                    if isinstance(item, dict):
                        r = find_otherwise(item, depth+1)
                        if r: return r
        return None
    result = find_otherwise(eff)
    if result:
        t = entry.get('triggerless_text', '')
        print('Entry #' + str(i) + ': ' + t[:70])
        print('  result: ' + json.dumps(result, ensure_ascii=False)[:200])
        print()

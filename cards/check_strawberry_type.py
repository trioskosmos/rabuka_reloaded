"""Check the EXACT condition_type for Strawberry Trapper's group sub-condition."""
import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
entry = d['unique_abilities'][436]
cond = entry['effect']['condition']['conditions'][0]
print("condition_type:", cond.get('type'))
print("full:", json.dumps(cond, indent=2, ensure_ascii=False))

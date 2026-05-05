"""Check group_names and group fields in Strawberry Trapper's condition."""
import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
entry = d['unique_abilities'][436]
cond = entry['effect']['condition']['conditions'][0]
print("Sub-condition A (group):")
print(json.dumps(cond, indent=2, ensure_ascii=False))

"""Check current abilities.json for SUNNY DAY SONG"""
import json

with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

ua = data.get('unique_abilities', [])
entry = ua[523]
effect = entry.get('effect', {})
print(json.dumps(effect, indent=2, ensure_ascii=False))

import json

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

ua = data.get('unique_abilities', [])
entry = ua[523]
print("full_text:", repr(entry.get('full_text', '')[:200]))
print()
print("triggerless_text:", repr(entry.get('triggerless_text', '')[:200]))

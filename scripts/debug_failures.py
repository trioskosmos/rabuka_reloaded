import json
import os

path = "test_parser/real_abilities.json"
with open(path, "r", encoding="utf-8") as f:
    data = json.load(f)

unknown = [a for a in data['abilities'] if any('parse_failed' in e.get('text', '') for e in a['parsed']['effects'])]
print(f"Found {len(unknown)} unknown abilities")

for u in unknown[:20]:
    print("-" * 40)
    print(f"TEXT: {u['full_text']}")
    print(f"ERR : {u['parsed']['effects'][0]['text']}")

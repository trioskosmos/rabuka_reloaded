import json
ABILITIES_FILE = 'cards/abilities.json'
with open(ABILITIES_FILE, encoding='utf-8') as f:
    data = json.load(f)
abilities = data['unique_abilities']
key_entries = {'choudo': 145, 'kagiri': 199, 'goukei': 216}
for name, idx in key_entries.items():
    a = abilities[idx]
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    eff = a.get('effect', {})
    if isinstance(eff, dict):
        print(name, idx, t[:60])
        print(json.dumps(eff, ensure_ascii=False, indent=2)[:200])

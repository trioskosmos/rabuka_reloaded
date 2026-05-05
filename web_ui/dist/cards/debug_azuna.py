"""Debug the AZUNA card parser output."""
import json
import re

d = json.load(open('cards/abilities.json', encoding='utf-8'))

def normalize(text):
    text = re.sub(r"'([^']{1,10})'", r'『\1』', text)
    text = text.replace('ライブ終了まで', 'ライブ終了時まで')
    return text

# Find the AZUNA card entries
targets = ['PL!N-bp3-027', 'PL!S-pb1-021']
for i, entry in enumerate(d['unique_abilities']):
    for c in entry.get('cards', []):
        for t in targets:
            if t in c:
                tt = entry.get('triggerless_text', '')
                nt = normalize(tt)
                action = entry.get('effect', {}).get('action', '?')
                print(f"Entry {i} ({t}):")
                print(f"  text: {tt[:80]}")
                print(f"  normalized: {nt[:80]}")
                print(f"  same: {tt == nt}")
                print(f"  has 'そうした場合': {'そうした場合' in tt}")
                print(f"  action: {action}")
                print()
                break

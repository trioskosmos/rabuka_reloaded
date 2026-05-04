import json
d = json.load(open('cards/abilities.json', encoding='utf-8'))
for i, entry in enumerate(d['unique_abilities']):
    for c in entry.get('cards', []):
        if 'PL!N-bp3-027' in c:
            tt = entry.get('triggerless_text', '')
            print(f'Entry {i}:')
            print(f'  has sou_shita: {"そうした場合" in tt}')
            print(f'  action: {entry.get("effect", {}).get("action")}')
            print(f'  text: {tt}')
            print()

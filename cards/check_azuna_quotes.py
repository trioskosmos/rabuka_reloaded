import json
data = json.load(open('cards/abilities.json', encoding='utf-8'))
for ab in data['unique_abilities']:
    for c in ab.get('cards', []):
        if 'PL!N-bp3-027' in c:
            tt = ab.get('triggerless_text','')
            print('Has ASCII quote:', repr("'" in tt))
            print('Has curly quotes:', repr('\u2018' in tt or '\u2019' in tt))
            # Show full triggerless text
            print('Full text:', repr(tt)[:200])

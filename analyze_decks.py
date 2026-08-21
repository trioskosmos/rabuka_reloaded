import json, glob, re, os
cards = json.load(open('cards/cards.json', encoding='utf-8'))
by_no = {k: v for k, v in cards.items()}
for f in glob.glob('web_ui/decks/*.txt'):
    txt = open(f, encoding='utf-8', errors='replace').read()
    nos = re.findall(r'\b[A-Z]{2,3}!\S+', txt)
    lives = []
    for no in set(nos):
        c = by_no.get(no)
        if c and c.get('type') == 'ライブ':
            lives.append(c.get('score'))
    if lives:
        cheap = sum(1 for s in lives if s and s <= 2)
        print(os.path.basename(f), '| lives:', len(lives), '| cheap(<=2):', cheap,
              '| scores:', sorted(x for x in lives if x is not None))

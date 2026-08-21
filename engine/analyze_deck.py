# -*- coding: utf-8 -*-
import json, sys, re
from collections import Counter

cards = json.load(open('../cards/cards.json'))
by_no = {no.lower(): c for no, c in cards.items()}
deck_path = sys.argv[1] if len(sys.argv) > 1 else '../web_ui/decks/5CP3Z idou.txt'
deck = open(deck_path).read()

producers = Counter()
lives = []
members = []
for line in deck.splitlines():
    m = re.match(r'\s*(?:(\d+)\s*x\s+)?(\S+)', line)
    if not m:
        continue
    cnt = int(m.group(1)) if m.group(1) else 1
    no = m.group(2)
    c = by_no.get(no.lower())
    if not c:
        continue
    t = c.get('type', '')
    if t == u'メンバー':
        members.append((cnt, no, c))
        for col, v in (c.get('base_heart') or {}).items():
            producers[col] += cnt * v
    elif t == u'ライブ':
        lives.append((cnt, no, c))

print('members:', sum(x[0] for x in members), 'lives:', sum(x[0] for x in lives))
print('producers:', dict(producers))
print()
for cnt, no, c in lives:
    nh = c.get('need_heart') or {}
    ok = all(producers.get(col, 0) >= 5 * v for col, v in nh.items())
    print(no, 'x%d' % cnt, 'score=%s' % c.get('score'), 'needs=%s' % nh, 'supported5=%s' % ok)

costs = Counter()
for cnt, no, c in members:
    costs[c.get('cost')] += cnt
print()
print('cost curve:', dict(sorted(costs.items(), key=lambda x: (x[0] is None, x[0]))))

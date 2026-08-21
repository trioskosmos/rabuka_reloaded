# -*- coding: utf-8 -*-
import csv
from collections import defaultdict

rows = list(csv.reader(open('../test_output/bot_arena_trace.csv')))
snaps = [r for r in rows if len(r) > 2 and r[2] == 'ENTER']

def kv(r):
    d = {}
    for cell in r[3:]:
        if '=' in cell:
            k, v = cell.split('=', 1)
            d[k] = v
    return d

games = defaultdict(list)
for r in snaps:
    games[r[0]].append((int(r[1]), kv(r)))

picked = None
for g, rs in sorted(games.items(), key=lambda x: int(x[0])):
    s1, s2 = int(rs[-1][1]['succ_p1']), int(rs[-1][1]['succ_p2'])
    if s1 < 3 and s2 < 3 and rs[-1][0] >= 8:
        picked = (g, rs)
        break

g, rs = picked
print('=== stalled game', g, 'final succ', rs[-1][1]['succ_p1'], '-', rs[-1][1]['succ_p2'])
seen = set()
for t, d in rs:
    if t in seen:
        continue
    seen.add(t)
    en1, en2 = d['en_p1'], d['en_p2']
    c1, c2 = d['cost_p1'], d['cost_p2']
    h1, h2 = d['hand_p1'], d['hand_p2']
    print(' t%3d en %2s-%2s | stage_cost %3s-%3s | hand %2s-%2s' % (t, en1, en2, c1, c2, h1, h2))

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

def outcome(rs):
    s1, s2 = int(rs[-1][1]['succ_p1']), int(rs[-1][1]['succ_p2'])
    if s1 >= 3 and s2 <= 2: return 'P1'
    if s2 >= 3 and s1 <= 2: return 'P2'
    return 'DRAW'

# Per-turn average stage cost and energy, per player, split by final outcome
agg = defaultdict(lambda: [0, 0, 0, 0, 0])  # turn -> n, cost_p1_sum, cost_p2_sum, en_p1_sum, en_p2_sum
outcome_turns = defaultdict(lambda: defaultdict(list))  # outcome -> turn -> [cost_p1]

for g, rs in games.items():
    res = outcome(rs)
    seen = set()
    last = {}
    for t, d in rs:
        last[t] = d
    for t, d in sorted(last.items()):
        a = agg[t]
        a[0] += 1
        a[1] += int(d['cost_p1'])
        a[2] += int(d['cost_p2'])
        a[3] += int(d['en_p1'])
        a[4] += int(d['en_p2'])
        outcome_turns[res][t].append(int(d['cost_p1']))

print('turn | games | avg cost P1(v3) | avg cost P2(v2) | avg en P1 | avg en P2 | guide curve')
guide = {1: 4, 2: 9, 3: 13, 4: 22, 5: 28, 6: 34, 7: 40}
for t in sorted(agg):
    if t > 12:
        break
    n, c1, c2, e1, e2 = agg[t]
    print(f'  {t:2d} | {n:5d} | {c1/n:8.1f} | {c2/n:8.1f} | {e1/n:6.1f} | {e2/n:6.1f} | ~{guide.get(t,"?")}')

print()
print('=== v3 stage cost by outcome (avg per turn) ===')
for res in ('P1', 'P2', 'DRAW'):
    turns = outcome_turns[res]
    line = []
    for t in sorted(turns):
        if t > 8 or not turns[t]:
            continue
        line.append(f't{t}:{sum(turns[t])/len(turns[t]):.0f}')
    print(f'{res:5}:', ' '.join(line))

# energy hoarding: fraction of snapshots with active energy >= 6
hoard = Counter = {'p1': [0, 0], 'p2': [0, 0]}
for g, rs in games.items():
    for t, d in rs:
        if t >= 4:
            hoard['p1'][1] += 1
            if int(d['en_p1']) >= 6:
                hoard['p1'][0] += 1
            hoard['p2'][1] += 1
            if int(d['en_p2']) >= 6:
                hoard['p2'][0] += 1
print()
print(f'energy>=6 after t4 (v3): {hoard["p1"][0]}/{hoard["p1"][1]} = {100*hoard["p1"][0]/max(1,hoard["p1"][1]):.0f}%')
print(f'energy>=6 after t4 (v2): {hoard["p2"][0]}/{hoard["p2"][1]} = {100*hoard["p2"][0]/max(1,hoard["p2"][1]):.0f}%')

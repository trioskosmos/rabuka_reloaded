# -*- coding: utf-8 -*-
import csv
from collections import defaultdict

rows = list(csv.reader(open('../test_output/bot_arena_trace.csv')))
snaps = []
for r in rows:
    if len(r) > 2 and r[2] == 'ENTER':
        d = {}
        for c in r[3:]:
            if '=' in c:
                k, v = c.split('=', 1)
                d[k] = v
        snaps.append((int(r[0]), int(r[1]), d))
    elif len(r) > 6 and r[4].startswith('CHOICE:'):
        snaps.append((int(r[0]), int(r[1]), {'dec': r[3], 'act': r[4], 'sel': r[6]}))

# per (game,turn): track live-zone peaks and success deltas
games = defaultdict(list)
for g, t, x in snaps:
    games[g].append((t, x))

stats = {
    'P1': Counter := {'set': 0, 'placed': 0, 'set0_turns': 0, 'opp_placed_on_set0': 0},
    'P2': {'set': 0, 'placed': 0, 'set0_turns': 0, 'opp_placed_on_set0': 0},
}
# For each turn: max live zone per player mid-turn, and success delta across the turn
turn_data = {}
for g, evs in games.items():
    turns = defaultdict(list)
    for t, x in evs:
        if isinstance(x, dict) and 'live_p1' in x:
            turns[t].append(x)
    for t, xs in turns.items():
        if not xs:
            continue
        first, last = xs[0], xs[-1]
        maxl1 = max(int(x['live_p1']) for x in xs)
        maxl2 = max(int(x['live_p2']) for x in xs)
        s1b = int(first['succ_p1']); s1a = int(last['succ_p1'])
        s2b = int(first['succ_p2']); s2a = int(last['succ_p2'])
        turn_data[(g, t)] = (maxl1, maxl2, s1a - s1b, s2a - s2b)

for (g, t), (ml1, ml2, d1, d2) in sorted(turn_data.items()):
    if ml1 > 0:
        stats['P1']['set'] += 1
        if d1 > 0:
            stats['P1']['placed'] += 1
    else:
        stats['P1']['set0_turns'] += 1
        if d2 > 0:
            stats['P1']['opp_placed_on_set0'] += 1
    if ml2 > 0:
        stats['P2']['set'] += 1
        if d2 > 0:
            stats['P2']['placed'] += 1
    else:
        stats['P2']['set0_turns'] += 1
        if d1 > 0:
            stats['P2']['opp_placed_on_set0'] += 1

print('=== per-player live-phase outcomes (v4=P1, v2=P2) ===')
for p in ('P1', 'P2'):
    s = stats[p]
    setn = s['set']
    rate = 100.0 * s['placed'] / max(1, setn)
    print(f"{p}: turns with lives set: {setn} | placed on those: {s['placed']} ({rate:.0f}% check-win)")
    print(f"   turns with ZERO lives set: {s['set0_turns']} | opponent placed freely: {s['opp_placed_on_set0']}")

both_zero = sum(1 for v in turn_data.values() if v[0] == 0 and v[1] == 0)
no_place = sum(1 for v in turn_data.values() if v[2] == 0 and v[3] == 0)
print(f"\ntotal turn-phases: {len(turn_data)}")
print(f"double-pass phases (neither sets): {both_zero}")
print(f"phases with NO placement at all: {no_place} ({100*no_place/max(1,len(turn_data)):.0f}%)")

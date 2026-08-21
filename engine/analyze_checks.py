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

# Track per-turn: max live zone size during the live phase window,
# and whether successes incremented across the victory boundary.
set_counts = defaultdict(lambda: defaultdict(int))   # player -> n_lives -> count
check_events = defaultdict(lambda: [0, 0, 0])         # player -> [phases_with_lives, succ_increased, no_increase]

for g, rs in games.items():
    prev_succ1 = prev_succ2 = None
    prev_live1 = prev_live2 = None
    for i, (t, d) in enumerate(rs):
        s1, s2 = int(d['succ_p1']), int(d['succ_p2'])
        l1, l2 = int(d['live_p1']), int(d['live_p2'])
        act = d['active']
        ph = None
        # detect live-set entry snapshots: when a LiveCardSet phase begins we
        # can't see the phase name directly, but live-zone growth happens here.
        if prev_live1 is not None:
            if l1 > prev_live1:
                set_counts['P1'][min(l1, 4)] += 1
            if l2 > prev_live2:
                set_counts['P2'][min(l2, 4)] += 1
        # success increments between consecutive snapshots
        if prev_succ1 is not None and s1 > prev_succ1:
            check_events['P1'][0] += 1
            check_events['P1'][1] += 1
        if prev_succ1 is not None and s1 == prev_succ1 and (l1 > 0 or (prev_live1 or 0) > 0):
            pass
        if prev_succ2 is not None and s2 > prev_succ2:
            check_events['P2'][0] += 1
            check_events['P2'][1] += 1
        prev_succ1, prev_succ2 = s1, s2
        prev_live1, prev_live2 = l1, l2

print('lives set per event (count of zone-growth events by resulting zone size):')
for p in ('P1', 'P2'):
    print(' ', p, dict(sorted(set_counts[p].items())))

# Success placements total
for p in ('P1', 'P2'):
    e = check_events[p]
    print(f'{p}: success placements observed: {e[1]}')

# Per-game placement rate: how many live phases produce a placement?
# Estimate: total successes placed / total turns survived
turns_by_game = {g: rs[-1][0] for g, rs in games.items()}
succ_by_game = {}
for g, rs in games.items():
    succ_by_game[g] = (int(rs[-1][1]['succ_p1']), int(rs[-1][1]['succ_p2']))
total_turns = sum(turns_by_game.values())
total_placements = sum(e[1] for e in check_events.values())
print(f'total turns {total_turns}, total placements {total_placements} '
      f'-> {total_placements/max(1,total_turns):.2f} placements/turn (should approach ~1.0-1.5)')

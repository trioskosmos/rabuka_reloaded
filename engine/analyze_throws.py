# -*- coding: utf-8 -*-
import csv
from collections import defaultdict, Counter

rows = list(csv.reader(open('../test_output/bot_arena_trace.csv')))

# CHOICE rows: live-set decisions per player per turn
# ENTER rows: board snapshots
choices = defaultdict(list)   # (game,turn,player) -> [action types]
snaps = []
for r in rows:
    if len(r) < 5:
        continue
    if r[2] == 'ENTER':
        d = {}
        for cell in r[3:]:
            if '=' in cell:
                k, v = cell.split('=', 1)
                d[k] = v
        snaps.append((int(r[0]), int(r[1]), d))
    el    if r[4].startswith('CHOICE:'):
        choices[(r[0], r[1], r[3])].append(r[4])

# group snapshots by game/turn: last snapshot of each turn has zone state
turn_state = {}
for g, t, d in snaps:
    turn_state[(g, t)] = (int(d['live_p1']), int(d['live_p2']),
                          int(d['succ_p1']), int(d['succ_p2']))

# For each (game, turn): did P1/P2 select any live?
sel_any = defaultdict(lambda: {'P1': False, 'P2': False})
for (g, t, p), acts in choices.items():
    if any(a.startswith('CHOICE:select_live') for a in acts):
        sel_any[(g, t)][p] = True

both_pass = 0
one_pass = 0
both_set = 0
total_turns_with_live = len(sel_any)
empty_detail = []
for key, sel in sel_any.items():
    a, b = sel['P1'], sel['P2']
    if not a and not b:
        both_pass += 1
    elif a and b:
        both_set += 1
    else:
        one_pass += 1

print(f'turns with live-set decisions: {total_turns_with_live}')
print(f'  both set lives:   {both_set} ({100*both_set/max(1,total_turns_with_live):.0f}%)')
print(f'  exactly one sets: {one_pass} ({100*one_pass/max(1,total_turns_with_live):.0f}%)')
print(f'  BOTH PASS (double throw): {both_pass} ({100*both_pass/max(1,total_turns_with_live):.0f}%)')

# who passes more?
p1_only_pass = sum(1 for (g,t,p),acts in choices.items() if False)
pass_by = Counter()
for (g, t, p), acts in choices.items():
    if not any(a.startswith('CHOICE:select_live') for a in acts):
        pass_by[p] += 1
print('empty-confirm events by player:', dict(pass_by))

# draws vs finished among these games
game_outcome = {}
for (g, t), (l1, l2, s1, s2) in sorted(turn_state.items()):
    game_outcome[g] = (s1, s2)
res = Counter()
for g, (s1, s2) in game_outcome.items():
    if s1 >= 3 and s2 <= 2: res['P1'] += 1
    elif s2 >= 3 and s1 <= 2: res['P2'] += 1
    else: res['DRAW'] += 1

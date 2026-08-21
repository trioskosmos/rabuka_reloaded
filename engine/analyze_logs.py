# -*- coding: utf-8 -*-
import glob, re
from collections import Counter, defaultdict

files = sorted(glob.glob('../test_output/arena_logs/game_*.txt'))
stats = Counter()
per_game = []

for f in files:
    text = open(f, encoding='utf-8').read()
    head = text.splitlines()[0]
    m = re.search(r'final success (\d+)-(\d+) \| (\w+ \w+)', head)
    if not m:
        continue
    z1, z2, res = int(m.group(1)), int(m.group(2)), m.group(3)
    stats[res] += 1
    turns = [int(x) for x in re.findall(r'turn=(\d+)', text)]
    max_turn = max(turns) if turns else 0
    # count live phases where P1 had a performance but victory shows nothing placed
    perf_first = text.count('[[phase_performance_first]]')
    perf_second = text.count('[[phase_performance_second]]')
    per_game.append((f.split('_')[-1][:3], z1, z2, res, max_turn, perf_first, perf_second))

print('results:', dict(stats))
print('game | z1-z2 | result | turns | perfFA | perfSA')
for g in per_game:
    print(g)

# draws: how long?
draws = [g for g in per_game if g[3] == 'DRAW']
if draws:
    print('\ndraws:', len(draws), 'avg turn', sum(d[4] for d in draws)/len(draws))
losses = [g for g in per_game if g[3] == 'P2 WINS']
wins = [g for g in per_game if g[3] == 'P1 WINS']
if losses: print('P1 losses avg turn:', sum(l[4] for l in losses)/len(losses))
if wins: print('P1 wins avg turn:', sum(w[4] for w in wins)/len(wins))

# -*- coding: utf-8 -*-
import csv, sys

target = sys.argv[1] if len(sys.argv) > 1 else '1'
rows = list(csv.reader(open('../test_output/bot_arena_trace.csv')))

for r in rows[1:]:
    if r[0] != target:
        continue
    if r[2] == 'ENTER':
        d = dict(c.split('=', 1) for c in r[3:] if '=' in c)
        print('t%3s [%s] live %s-%s succ %s-%s hand %s/%s en %s-%s cost %s-%s' % (
            r[1], d.get('active', '?'),
            d.get('live_p1', '?'), d.get('live_p2', '?'),
            d.get('succ_p1', '?'), d.get('succ_p2', '?'),
            d.get('hand_p1', '?'), d.get('hand_p2', '?'),
            d.get('en_p1', '?'), d.get('en_p2', '?'),
            d.get('cost_p1', '?'), d.get('cost_p2', '?')))
    elif len(r) > 6:
        at = r[4]
        if 'Choice' in at or 'Select' in at or 'Confirm' in at or 'Skip' in at or 'Pass' in at or 'play_' in at or 'use_' in at:
            print('     t%3s   DECIDE %-4s %-28s card=%-20s sel=%s' % (r[1], r[3], at, r[5][:22], r[6][:16]))

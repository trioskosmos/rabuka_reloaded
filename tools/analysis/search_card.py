# -*- coding: utf-8 -*-
import json
import sys

from _paths import CARDS_JSON

d = json.load(open(CARDS_JSON, encoding='utf-8'))
needles = sys.argv[1:] or ['PL!SP-BP2-008-R']
for no in needles:
    c = d.get(no) or d.get(no.lower())
    if c:
        print(no, '| cost_field=', c.get('cost'))
        print('ability=%s' % (c.get('ability') or '')[:300])
    else:
        print(no, '| not found')

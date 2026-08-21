# -*- coding: utf-8 -*-
import json

d = json.load(open('../cards/cards.json'))
for no in ['PL!SP-BP2-008-R']:
    c = d.get(no) or d.get(no.lower())
    if c:
        print(no, '| cost_field=', c.get('cost'))
        print('ability=%s' % (c.get('ability') or '')[:300])

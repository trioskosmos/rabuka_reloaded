# -*- coding: utf-8 -*-
import json

d = json.load(open('../cards/cards.json'))
for no, c in d.items():
    if 'HS' in no.upper() and 'bp1-002' in no.lower():
        print(no)
        print('name=%s type=%s cost=%s' % (c.get('name'), c.get('type'), c.get('cost')))
        print('ability=%s' % (c.get('ability') or '')[:600])
        print('---')

# -*- coding: utf-8 -*-
import json

d = json.load(open('../cards/cards.json'))
target = u'始まりは君の空'
out = []
for no, c in d.items():
    if c.get('name') == target:
        out.append(u'%s\nname=%s type=%s cost=%s score=%s\nneed_heart=%s\nability=%s\nfaq=%s\n---' % (
            no, c.get('name'), c.get('type'), c.get('cost'), c.get('score'),
            json.dumps(c.get('need_heart'), ensure_ascii=False),
            (c.get('ability') or '')[:900],
            json.dumps((c.get('faq') or [])[:3], ensure_ascii=False)[:800]))
open('../tmp_search.txt', 'w', encoding='utf-8').write(u'\n\n'.join(out))
print(len(out))

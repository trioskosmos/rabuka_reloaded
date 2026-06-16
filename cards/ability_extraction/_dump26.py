# -*- coding: utf-8 -*-
import json

import os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
with open(os.path.join(ROOT, 'abilities.json'), encoding='utf-8') as f:
    cur = json.load(f)
ref_path = r'C:\Users\trios\Downloads\rabuka_reloaded-master (2)\rabuka_reloaded-master\cards\abilities.json'
with open(ref_path, encoding='utf-8') as f:
    ref = json.load(f)

cur_by = {a['triggerless_text']: a for a in cur['unique_abilities']}
ref_by = {a['triggerless_text']: a for a in ref['unique_abilities']}

def deep_equal(a, b):
    return json.dumps(a, sort_keys=True, ensure_ascii=False) == json.dumps(b, sort_keys=True, ensure_ascii=False)

lines = []
idx = 0
for t in sorted(set(cur_by) & set(ref_by)):
    ce = cur_by[t].get('effect')
    re_ = ref_by[t].get('effect')
    if not deep_equal(ce, re_):
        idx += 1
        lines.append("=" * 80)
        lines.append(f"### DIFF {idx}")
        lines.append("TEXT: " + t)
        lines.append("--- CUR effect ---")
        lines.append(json.dumps(ce, ensure_ascii=False, indent=2))
        lines.append("--- REF effect ---")
        lines.append(json.dumps(re_, ensure_ascii=False, indent=2))
        lines.append("")

with open(os.path.join(HERE, '_dump26.txt'), 'w', encoding='utf-8') as f:
    f.write(chr(10).join(lines) + chr(10))
print(f"wrote {idx} diffs")

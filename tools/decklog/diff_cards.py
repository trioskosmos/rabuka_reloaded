import json
from pathlib import Path

old = json.loads(Path("cards/cards.json.bak-before-pr").read_text(encoding="utf-8"))
new = json.loads(Path("cards/cards.json").read_text(encoding="utf-8"))

added = [k for k in new if k not in old]
removed = [k for k in old if k not in new]
changed = [k for k in new if k in old and json.dumps(new[k], ensure_ascii=False, sort_keys=True)
           != json.dumps(old[k], ensure_ascii=False, sort_keys=True)]

print(f"before: {len(old)}   after: {len(new)}")
print(f"added  : {len(added)}")
print(f"removed: {len(removed)}")
print(f"changed: {len(changed)}")
print()
fields = {}
for k in changed:
    o, n = old[k], new[k]
    for f in set(o) | set(n):
        if json.dumps(o.get(f), ensure_ascii=False, sort_keys=True) != \
           json.dumps(n.get(f), ensure_ascii=False, sort_keys=True):
            fields[f] = fields.get(f, 0) + 1
print("which fields changed, and how often:")
for f, c in sorted(fields.items(), key=lambda kv: -kv[1]):
    print(f"  {f:16} {c}")
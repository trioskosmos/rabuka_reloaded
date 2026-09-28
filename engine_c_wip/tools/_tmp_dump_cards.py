import json, sys
path = sys.argv[1]
want = set(sys.argv[2:])
db = json.load(open(path, encoding="utf-8"))
ua = db["unique_abilities"]
if isinstance(ua, dict):
    items = list(ua.items())
else:
    items = [(i, v) for i, v in enumerate(ua)]
for cid, c in items:
    txt = json.dumps(c, ensure_ascii=False)
    for w in want:
        if w in txt:
            print("=== idx", cid)
            print(json.dumps(c, ensure_ascii=False, indent=1))
            print()
            break

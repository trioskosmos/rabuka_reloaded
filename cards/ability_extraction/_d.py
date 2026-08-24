import json, os, sys

tmp = os.environ.get("TEMP", r"C:\Users\trios\AppData\Local\Temp")
ref = json.load(open(os.path.join(tmp, "opencode", "abilities_ref.json"), encoding="utf-8"))
new = json.load(open(sys.argv[1], encoding="utf-8"))


def strip_volatile(d):
    if isinstance(d, dict):
        return {k: strip_volatile(v) for k, v in d.items() if k != "generated_at"}
    if isinstance(d, list):
        return [strip_volatile(x) for x in d]
    return d


r, n = strip_volatile(ref), strip_volatile(new)
if r == n:
    print("IDENTICAL (except generated_at)")
else:
    ra = {a["full_text"]: a for a in r["unique_abilities"]}
    na = {a["full_text"]: a for a in n["unique_abilities"]}
    diff = sorted(k for k in ra.keys() & na.keys() if ra[k] != na[k])
    print(f"{len(diff)} abilities differ; {len(ra.keys() ^ na.keys())} added/removed")
    for k in diff[:10]:
        print("---", k[:70])

import json

bak = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json.bak"
o = json.load(open(bak, encoding="utf-8"))
for i, a in enumerate(o["unique_abilities"]):
    e = a.get("effect") or {}
    if "target_member" in e:
        print(f"AB#{i}: target_member={e['target_member']!r}")
        print(f"  text: {e.get('text','')[:100]}")

new = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json"
n = json.load(open(new, encoding="utf-8"))
for i, a in enumerate(n["unique_abilities"]):
    e = a.get("effect") or {}
    if "target_member" in e:
        print(f"NEW AB#{i}: target_member={e['target_member']!r}")

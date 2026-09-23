import json, re, sys
from collections import Counter, defaultdict

from _paths import ABILITIES_JSON

a = json.load(open(ABILITIES_JSON, encoding="utf-8"))
u = a["unique_abilities"]

# For each unique ability, does any effect node with ライブ終了時まで lack duration?
# Also: does top-level effect lack duration while text has the phrase?

def find_duration_misses(node, path, out, parent_has_dur=False):
    if isinstance(node, dict):
        text = ""
        for tk in ("text", "full_text", "triggerless_text"):
            if isinstance(node.get(tk), str):
                text = node[tk]
                break
        has_dur = "duration" in node and node["duration"]
        phrase = "ライブ終了時まで" in text or "ライブ終了まで" in text
        # miss if this node's own text has phrase and this node has no duration
        # (parent duration may still apply depending on engine semantics)
        if phrase and not has_dur:
            out.append({
                "path": path,
                "action": node.get("action"),
                "text": text[:240],
                "keys": sorted(node.keys()),
                "parent_has_dur": parent_has_dur,
            })
        for k, v in node.items():
            find_duration_misses(v, path + "/" + str(k), out, parent_has_dur or has_dur)
    elif isinstance(node, list):
        for i, v in enumerate(node):
            find_duration_misses(v, path + f"[{i}]", out, parent_has_dur)

misses = []
for i, ab in enumerate(u):
    find_duration_misses(ab.get("effect"), f"ab[{i}]", misses)

# Filter: only where NO ancestor has duration either (true gaps)
true_gaps = [m for m in misses if not m["parent_has_dur"]]
print(f"all node-level misses: {len(misses)}")
print(f"true gaps (no ancestor duration): {len(true_gaps)}")

# unique abilities affected
abs_idx = set()
for m in true_gaps:
    m2 = re.match(r"ab\[(\d+)\]", m["path"])
    if m2:
        abs_idx.add(int(m2.group(1)))
print(f"unique abilities with true gap: {len(abs_idx)}")

by_action = Counter(m["action"] for m in true_gaps)
print("by action:", by_action.most_common(30))

with open("tmp_duration_gaps.out", "w", encoding="utf-8") as f:
    f.write(f"true_gaps={len(true_gaps)} unique_abs={len(abs_idx)}\n\n")
    for i in sorted(abs_idx):
        ab = u[i]
        f.write(f"=== ab[{i}] cards={ab.get('card_count')} cards={ab.get('cards')[:5]}\n")
        f.write(f"full: {(ab.get('full_text') or '')[:300]}\n")
        # all gap nodes for this ab
        for m in true_gaps:
            if m["path"].startswith(f"ab[{i}]"):
                f.write(f"  {m['path']} action={m['action']} keys={m['keys']}\n")
                f.write(f"    text={m['text']}\n")
        f.write("\n")

print("wrote tmp_duration_gaps.out")

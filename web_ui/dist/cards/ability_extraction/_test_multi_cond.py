"""Test multi-branch conditional fix."""
import json, sys
from pathlib import Path
HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))
from parser import parse_ability

path = HERE.parent / "abilities.json"
with open(path, encoding="utf-8") as f:
    data = json.load(f)
abilities = data["unique_abilities"]

def show(label, eff, indent=0):
    p = "  " * indent
    print("%s%s:" % (p, label))
    if isinstance(eff, dict):
        for k, v in eff.items():
            if k in ("text", "_is_conditional", "_pattern", "_raw_text"):
                continue
            if isinstance(v, dict) and any(sk in v for sk in ("action", "actions", "condition")):
                print("%s  %s:" % (p, k))
                show("", v, indent + 2)
            elif isinstance(v, list) and v and isinstance(v[0], dict):
                print("%s  %s:" % (p, k))
                for i, item in enumerate(v):
                    print("%s  [%d]" % (p, i))
                    show("", item, indent + 3)
            else:
                vs = json.dumps(v, ensure_ascii=False)
                if len(vs) > 80:
                    vs = vs[:80] + "..."
                print("%s  %s: %s" % (p, k, vs))

# Test #150
print("=" * 60)
print("#150 BEFORE (stored in abilities.json):")
a = abilities[150]
show("effect", a.get("effect", {}))

print()
t = a.get("triggerless_text", "")
parsed = parse_ability(t)
print("#150 AFTER (fresh parser output):")
show("effect", parsed.get("effect", {}))
print()

# Test #63
print("=" * 60)
print("#63 BEFORE (stored in abilities.json):")
a = abilities[63]
show("effect", a.get("effect", {}))

print()
t = a.get("triggerless_text", "")
parsed = parse_ability(t)
print("#63 AFTER (fresh parser output):")
show("effect", parsed.get("effect", {}))

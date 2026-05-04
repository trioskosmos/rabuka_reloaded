"""Test icon_all and #63 fixes."""
import json, sys, re
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from parser import parse_action, parse_ability, _count_heart_icons

# Test _count_heart_icons
assert _count_heart_icons("{{heart_03.png|heart03}}{{heart_03.png|heart03}}") == 2
assert _count_heart_icons("{{icon_all.png|ハート}}{{icon_all.png|ハート}}") == 2
assert _count_heart_icons("nothing") is None
print("_count_heart_icons tests passed")

# Test icon_all detection in parse_action
r = parse_action("{{icon_all.png|ハート}}を得る")
print("icon_all action:", r.get("action"), "resource:", r.get("resource"), "count:", r.get("count"))
assert r.get("action") == "gain_resource"
assert r.get("resource") == "heart"
assert r.get("count") == 1

# Test #63
path = Path(__file__).parent.parent / "abilities.json"
with open(path, encoding="utf-8") as f:
    data = json.load(f)
a = data["unique_abilities"][63]
t = a.get("triggerless_text", "")
p = parse_ability(t)
acts = p.get("effect", {}).get("actions", [])
print()
print("#63 sub-actions:")
for i, act in enumerate(acts):
    print("  [%d]: %s resource=%s count=%s" % (i, act.get("action"), act.get("resource"), act.get("count")))

print()
print("All tests passed!")

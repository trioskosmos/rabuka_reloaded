"""
Investigate the 10 ERROR-level validation failures in detail.
"""
import json, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
import parser

ABILITIES_FILE = Path(__file__).parent.parent.parent / "abilities.json"
with open(ABILITIES_FILE, encoding="utf-8") as f:
    data = json.load(f)
data = parser.process_abilities(data)
abilities = data['unique_abilities']

def show(idx, label):
    a = abilities[idx]
    t = a.get("triggerless_text", "") or a.get("full_text", "")
    eff = a.get("effect", {})
    cost = a.get("cost", {})
    print(f"=== {label} (ability #{idx}) ===")
    print(f"text:   {t}")
    print(f"effect: {json.dumps(eff, ensure_ascii=False, indent=2)[:600]}")
    print(f"cost:   {json.dumps(cost, ensure_ascii=False, indent=2)[:200]}")
    print()

# E1: gain_resource_incomplete (#93)
show(93, "gain_resource_incomplete #93")

# E2: modify_score_incomplete (#150)
show(150, "modify_score_incomplete #150")

# E3: move_cards_incomplete (#290)
show(290, "move_cards_incomplete #290")

# E4: gain_ability_no_text (#353)
show(353, "gain_ability_no_text #353")

# E5: gain_resource_incomplete (#390)
show(390, "gain_resource_incomplete #390 (Emma Punch)")

# E6: move_cards_incomplete (#505)
show(505, "move_cards_incomplete #505")

# E7: gain_resource_incomplete (#584)
show(584, "gain_resource_incomplete #584")

# E8: move_cards_incomplete (#605)
show(605, "move_cards_incomplete #605")

# E9: gain_ability_no_text (#621)
show(621, "gain_ability_no_text #621")

# E10: move_cards_incomplete (#628)
show(628, "move_cards_incomplete #628")

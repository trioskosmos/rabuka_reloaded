"""Rank PBLL02 abilities by how many JSON fields no test reads.

`audit_field_coverage.py` reports the set; this ranks it, so the worklist has an order.
Also reports how many test files mention each card, so an ability with a single thin
file is distinguishable from one whose files simply use different vocabulary.
"""
import io
import json
import re
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
TESTS = ROOT / "engine" / "tests"

d = json.load(io.open(ROOT / "cards" / "abilities.json", encoding="utf-8"))
texts = {f: io.open(f, encoding="utf-8", errors="replace").read() for f in TESTS.rglob("*.rs")}

ACCESSOR = {
    "distinct": "distinct_any", "state_change": "state_change_any",
    "any_number": "any_number_any", "cost_limit": "cost_limit_any",
    "group_names": "group_names_any", "card_names": "card_names_any",
    "exclude_group_names": "exclude_group_names_any", "per_unit": "per_unit_any",
    "source": "source", "destination": "destination", "target": "target",
    "operation": "operation_any", "value": "value_any", "duration": "duration_any",
    "cost_total": "cost_total", "characters": "characters_any",
    "self_cost": "self_cost_any", "self_target": "self_target", "all": "all_any",
    "max": "max", "optional": "optional", "position": "position", "unit": "unit",
    "type": "type", "text": "text", "count": "count", "card_type": "card_type_any",
    "kind": "kind", "location": "location", "all_members": "all_members",
    "per_group": "per_group", "per_group_count": "per_group_count",
    "alternative_count_type": "alternative_count_type_any",
    "same_unit_name": "same_unit_name_any", "count_from": "count_from",
    "condition": "condition", "choice_condition": "choice_condition",
    "trigger_event": "get_trigger_event", "state": "state_any",
    "alternative_condition": "alternative_condition",
    "choice_type": "choice_type_any", "choice_options": "choice_options_any",
    "options": "options_any", "discard_remaining": "discard_remaining_any",
    "cost_limit_operator": "cost_limit_operator_any",
}


def collect(node, out):
    if isinstance(node, dict):
        for k, v in node.items():
            if v is None or v == [] or v == {} or v is False:
                continue
            out.add(k)
            collect(v, out)
    elif isinstance(node, list):
        for v in node:
            collect(v, out)


rows = []
for idx, a in enumerate(d["unique_abilities"]):
    cards = a.get("cards") or []
    if not any("pb2-" in c for c in cards):
        continue
    keys = set()
    collect(a.get("cost"), keys)
    collect(a.get("effect"), keys)
    keys -= {"text", "type", "action"}
    if not keys:
        continue
    blob = "\n".join(
        t for t in texts.values()
        if any(c.split("|")[0].strip() in t for c in cards)
    )
    unread = [k for k in sorted(keys)
              if not re.search(r"\b" + re.escape(ACCESSOR.get(k, k)) + r"\b", blob)]
    nfiles = sum(1 for t in texts.values()
                 if any(c.split("|")[0].strip() in t for c in cards))
    if unread:
        rows.append((len(unread), idx, cards[0].split("|")[0].strip(), unread, nfiles))

rows.sort(reverse=True)
print(f"abilities with at least one unasserted field: {len(rows)}")
print(f"total unasserted field/ability pairs: {sum(r[0] for r in rows)}")
print()
print("top 20 by unasserted field count:")
for n, idx, card, unread, nfiles in rows[:20]:
    print(f"  [{n:2d}] {card}  ({nfiles} file(s))  trigger={d['unique_abilities'][idx].get('triggers')}")
    print(f"        {unread}")

"""Which JSON fields of the PBLL02 abilities do the tests actually read?

Coverage counts ABILITIES, not FIELDS: a test that pins `action` and `count` is counted
the same as one that pins every field. So a field can be silently wrong with full
coverage showing green.

This lists every non-null key on each PBLL02 ability's cost/effect JSON and reports the
keys that no test for that card mentions at all — the JSON entries that are parsed,
stored and shipped, but never asserted. Those are the places a parse regression could
land without turning a single test red.
"""
import io
import json
import re
from pathlib import Path

ROOT = Path(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded")
TESTS = ROOT / "engine" / "tests"

d = json.load(io.open(ROOT / "cards" / "abilities.json", encoding="utf-8"))
abilities = d["unique_abilities"]

# Index test files by the card numbers they mention.
files = list(TESTS.rglob("*.rs"))
texts = {f: io.open(f, encoding="utf-8", errors="replace").read() for f in files}

# Keys the tests can only reach through an accessor, mapped to the accessor fragment.
# A key counts as read if the raw key name OR its accessor appears.
ACCESSOR = {
    "distinct": "distinct_any",
    "state_change": "state_change_any",
    "any_number": "any_number_any",
    "cost_limit": "cost_limit_any",
    "group_names": "group_names_any",
    "card_names": "card_names_any",
    "exclude_group_names": "exclude_group_names_any",
    "per_unit": "per_unit_any",
    "source": "source",
    "destination": "destination",
    "target": "target",
    "operation": "operation_any",
    "value": "value_any",
    "duration": "duration_any",
    "cost_total": "cost_total",
    "characters": "characters_any",
    "self_cost": "self_cost_any",
    "self_target": "self_target",
    "all": "all_any",
    "max": "max",
    "optional": "optional",
    "position": "position",
    "unit": "unit",
    "type": "type",
    "text": "text",
    "count": "count",
    "card_type": "card_type_any",
    "kind": "kind",
    "location": "location",
    "self_target": "self_target",
    "all_members": "all_members",
    "per_group": "per_group",
    "per_group_count": "per_group_count",
    "alternative_count_type": "alternative_count_type_any",
    "same_unit_name": "same_unit_name_any",
    "count_from": "count_from",
    "condition": "condition",
    "choice_condition": "choice_condition",
    "trigger_event": "get_trigger_event",
    "state": "state_any",
    "stage_select_intent": "stage_select",
    "post_debut": "post_debut",
    "alternative_condition": "alternative_condition",
    "choice_type": "choice_type_any",
    "choice_options": "choice_options_any",
    "options": "options_any",
}


def collect_keys(node, out):
    if isinstance(node, dict):
        for k, v in node.items():
            if v is None or v == [] or v == {} or v is False:
                continue
            out.add(k)
            collect_keys(v, out)
    elif isinstance(node, list):
        for v in node:
            collect_keys(v, out)


rows = []
for idx, a in enumerate(abilities):
    cards = a.get("cards") or []
    if not any("pb2-" in c for c in cards):
        continue
    keys = set()
    collect_keys(a.get("cost"), keys)
    collect_keys(a.get("effect"), keys)
    keys -= {"text", "type", "action"}
    if not keys:
        continue
    # Test files mentioning any of this ability's card numbers.
    hay = []
    for f, t in texts.items():
        if any(c.split("|")[0].strip() in t for c in cards):
            hay.append(t)
    blob = "\n".join(hay)
    unread = []
    for k in sorted(keys):
        acc = ACCESSOR.get(k, k)
        if not re.search(r"\b" + re.escape(acc) + r"\b", blob):
            unread.append(k)
    rows.append((idx, cards[0].split("|")[0].strip(), sorted(keys), unread, len(hay)))

print(f"PBLL02 abilities examined: {len(rows)}")
total_unread = 0
for idx, card, keys, unread, nfiles in rows:
    if unread:
        total_unread += len(unread)
        print(f"  [{idx}] {card}  ({nfiles} test file(s))")
        print(f"      never asserted: {unread}")
print()
print(f"field/ability pairs with no test assertion: {total_unread}")

import json, re, glob, sys, os
from collections import defaultdict, Counter


def normalize(s):
    s = s.strip().strip('"').strip("'")
    s = s.replace("\uff0b", "+").replace("\uff2b", "K").replace("\uff30", "P")
    s = s.replace("\uff2c", "L").replace("\uff21", "A").replace("\uff23", "C")
    s = s.replace("\uff25", "E").replace("\uff28", "H").replace("\uff33", "S")
    s = s.replace("\uff2e", "N")
    return s


with open("cards/abilities.json", encoding="utf-8") as f:
    abilities_data = json.load(f)
with open("cards/cards.json", encoding="utf-8") as f:
    cards_data = json.load(f)

MEMBER_TYPE = "\u30e1\u30f3\u30d0\u30fc"

card_abilities = {}
for ua in abilities_data.get("unique_abilities", []):
    full_text = ua.get("full_text", "")
    triggers = ua.get("triggers", [])
    if isinstance(triggers, str):
        triggers = [triggers]
    cards_list = ua.get("cards", [])
    for entry in cards_list:
        parts = entry.split(" | ")
        card_no = normalize(parts[0])
        if card_no not in card_abilities:
            card_abilities[card_no] = []
        card_abilities[card_no].append(
            {
                "full_text": full_text,
                "triggers": triggers,
            }
        )

member_ability_cards = {}
for cn, info in cards_data.items():
    cn_norm = normalize(cn)
    if info.get("type") == MEMBER_TYPE and cn_norm in card_abilities:
        member_ability_cards[cn_norm] = card_abilities[cn_norm]

print(f"Member cards with abilities: {len(member_ability_cards)}")
print()

card_no_pattern = re.compile(
    r"PL![A-Za-z0-9_!@#\$%\^&\*\-+\uff0b\uff2b\uff30\uff2c\uff21\uff23\uff25\uff28\uff33\uff2e]+"
)
test_dir = "engine/tests/test_modules"
qa_test_file = "engine/src/qa_test_suite.rs"
scenarios_file = "cards/scenarios.json"

tested_card_nos = set()

for fpath in glob.glob(f"{test_dir}/*.rs"):
    with open(fpath, encoding="utf-8") as f:
        content = f.read()
    for match in card_no_pattern.finditer(content):
        cn = normalize(match.group())
        if cn in member_ability_cards:
            tested_card_nos.add(cn)

with open(qa_test_file, encoding="utf-8") as f:
    content = f.read()
for match in card_no_pattern.finditer(content):
    cn = normalize(match.group())
    if cn in member_ability_cards:
        tested_card_nos.add(cn)

if os.path.exists(scenarios_file):
    try:
        with open(scenarios_file, encoding="utf-8") as f:
            scenarios = json.load(f)
        for scenario in scenarios if isinstance(scenarios, list) else []:
            scn = scenario.get("card_no", "")
            cn = normalize(scn)
            if cn in member_ability_cards:
                tested_card_nos.add(cn)
    except Exception as e:
        print(f"Warning: scenarios.json parse error: {e}")

print(f"Tested member card_no: {len(tested_card_nos)}")
untested = set(member_ability_cards.keys()) - tested_card_nos
print(
    f"UNTESTED member card_no: {len(untested)} ({100 * len(untested) / len(member_ability_cards):.1f}%)"
)
print()

# Group untested by trigger
trigger_untested = defaultdict(set)
for cn in untested:
    for ab in member_ability_cards[cn]:
        for t in ab["triggers"]:
            trigger_untested[t].add(cn)

trig_labels = {
    "\u767b\u5834": "debut(\u767b\u5834)",
    "\u8d77\u52d5": "activation(\u8d77\u52d5)",
    "\u5e38\u6642": "continuous(\u5e38\u6642)",
    "\u81ea\u52d5": "auto(\u81ea\u52d5)",
    "\u30e9\u30a4\u30d6\u958b\u59cb\u6642": "live_start(\u30e9\u30a4\u30d6\u958b\u59cb\u6642)",
    "\u30e9\u30a4\u30d6\u6210\u529f\u6642": "live_success(\u30e9\u30a4\u30d6\u6210\u529f\u6642)",
    "\u5de6\u30b5\u30a4\u30c9": "left_side(\u5de6\u30b5\u30a4\u30c9)",
    "\u53f3\u30b5\u30a4\u30c9": "right_side(\u53f3\u30b5\u30a4\u30c9)",
}

print("Untested cards by trigger type:")
for t_jp, label in trig_labels.items():
    group = trigger_untested.get(t_jp, set())
    if group:
        print(f"  {label}: {len(group)}")
print()

# Coverage by trigger type
print("=== Coverage summary by trigger type ===")
for t_jp, label in trig_labels.items():
    all_cards = set()
    tested_c = set()
    for cn, abs_list in member_ability_cards.items():
        for ab in abs_list:
            if t_jp in ab["triggers"]:
                all_cards.add(cn)
                if cn in tested_card_nos:
                    tested_c.add(cn)
    pct = 100 * len(tested_c) / max(len(all_cards), 1)
    print(f"  {label}: {len(tested_c)}/{len(all_cards)} ({pct:.0f}%)")
print()

# Test file coverage
print("=== Test file card coverage ===")
test_file_cards = {}
for fpath in sorted(glob.glob(f"{test_dir}/*.rs")):
    fname = os.path.basename(fpath)
    with open(fpath, encoding="utf-8") as f:
        content = f.read()
    covered = set()
    for match in card_no_pattern.finditer(content):
        cn = normalize(match.group())
        if cn in member_ability_cards:
            covered.add(cn)
    if covered:
        test_file_cards[fname] = covered

for fname, cards in sorted(test_file_cards.items()):
    sample = ", ".join(sorted(cards)[:5])
    print(f"  {fname}: {len(cards)} cards -> {sample}")
print(f"Total files with coverage: {len(test_file_cards)}")
print()

# Write untested list to file
with open("untested_abilities.txt", "w", encoding="utf-8") as f:
    f.write(f"Untested member cards with abilities: {len(untested)}\n\n")
    for cn in sorted(untested):
        abs_list = member_ability_cards[cn]
        triggers = set()
        texts = set()
        for ab in abs_list:
            triggers.update(ab["triggers"])
            texts.add(ab["full_text"][:80])
        f.write(f"{cn}: triggers={triggers}\n")
        for t in texts:
            f.write(f"  {t}\n")
        f.write("\n")
print("Wrote untested_abilities.txt")

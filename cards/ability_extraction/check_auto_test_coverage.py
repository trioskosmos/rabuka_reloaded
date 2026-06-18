"""For each 自動 ability, check if ANY of its card IDs appear in ANY test file."""
import os, json
from collections import defaultdict

with open(r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json", encoding="utf-8") as f:
    data = json.load(f)

auto_abilities = [ab for ab in data["unique_abilities"] if ab.get("triggers") == "自動"]

tests_dir = r"C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\tests\test_modules"
test_files = [os.path.join(tests_dir, f) for f in os.listdir(tests_dir) if f.endswith(".rs")]

# Load all test file contents into one big string per file
test_content = {}
for tf in test_files:
    with open(tf, encoding="utf-8") as f:
        test_content[tf] = f.read()

all_content = "\n".join(test_content.values())

tested = 0
untested = 0
for ab in auto_abilities:
    card_ids = [c.split(" (ab#")[0] for c in ab["cards"]]
    # also try with + instead of ＋
    all_variants = set()
    for cid in card_ids:
        all_variants.add(cid)
        all_variants.add(cid.replace("＋", "+"))

    found = []
    for tf, content in test_content.items():
        for v in all_variants:
            if v in content:
                found.append((os.path.basename(tf), v))
                break

    cid0 = card_ids[0]
    if found:
        tested += 1
        print(f"TESTED   {cid0} | n={ab['card_count']}")
        for tf, v in found:
            print(f"  -> {tf} ({v})")
    else:
        untested += 1
        print(f"UNTESTED {cid0} | n={ab['card_count']}")

print(f"\nTested: {tested}/56  Untested: {untested}/56")

"""
Methodology
───────────
This script compares QAs in cards/qa_data.json against references found in
engine/src and engine/tests to identify unreferenced QAs.

Source files:
  - cards/qa_data.json -- all QAs with related_cards, questions, answers
  - cards/abilities.json -- parsed ability structures for every card
  - cards/ability_extraction/parser.py -- parses card ability text into JSON
  - engine/tests/ -- Rust test modules; each test should reference relevant QAs

How to read JSON files:
  python -c "import json; d=json.load(open('cards/qa_data.json')); [q for q in d if q['id']=='Q264']"
  python -c "import json; d=json.load(open('cards/abilities.json')); [a for a in d['unique_abilities'] if 'pb2-020' in str(a)]"
  python -c "import json; d=json.load(open('cards/cards.json')); print(d.get('PL!SP-pb2-020-R', {}).get('ability',''))"

For each QA in qa_data.json:
  - Group by related ability in abilities.json, not by QA number.
  - If the ability is already tested in an existing test file, add a Q reference there.
  - Otherwise add tests to the existing test file for that card/ability group.
  - Do NOT create separate "qa_new_testsXXX.rs" files; put tests with related abilities.
  - The QA involves a card → that card must be properly parsed, exist in engine,
    and have a written scenario with all edge cases tested (like existing tests).
  - The QA is a general rule (no card) → the rule must be properly implemented
    in engine code.
  - The QA is about real-life/tournament procedures → low priority, skip.
  - When a test proves the parser/engine does NOT match the expected behavior
    from the Q&A, fix the parser/engine — do NOT leave passing tests that
    confirm incorrect behavior.

QAs NOT referenced in engine code are candidates for new tests or implementation.

Only .rs files are scanned; this script (.py) excludes itself automatically.
"""

import json, os, re

qa_path = r"cards/qa_data.json"
search_dirs = [r"engine/src", r"engine/tests"]

methodology = """Methodology
───────────
Source files:
  - cards/qa_data.json -- all QAs with related_cards, questions, answers
  - cards/abilities.json -- parsed ability structures for every card
  - cards/ability_extraction/parser.py -- parses card ability text into JSON
  - engine/tests/ -- Rust test modules; each test should reference relevant QAs

How to read JSON files:
  python -c "import json; d=json.load(open('cards/qa_data.json')); [q for q in d if q['id']=='Q264']"
  python -c "import json; d=json.load(open('cards/abilities.json')); [a for a in d['unique_abilities'] if 'pb2-020' in str(a)]"
  python -c "import json; d=json.load(open('cards/cards.json')); print(d.get('PL!SP-pb2-020-R', {}).get('ability',''))"

For each QA in qa_data.json:
  - Group by related ability in abilities.json, not by QA number.
  - If the ability is already tested in an existing test file, add a Q reference there.
  - Otherwise add tests to the existing test file for that card/ability group.
  - Do NOT create separate "qa_new_testsXXX.rs" files; put tests with related abilities.
  - The QA involves a card → that card must be properly parsed, exist in engine,
    and have a written scenario with all edge cases tested (like existing tests).
  - The QA is a general rule (no card) → the rule must be properly implemented
    in engine code.
  - The QA is about real-life/tournament procedures → low priority, skip.
  - When a test proves the parser/engine does NOT match the expected behavior
    from the Q&A, fix the parser/engine — do NOT leave passing tests that
    confirm incorrect behavior.

QAs NOT referenced in engine code are candidates for new tests or implementation.
"""
print(methodology)

with open(qa_path, "r", encoding="utf-8") as f:
    qa_list = json.load(f)

qa_ids = [q["id"] for q in qa_list]

found = set()
for sd in search_dirs:
    for root, dirs, files in os.walk(sd):
        for fname in files:
            if fname.endswith(".rs"):
                with open(
                    os.path.join(root, fname), "r", encoding="utf-8", errors="ignore"
                ) as f:
                    for m in re.finditer(r"[Qq](\d{1,3})", f.read()):
                        found.add(f"Q{int(m.group(1))}")

missing = [q for q in qa_ids if q not in found]

# Q1-Q22 contain real-life/tournament-procedural QAs that don't apply to
# engine code: product info (Q1), tournament rules (Q2), deck construction
# (Q3-Q7), sleeves/etiquette (Q8-Q14), forgot-phase procedures (Q20-Q22).
# Q15-Q19 (energy orientation, RPS, mulligan) ARE game rules and excluded
# from this filter since they may have engine references.
irl = {f"Q{i}" for i in range(1, 15)} | {f"Q{i}" for i in range(20, 23)}
missing = [q for q in missing if q not in irl]

print(f"Total QAs in qa_data.json: {len(qa_ids)}")
print(f"QAs referenced in {', '.join(search_dirs)}: {len(found)}")
print(f"QAs NOT referenced: {len(missing)}")
print()
print("=== QAs NOT referenced (highest first) ===")
for q in missing:
    print(q)

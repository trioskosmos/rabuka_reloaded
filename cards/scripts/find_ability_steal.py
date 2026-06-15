"""Find ALL abilities that reference/steal/copy other cards' abilities."""

import json, re
from pathlib import Path

with open(Path(__file__).parent.parent / "abilities.json", "r", encoding="utf-8") as f:
    data = json.load(f)

abilities = data["unique_abilities"]

# Patterns for "gain ability from a source"
patterns = [
    # Pattern 1: "持つ{trigger}能力を得る" — gain abilities FROM a card matching conditions
    ("持つ...能力", r"持つ\{\{[^}]*\}\}能力"),
    # Pattern 2: "の能力を得る" — gain ability OF something
    ("の...能力を得る", r"の[^を]*能力を得る"),
    # Pattern 3: "能力をすべて得る" — gain ALL abilities
    ("能力をすべて得る", r"能力をすべて得る"),
    # Pattern 4: "すべての能力を得る" — gain all abilities
    ("すべての能力を得る", r"すべての能力を得る"),
    # Pattern 5: "能力を得る" general (might overlap with above)
    ("能力を得る", r"能力を得る"),
    # Pattern 6: Explicit "持つ能力" — has-ability referencing
    ("持つ能力", r"持つ能力"),
]

print("=== Ability-referencing patterns ===")
for name, pat in patterns:
    matching = [
        (i, a)
        for i, a in enumerate(abilities)
        if re.search(pat, a.get("triggerless_text", ""))
    ]
    print(f"\n--- {name} -- {len(matching)} matches ---")
    for idx, a in matching:
        t = a.get("triggerless_text", "")
        eff = a.get("effect", {})
        print(f"\n  #{idx} action={eff.get('action')}")
        print(f"       cards={a.get('cards', [])[:2]}")
        print(f"       text={t[:120]}")
        # Show what the source condition is (which cards to steal from)
        src_match = re.search(r"[のがを]持つ[^。]*能力", t)
        if src_match:
            print(f"       SOURCE: {src_match.group()[:80]}")
        print()

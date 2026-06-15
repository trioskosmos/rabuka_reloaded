"""Find ALL abilities that reference/steal/copy other cards' abilities."""

import json, re, sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8")

with open(Path(__file__).parent.parent / "abilities.json", "r", encoding="utf-8") as f:
    data = json.load(f)

abilities = data["unique_abilities"]

# The kanji for 能力 can be 能 or other forms. Let me search for the actual byte
# patterns. First, dump texts that contain 得る to find all gain-related abilities.
print("=== ALL abilities containing 得る (gain) ===")
for i, a in enumerate(abilities):
    t = a.get("triggerless_text", "")
    if "得" in t:
        eff = a.get("effect") or {}
        print(f"\n#{i} action={eff.get('action', '?')}")
        print(f"  cards={a.get('cards', [])}")
        print(f"  text={t[:120]}")

print("\n\n=== ALL abilities mentioning other cards' abilities ===")
# Search for patterns like:
# - "が持つ" (card has)
# - "の持つ" (card's)
# - "の下" (under card)
# - "ている" (cards that are...)
for i, a in enumerate(abilities):
    t = a.get("triggerless_text", "")
    eff = a.get("effect") or {}
    # Check if the text references OTHER CARDS' abilities
    if any(
        pat in t for pat in ["が持つ", "の持つ", "持っている", "の下に置かれている"]
    ):
        print(f"\n#{i} action={eff.get('action', '?')}")
        print(f"  cards={a.get('cards', [])}")
        print(f"  text={t[:150]}")

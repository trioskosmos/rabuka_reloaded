"""
Full pipeline: take a raw cards JSON file and produce a properly formatted one.

Standardizations:
1. Strip extra quote-wrapping from ability strings (P+/SEC variants)
2. Insert newlines between separate ability triggers so the parser can split them

Usage:
    python cards/scripts/standardize_cards.py <input.json> [output.json]
"""

import json
import re
import sys

TRIGGER_AFTER_PUNCT = re.compile(
    r"([。！？])\s*(?=\{\{(?:kidou|jidou|toujyou|live_start|live_success|live_end)\.png\|)"
)


def strip_extra_quotes(ability: str) -> str:
    if ability.startswith('"') and ability.endswith('"') and len(ability) >= 2:
        inner = ability[1:-1]
        if "{{" in inner:
            return inner
    return ability


def _outside_quotes(ability: str, pos: int) -> bool:
    before = ability[:pos]
    return before.count("「") <= before.count("」")


def split_ability_lines(ability: str) -> str:
    return TRIGGER_AFTER_PUNCT.sub(
        lambda m: m.group(1) + "\n"
        if _outside_quotes(ability, m.start())
        else m.group(0),
        ability,
    )


def standardize(card: dict) -> bool:
    ability = card.get("ability")
    if not ability:
        return False
    original = ability
    ability = strip_extra_quotes(ability)
    ability = split_ability_lines(ability)
    if ability != original:
        card["ability"] = ability
        return True
    return False


def process_file(input_path: str, output_path: str | None = None) -> None:
    with open(input_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    changed = 0
    for card in data.values():
        if standardize(card):
            changed += 1

    out = output_path or input_path
    with open(out, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f"Processed {len(data)} cards, modified {changed}.")
    print(f"Output: {out}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    process_file(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None)

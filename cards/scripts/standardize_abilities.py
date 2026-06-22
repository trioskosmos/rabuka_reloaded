"""
Strip extra quote-wrapping from card ability strings.

Fixes data errors where ability text is wrapped in literal quote
characters (found in P+ and SEC rarity variants).

Usage:
    python cards/scripts/standardize_abilities.py <input.json> [output.json]
    (if output.json omitted, input file is modified in place)
"""

import json
import sys


def strip_extra_quotes(ability: str) -> str:
    if ability.startswith('"') and ability.endswith('"') and len(ability) >= 2:
        inner = ability[1:-1]
        if "{{" in inner:
            return inner
    return ability


def process_file(input_path: str, output_path: str | None = None) -> None:
    with open(input_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    changed = 0
    for key, card in data.items():
        if "ability" in card and card["ability"]:
            original = card["ability"]
            cleaned = strip_extra_quotes(original)
            if cleaned != original:
                card["ability"] = cleaned
                changed += 1

    out = output_path or input_path
    with open(out, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f"Processed {len(data)} cards, fixed {changed} ability strings.")
    print(f"Output: {out}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2] if len(sys.argv) > 2 else None
    process_file(input_path, output_path)

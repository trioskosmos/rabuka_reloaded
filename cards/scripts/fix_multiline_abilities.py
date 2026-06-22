"""
Split multi-ability card text onto separate lines.

The parser splits abilities by newline. Cards with multiple separate
triggers (e.g. {{live_start}} + {{live_success}}) must have each
trigger block on its own line. This script restores newlines where
they were merged onto a single line.

Trigger icons that start a new line: kidou, jidou, toujyou,
live_start, live_success, live_end.

Cost/position icons (turn, heart, energy, blade, center, etc.) and
slash-combined triggers ({{A}}/{{B}}) are NOT split.

Usage:
    python cards/scripts/fix_multiline_abilities.py <input.json> [output.json]
"""

import json
import re
import sys

NEW_ABILITY_TRIGGERS = {
    "kidou",
    "jidou",
    "toujyou",
    "live_start",
    "live_success",
    "live_end",
}

ICON_PATTERN = re.compile(r"\{\{(\w+)\.png\|([^}]+)\}\}")


def has_newlines_between_triggers(text: str) -> bool:
    """Check if ability text already has proper newline separation."""
    lines = text.split("\n")
    trigger_count = 0
    for line in lines:
        line = line.strip()
        if not line:
            continue
        # Check if line starts with a trigger
        first_icon = ICON_PATTERN.search(line)
        if first_icon and first_icon.start() == 0:
            icon_file = first_icon.group(1)
            if icon_file in NEW_ABILITY_TRIGGERS:
                trigger_count += 1
    return trigger_count > 1


def needs_newline_before(text: str, pos: int) -> bool:
    """Check if position pos (start of a {{trigger}} match) needs a newline before it."""
    before = text[:pos]
    # Don't split if at the start of text
    if not before.strip():
        return False
    # Don't split if inside 「」 quotes
    open_quotes = before.count("「")
    close_quotes = before.count("」")
    if open_quotes > close_quotes:
        return False
    # Don't split if there's already a newline right before
    if before.rstrip().endswith("\n"):
        return False
    # Don't split if the trigger is part of a slash combo: {{A}}/{{trigger}}
    if before.rstrip().endswith("/") or before.rstrip().endswith("／"):
        return False
    return True


def restore_newlines(ability: str) -> str:
    """Insert newlines before separate ability triggers that appear mid-line."""
    changes = True
    while changes:
        changes = False
        for m in ICON_PATTERN.finditer(ability):
            icon_file = m.group(1)
            if icon_file not in NEW_ABILITY_TRIGGERS:
                continue
            pos = m.start()
            if needs_newline_before(ability, pos):
                ability = ability[:pos] + "\n" + ability[pos:]
                changes = True
                break
    return ability


def process_file(input_path: str, output_path: str | None = None) -> None:
    with open(input_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    changed = 0
    for key, card in data.items():
        if "ability" in card and card["ability"]:
            original = card["ability"]
            fixed = restore_newlines(original)
            if fixed != original:
                card["ability"] = fixed
                changed += 1

    out = output_path or input_path
    with open(out, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f"Processed {len(data)} cards, fixed {changed}.")
    print(f"Output: {out}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2] if len(sys.argv) > 2 else None
    process_file(input_path, output_path)

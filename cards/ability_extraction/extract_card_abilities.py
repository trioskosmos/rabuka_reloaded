#!/usr/bin/env python3
"""
Extract card abilities from cards.json.
Splits abilities by newline, extracts triggers, groups by text.
Reuses parsed cost/effect from the existing working abilities.json for consistency.
"""

import json
import re
import sys
from pathlib import Path
from datetime import datetime
from collections import defaultdict

sys.path.append(str(Path(__file__).parent.parent))

TRIGGER_PATTERN = re.compile(r"\{\{([^|]+)\|([^}]+)\}\}")
SLASH_TRIGGER_PATTERN = re.compile(r"/\{\{([^|]+)\|([^}]+)\}\}")


def extract_trigger(text: str) -> tuple[list, int | None, str]:
    cost_icon_patterns = [
        "icon_energy",
        "heart",
        "icon_blade",
        "icon_b_all",
        "icon_score",
    ]
    position_icon_patterns = ["center", "left", "right"]
    use_limit_patterns = ["turn", "ターン"]

    triggers = []
    use_limit = None
    text = text.strip().lstrip('"').lstrip("\u201c").lstrip("\u201d").strip()
    effect = text

    bracket_turn_match = re.search(r"［ターン1回］", effect)
    if bracket_turn_match:
        use_limit = 1
        effect = effect.replace("［ターン1回］", "", 1).strip()

    slash_matches = SLASH_TRIGGER_PATTERN.findall(text)
    for match in slash_matches:
        icon_file, icon_text = match
        slash_pattern = f"/{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(slash_pattern, "", 1)
        triggers.append(icon_text)

    trigger_matches = TRIGGER_PATTERN.findall(text)
    pos = 0
    for match in trigger_matches:
        icon_file, icon_text = match
        match_start = text.find(f"{{{{{icon_file}|{icon_text}}}}}", pos)

        before = text[pos:match_start]
        if before.strip() and before.strip() != "：":
            if before.strip() == "/":
                pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
                continue
            break

        if any(cost_pattern in icon_file for cost_pattern in cost_icon_patterns):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        if triggers and any(p in icon_file for p in position_icon_patterns):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        if any(
            use_limit_pattern in icon_text for use_limit_pattern in use_limit_patterns
        ):
            num_match = re.match(r"ターン(\d+)回", icon_text)
            use_limit = int(num_match.group(1)) if num_match else None
            trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
            effect = effect.replace(trigger_pattern, "", 1)
            pos = match_start + len(trigger_pattern)
            continue

        quote_count = text[:match_start].count("「") - text[:match_start].count("」")
        if quote_count > 0:
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        if icon_text in triggers:
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue
        triggers.append(icon_text)
        trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(trigger_pattern, "", 1)
        pos = match_start + len(trigger_pattern)

    effect = effect.strip()
    return triggers, use_limit, effect


def extract_abilities_from_card(card_id: str, card: dict) -> list:
    abilities = []
    ability_text = card.get("ability", "")
    if not ability_text:
        return abilities

    ability_lines = ability_text.split("\n")
    for i, line in enumerate(ability_lines):
        line = line.strip()
        if not line:
            continue

        if line.startswith("・"):
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            continue

        if (line.startswith("(") and line.endswith(")")) or (
            line.startswith("（") and line.endswith("）")
        ):
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            else:
                abilities.append(
                    {
                        "card_id": card_id,
                        "full_text": line,
                        "triggerless_text": "",
                        "use_limit": None,
                        "triggers": [],
                        "trigger_count": 0,
                        "ability_index": i,
                        "is_null": True,
                    }
                )
            continue

        triggers, use_limit, effect = extract_trigger(line)

        if not triggers and abilities and abilities[-1]["trigger_count"] > 0:
            if line.startswith("回答が") or line.startswith("場合") or "の場合" in line:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
                continue
            if (
                line.startswith("とき、")
                or line.startswith("なら、")
                or line.startswith("場合、")
            ):
                abilities[-1]["full_text"] += line
                abilities[-1]["triggerless_text"] += line
                continue

        if not triggers:
            abilities.append(
                {
                    "card_id": card_id,
                    "full_text": line,
                    "triggerless_text": "",
                    "use_limit": None,
                    "triggers": [],
                    "trigger_count": 0,
                    "ability_index": i,
                    "is_null": True,
                }
            )
        else:
            abilities.append(
                {
                    "card_id": card_id,
                    "full_text": line,
                    "triggerless_text": effect,
                    "use_limit": use_limit,
                    "once_per_turn": use_limit == 1 if use_limit else False,
                    "triggers": triggers,
                    "trigger_count": len(triggers),
                    "ability_index": i,
                }
            )

    return abilities


def extract_all_abilities(cards_file: Path, working_file: Path) -> dict:
    with open(cards_file, encoding="utf-8") as f:
        cards = json.load(f)

    working_data = {}
    if working_file.exists():
        try:
            with open(working_file, encoding="utf-8") as f:
                wd = json.load(f)
            for u in wd.get("unique_abilities", []):
                working_data[u["full_text"]] = u
        except Exception:
            pass
    if not working_data:
        import subprocess

        try:
            r = subprocess.run(
                ["git", "show", "HEAD:cards/abilities.json"],
                capture_output=True,
                timeout=10,
            )
            if r.returncode == 0:
                wd = json.loads(r.stdout)
                for u in wd.get("unique_abilities", []):
                    working_data[u["full_text"]] = u
        except Exception:
            pass

    all_abilities = []
    ability_groups = defaultdict(list)

    if isinstance(cards, list):
        cards_dict = {card.get("card_no", str(i)): card for i, card in enumerate(cards)}
    else:
        cards_dict = cards

    for card_id, card in cards_dict.items():
        abilities = extract_abilities_from_card(card_id, card)
        for ability in abilities:
            all_abilities.append(ability)
            card_example = (
                f"{card_id} | {card.get('name', '')} (ab#{ability['ability_index']})"
            )
            ability_groups[ability["full_text"]].append(card_example)

    unique_abilities = []
    for full_text, card_examples in ability_groups.items():
        sample = next(a for a in all_abilities if a["full_text"] == full_text)

        is_null = sample.get("is_null", False)

        if full_text in working_data:
            wu = working_data[full_text]
            entry = {
                "full_text": full_text,
                "triggerless_text": sample["triggerless_text"],
                "card_count": len(card_examples),
                "cards": card_examples,
                "triggers": ", ".join(sample["triggers"])
                if sample["triggers"]
                else wu.get("triggers"),
                "use_limit": wu.get("use_limit"),
                "is_null": wu.get("is_null", is_null),
                "cost": wu.get("cost"),
                "effect": wu.get("effect"),
            }
        else:
            try:
                print(f"WARNING: New ability not in working file: {full_text[:60]}...")
            except UnicodeEncodeError:
                print("WARNING: New ability not in working file (non-ASCII text)")
            entry = {
                "full_text": full_text,
                "triggerless_text": sample["triggerless_text"],
                "card_count": len(card_examples),
                "cards": card_examples,
                "triggers": ", ".join(sample["triggers"])
                if sample["triggers"]
                else None,
                "use_limit": sample["use_limit"],
                "is_null": is_null,
                "cost": None,
                "effect": None,
            }

        unique_abilities.append(entry)

    unique_abilities.sort(key=lambda x: -x["card_count"])

    return {
        "schema": "extracted_abilities.v1",
        "generated_at": datetime.now().isoformat(),
        "generated_by": "cards/ability_extraction/extract_card_abilities.py",
        "source_file": str(cards_file),
        "statistics": {
            "total_cards": len(cards_dict),
            "cards_with_abilities": len(
                [c for c in cards_dict.values() if c.get("ability")]
            ),
            "total_abilities": len(all_abilities),
            "unique_abilities": len(unique_abilities),
        },
        "unique_abilities": unique_abilities,
    }


def main():
    cards_file = Path(__file__).parent.parent / "cards.json"
    output_file = Path(__file__).parent.parent / "abilities.json"
    working_file = output_file

    print(f"Extracting abilities from {cards_file}...")
    result = extract_all_abilities(cards_file, working_file)

    print(
        f"Found {result['statistics']['total_abilities']} abilities across "
        f"{result['statistics']['cards_with_abilities']} cards"
    )
    print(f"Unique abilities: {result['statistics']['unique_abilities']}")

    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)

    print(f"Output written to {output_file}")
    print("Validation complete.")


if __name__ == "__main__":
    main()

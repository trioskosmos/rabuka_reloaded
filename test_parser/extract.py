"""Extract abilities from cards.json, group by full text."""

import json
import re
from datetime import datetime
from collections import defaultdict
from pathlib import Path

TRIGGER_PATTERN = re.compile(r"\{\{([^|]+)\|([^}]+)\}\}")
SLASH_TRIGGER_PATTERN = re.compile(r"/\{\{([^|]+)\|([^}]+)\}\}")
USE_LIMIT_PATTERNS = ["turn", "ターン"]
COST_ICON_PATTERNS = ["icon_energy", "heart", "icon_blade", "icon_b_all", "icon_score"]


def extract_trigger(text: str):
    triggers = []
    use_limit = None
    text = text.strip().lstrip('"').lstrip("\u201c").lstrip("\u201d").strip()
    effect = text

    bracket_turn_match = re.search(r"［ターン1回］", effect)
    if bracket_turn_match:
        use_limit = 1
        effect = effect.replace("［ターン1回］", "", 1).strip()

    slash_matches = SLASH_TRIGGER_PATTERN.findall(text)
    for icon_file, icon_text in slash_matches:
        slash_pattern = f"/{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(slash_pattern, "", 1)
        triggers.append(icon_text)

    trigger_matches = TRIGGER_PATTERN.findall(text)
    pos = 0
    for icon_file, icon_text in trigger_matches:
        match_start = text.find(f"{{{{{icon_file}|{icon_text}}}}}", pos)

        before = text[pos:match_start]
        if before.strip() and before.strip() != "：":
            if before.strip() == "/":
                pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
                continue
            break

        if any(cost_pattern in icon_file for cost_pattern in COST_ICON_PATTERNS):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        if any(
            use_limit_pattern in icon_text for use_limit_pattern in USE_LIMIT_PATTERNS
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


def extract_all_abilities(cards_file: Path) -> dict:
    with open(cards_file, encoding="utf-8") as f:
        cards = json.load(f)

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
        entry = {
            "full_text": full_text,
            "triggerless_text": sample["triggerless_text"],
            "card_count": len(card_examples),
            "cards": card_examples,
            "triggers": ", ".join(sample["triggers"]) if sample["triggers"] else None,
            "use_limit": sample["use_limit"],
            "is_null": sample.get("is_null", False),
            "cost": None,
            "effect": None,
        }
        unique_abilities.append(entry)

    unique_abilities.sort(key=lambda x: -x["card_count"])

    return {
        "schema": "extracted_abilities.v1",
        "generated_at": datetime.now().isoformat(),
        "generated_by": "test_parser",
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

#!/usr/bin/env python3
"""
Extract card abilities from cards.json.
Splits abilities by newline and extracts triggers.

This script generates: data/abilities_extracted_from_cards.json
Source: data/cards.json
"""

import json
import re
import sys
from pathlib import Path
from datetime import datetime
from collections import defaultdict

# Add parent directory to path for imports
sys.path.append(str(Path(__file__).parent.parent))

from parser import (
    parse_cost,
    parse_effect,
    _normalize_effect_tree,
    _collapse_to_effect_steps,
)

TRIGGER_PATTERN = re.compile(r"\{\{([^|]+)\|([^}]+)\}\}")
# Also match patterns with / prefix like /{{...}}
SLASH_TRIGGER_PATTERN = re.compile(r"/\{\{([^|]+)\|([^}]+)\}\}")


def extract_trigger(text: str) -> tuple[list, int | None, str]:
    """Extract triggers and use limits from ability text and return (triggers, use_limit, effect)."""
    # Cost icon patterns to exclude from triggers.
    # IMPORTANT: Only filter as cost AFTER a trigger has been found.
    # If at the very start of text, treat as position/activation requirement,
    # not as a cost (e.g. {{live_start.png|ライブ開始時}}{{center.png|センター}}).
    cost_icon_patterns = [
        "icon_energy",
        "heart",
        "icon_blade",
        "icon_b_all",
        "icon_score",
    ]

    # Position icons that should NOT prevent trigger extraction.
    # These appear after trigger icons like {{live_start.png|ライブ開始時}}{{center.png|センター}}.
    # They are NOT costs -- they are activation position requirements.
    position_icon_patterns = ["center", "left", "right"]

    # Known trigger icon patterns (for debugging/validation)
    trigger_icon_patterns = [
        "kidou",
        "jidou",
        "toujyou",
        "live_start",
        "live_success",
        "live_end",
        "turn",
        "center",  # center can be both cost, trigger, and position requirement
    ]

    # Use limit patterns (turn restrictions)
    use_limit_patterns = ["turn", "ターン"]

    triggers = []
    use_limit = None
    # Strip leading/trailing quote chars that may be JSON artifacts
    text = text.strip().lstrip('"').lstrip("\u201c").lstrip("\u201d").strip()
    effect = text

    # Handle ［ターン1回］ bracket format (non-icon turn limit notation)
    bracket_turn_match = re.search(r"［ターン1回］", effect)
    if bracket_turn_match:
        use_limit = 1
        effect = effect.replace("［ターン1回］", "", 1).strip()

    # First, remove / prefix trigger patterns
    slash_matches = SLASH_TRIGGER_PATTERN.findall(text)
    for match in slash_matches:
        icon_file = match[0]
        icon_text = match[1]
        slash_pattern = f"/{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(slash_pattern, "", 1)
        triggers.append(icon_text)

    # Find all trigger patterns
    trigger_matches = TRIGGER_PATTERN.findall(text)

    # Only consider triggers at the very start (before any non-trigger, non-whitespace text)
    pos = 0
    for match in trigger_matches:
        icon_file = match[0]
        icon_text = match[1]
        match_start = text.find(f"{{{{{icon_file}|{icon_text}}}}}", pos)

        # Check if there's any non-trigger text before this match
        before = text[pos:match_start]
        if before.strip() and before.strip() != "：":
            # If the only non-trigger text is a slash prefix (from /{{trigger}}), skip it
            if before.strip() == "/":
                pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
                continue
            # Found non-trigger text, stop here
            break

        # Check if this is a cost icon (not a trigger).
        # BUT: if we already found a trigger, don't skip subsequent position icons
        # like {{center.png|センター}} -- they are activation position requirements,
        # not costs. Only skip actual cost resources (energy, heart, blade, score).
        if any(cost_pattern in icon_file for cost_pattern in cost_icon_patterns):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        # Position icons (center, left, right) at the start should be skipped
        # only if no trigger has been found yet AND no other trigger icon precedes.
        # If they appear after a trigger icon, they're position requirements, not costs.
        if triggers and any(p in icon_file for p in position_icon_patterns):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        # Check if this is a use limit (turn restriction)
        if any(
            use_limit_pattern in icon_text for use_limit_pattern in use_limit_patterns
        ):
            use_limit_text = icon_text
            # Convert Japanese turn limit text to integer
            if use_limit_text == "ターン1回":
                use_limit = 1
            elif use_limit_text == "ターン2回":
                use_limit = 2
            elif use_limit_text == "ターン3回":
                use_limit = 3
            else:
                # Try to extract any number from text like "ターンN回"
                num_match = re.match(r"ターン(\d+)回", use_limit_text)
                use_limit = int(num_match.group(1)) if num_match else use_limit_text
            # Remove use limit from effect
            trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
            effect = effect.replace(trigger_pattern, "", 1)
            pos = match_start + len(trigger_pattern)
            continue

        # Check if we're inside quoted text
        # Count quotes before this position
        quote_count = text[:match_start].count("「") - text[:match_start].count("」")
        if quote_count > 0:
            # We're inside quoted text, skip
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue

        # Skip if this trigger was already extracted (e.g. via slash prefix)
        if icon_text in triggers:
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue
        triggers.append(icon_text)
        # Remove this trigger icon from effect
        trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(trigger_pattern, "", 1)
        pos = match_start + len(trigger_pattern)

    effect = effect.strip()

    return triggers, use_limit, effect


def extract_abilities_from_card(card_id: str, card: dict) -> list:
    """Extract all abilities from a single card."""
    abilities = []

    ability_text = card.get("ability", "")
    if not ability_text:
        return abilities

    # Split by newline for multiple abilities
    ability_lines = ability_text.split("\n")

    for i, line in enumerate(ability_lines):
        line = line.strip()
        if not line:
            continue

        # Check if this is a continuation line (starts with ・)
        if line.startswith("・"):
            # Append to previous ability
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            continue

        # Check if this is a parenthetical note (wrapped in parentheses)
        # These should be appended to the previous ability
        if (line.startswith("(") and line.endswith(")")) or (
            line.startswith("（") and line.endswith("）")
        ):
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            else:
                # Standalone parenthetical note - treat as null ability (no-op)
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

        # Check if this line starts with a trigger pattern (new ability)
        # If it doesn't have a trigger but the previous ability had one, it might be a continuation
        triggers, use_limit, effect = extract_trigger(line)

        # Check if this is a continuation of a previous ability (no trigger, but previous had trigger)
        # This handles cases like "回答がチョコミントの場合、..." which are conditional outcomes
        if not triggers and abilities and abilities[-1]["trigger_count"] > 0:
            # Check if this looks like a conditional outcome (starts with "回答が" or similar patterns)
            if line.startswith("回答が") or line.startswith("場合") or "の場合" in line:
                # Append to previous ability
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
                continue
            # Check if this is a fragment continuation (starts with "とき、" or similar)
            # This handles cases where abilities are split incorrectly across newlines
            if (
                line.startswith("とき、")
                or line.startswith("なら、")
                or line.startswith("場合、")
            ):
                # Append to previous ability
                abilities[-1]["full_text"] += line
                abilities[-1]["triggerless_text"] += line
                continue

        # If this line has no trigger at all, treat it as a note (is_null: True)
        # Only parse as regular ability if it has trigger brackets
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


def _enrich_effect_type(effect, triggerless=""):
    """Extract heart colors from ability text. effect_type is NOT set here
    (the trigger field already implies whether it's continuous or triggered)."""
    if effect is None:
        return
    heart_colors = []
    seen = set()
    for m in re.findall(r"{{heart_(\d+)\.png\|heart\d+}}", triggerless):
        h = f"heart{m.zfill(2)}"
        if h not in seen:
            seen.add(h)
            heart_colors.append(h)
    if heart_colors and "heart_colors" not in effect:
        effect["heart_colors"] = heart_colors
    # Propagate heart_colors into location_condition for collective heart checks.
    # Skip check_self conditions -- they check a specific card's location, not
    # collective heart presence; heart_colors there is effect metadata leakage.
    if "heart_colors" in effect and "condition" in effect:
        cond = effect["condition"]
        if (
            isinstance(cond, dict)
            and cond.get("type") == "location_condition"
            and "heart_colors" not in cond
            and not cond.get("check_self")
        ):
            cond["heart_colors"] = effect["heart_colors"]


def extract_all_abilities(cards_file: Path) -> dict:
    """Extract all abilities from cards.json."""
    with open(cards_file, encoding="utf-8") as f:
        cards = json.load(f)

    all_abilities = []
    ability_groups = defaultdict(list)

    # Handle both dict and list formats
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

    # Group abilities by full_text
    unique_abilities = []
    for full_text, card_examples in ability_groups.items():
        sample = next(a for a in all_abilities if a["full_text"] == full_text)

        # Parse semantic effect and cost
        effect_text = sample["triggerless_text"]

        # Skip parsing for is_null abilities (notes without triggers)
        if sample.get("is_null", False):
            unique_abilities.append(
                {
                    "full_text": full_text,
                    "triggerless_text": sample["triggerless_text"],
                    "card_count": len(card_examples),
                    "cards": card_examples,
                    "triggers": ", ".join(sample["triggers"])
                    if sample["triggers"]
                    else None,
                    "use_limit": sample["use_limit"],
                    "is_null": True,
                    "cost": None,
                    "effect": None,
                }
            )
            continue

        # Split cost and effect
        cost_text = None
        if "：" in effect_text:
            parts = effect_text.split("：", 1)
            cost_text = parts[0].strip()
            effect_text = parts[1].strip()

        # Parse cost
        cost = None
        if cost_text:
            try:
                cost = parse_cost(cost_text)
            except:
                cost = None

        # Parse effect
        effect = {}
        try:
            effect = parse_effect(effect_text)
            # Run post-processing normalizer (propagates exclude_self, distinct, position, original_value, etc.)
            effect = _normalize_effect_tree(effect, sample["triggerless_text"])
            # Collapse the 4 specialized compound shapes (look_and_select,
            # conditional_alternative, conditional_on_result,
            # conditional_on_optional) into the unified `effect_steps` form
            # so the engine can dispatch them through the single sequential
            # pipeline. This eliminates per-shape code paths.
            effect = _collapse_to_effect_steps(effect)
            # Check if effect has empty actions array
            if "actions" in effect and not effect["actions"]:
                print(f"Warning: Effect parsed with empty actions: {effect_text[:100]}")
                print(f"Effect dict: {effect}")
            _enrich_effect_type(effect, triggerless=sample["triggerless_text"])

        except Exception as e:
            print(f"Error parsing effect: {effect_text}")
            print(f"Exception: {e}")
            import traceback

            traceback.print_exc()
            effect = {"text": effect_text, "actions": []}

        # If the effect handler embedded a cost (e.g. "unless pay N energy"),
        # lift it to the ability level (Q92: player chooses whether to pay)
        if isinstance(effect, dict) and "cost" in effect:
            cost = effect.pop("cost")

        unique_abilities.append(
            {
                "full_text": full_text,
                "triggerless_text": sample["triggerless_text"],
                "card_count": len(card_examples),
                "cards": card_examples,
                "triggers": ", ".join(sample["triggers"])
                if sample["triggers"]
                else None,
                "use_limit": sample["use_limit"],
                "is_null": sample.get("is_null", False),
                "cost": cost,
                "effect": effect,
            }
        )

    # Sort by card count
    unique_abilities.sort(key=lambda x: -x["card_count"])

    return {
        "schema": "extracted_abilities.v1",
        "generated_at": datetime.now().isoformat(),
        "generated_by": "tools/ability_extraction/extract_card_abilities.py",
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


def test_parsing():
    test_ability = "{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。"
    triggers, use_limit, effect = extract_trigger(test_ability)

    print("=== Test Parsing ===")
    print(f"Original: {test_ability}")
    print(f"Triggers: {triggers}")
    print(f"Use Limit: {use_limit}")
    print(f"Effect: {effect}")
    print()


def main():
    test_parsing()

    cards_file = Path(__file__).parent.parent / "cards.json"
    output_file = Path(__file__).parent.parent / "abilities.json"

    print(f"Extracting abilities from {cards_file}...")
    result = extract_all_abilities(cards_file)

    print(
        f"Found {result['statistics']['total_abilities']} abilities across {result['statistics']['cards_with_abilities']} cards"
    )
    print(f"Unique abilities: {result['statistics']['unique_abilities']}")

    # Post-process: infer actions, apply targeted fixes
    from parser import process_abilities

    result = process_abilities(result)

    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)

    print(f"Output written to {output_file}")

    # Run basic validation: check for known gap patterns
    _validate_output(result)
    print("Validation complete.")


def _validate_output(result):
    """Post-extraction validation -- check for known parser gaps in output."""
    import re
    from collections import Counter

    abilities = result["unique_abilities"]
    gaps = Counter()

    for a in abilities:
        t = a.get("triggerless_text", "")
        if not t:
            continue
        eff = a.get("effect") or {}
        cond = eff.get("condition") or {}

        # same_name
        if "同じ名前" in t:
            if not cond.get("same_name") and not eff.get("same_name"):
                gaps["same_name"] += 1
                continue

        # different card names
        if "カード名の異なる" in t:

            def _find_distinct(obj, depth=0):
                if depth > 10 or not isinstance(obj, dict):
                    return False
                if obj.get("distinct") == "card_name":
                    return True
                for v in obj.values():
                    if isinstance(v, dict):
                        if _find_distinct(v, depth + 1):
                            return True
                    elif isinstance(v, list):
                        for item in v:
                            if isinstance(item, dict) and _find_distinct(
                                item, depth + 1
                            ):
                                return True
                return False

            if not _find_distinct(eff):
                gaps["different_name"] += 1

        # or_location (zone1 + か + zone2)
        if re.search(r"(?:成功)?ライブカード置き場(?:か(?!ら)|又は)", t):
            locs = cond.get("locations", [])
            if len(locs) < 2:
                gaps["or_location"] += 1

        # heart_content
        if re.search(r"必要ハートに含まれる\{\{heart_\d+\.png\|heart\d+\}\}が\d+", t):
            if not cond.get("heart_colors") or not cond.get("count"):
                gaps["heart_content"] += 1

    if gaps:
        print("\n  GAPS DETECTED:")
        for gap, count in gaps.most_common():
            print(f"    {gap}: {count} cards")
        print("    These should be fixed in parser.py before committing.")
    else:
        print("    No gaps detected -- all known patterns handled.")

    # Group filter check: bracketed names in text should have group_names in JSON
    _validate_group_filters(abilities)


BRACKET_RE = re.compile(r"『([^』]+)』")
FILTER_FIELDS = {
    "group_names",
    "exclude_group_names",
    "characters",
    "exclude_characters",
}


def _walk_filters(obj):
    """Recursively walk JSON and return all (value, field_name) pairs from filter fields."""
    filters = set()
    if isinstance(obj, dict):
        for key, value in obj.items():
            if key in FILTER_FIELDS and isinstance(value, list):
                for item in value:
                    if isinstance(item, str):
                        filters.add((item, key))
            if isinstance(value, (dict, list)):
                filters.update(_walk_filters(value))
    elif isinstance(obj, list):
        for item in obj:
            if isinstance(item, (dict, list)):
                filters.update(_walk_filters(item))
    return filters


def _validate_group_filters(abilities):
    """Check that every bracketed 『X』 name in ability text has a corresponding
    group_names/characters filter somewhere in the JSON structure."""
    issues = []
    for a in abilities:
        text = a.get("full_text", "") + a.get("triggerless_text", "")
        bracketed = set(BRACKET_RE.findall(text))
        if not bracketed:
            continue

        # Normalize variants
        def variants(name):
            v = {name}
            if "!" in name:
                v.add(name.replace("!", "！"))
            if "！" in name:
                v.add(name.replace("！", "!"))
            if "µ" in name:
                v.add(name.replace("µ", "μ"))
            if "μ" in name:
                v.add(name.replace("μ", "µ"))
            return v

        filter_values = {fv for fv, _ in _walk_filters(a)}

        for name in bracketed:
            if not (variants(name) & filter_values):
                card_list = a.get("cards", [])
                text_preview = a.get("full_text", "")[:80]
                issues.append(
                    f'  『{name}』 in "{text_preview}…" — {card_list[0] if card_list else "?"}'
                )
                break  # one report per ability

    if issues:
        print(
            f"\n  GROUP FILTER ISSUES ({len(issues)} abilities with missing group_names):"
        )
        for issue in issues:
            safe = issue.encode("utf-8", errors="replace").decode("utf-8")
            print(safe)
    else:
        print("    All bracketed names have matching filter fields.")


if __name__ == "__main__":
    main()

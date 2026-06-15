import json
import re
from collections import defaultdict

CUSTOM_DISPATCH_RULES = [
    ("移動させる|移動する|移動し", "position_change"),
    ("エリアを選ぶ", "area_select"),
]


def find_custom_actions(obj, path=""):
    results = []
    if isinstance(obj, dict):
        action = obj.get("action")
        text = obj.get("text", "")
        if action == "custom":
            results.append((path, text, obj))
        elif action in ("sequential", "compound"):
            for key in ("actions", "primary_effect", "alternative_effect"):
                if key in obj:
                    for i, sub in enumerate(
                        obj[key] if isinstance(obj[key], list) else [obj[key]]
                    ):
                        results.extend(find_custom_actions(sub, f"{path}.{key}[{i}]"))
        for key in ("condition", "result_condition"):
            if key in obj:
                results.extend(find_custom_actions(obj[key], f"{path}.{key}"))
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            results.extend(find_custom_actions(item, f"{path}[{i}]"))
    return results


def find_area_selects(obj, path=""):
    results = []
    if isinstance(obj, dict):
        action = obj.get("action")
        text = obj.get("text", "")
        source = obj.get("source")
        heart_colors = obj.get("heart_colors")
        card_type = obj.get("card_type")
        or_card_types = obj.get("or_card_types")
        distinct = obj.get("distinct")
        if (
            action == "select"
            and source is None
            and not heart_colors
            and not or_card_types
            and not distinct
        ):
            if card_type != "member_card" or "エリア" in text:
                results.append((path, text, obj))
        for key in ("actions", "primary_effect", "alternative_effect"):
            if key in obj:
                for i, sub in enumerate(
                    obj[key] if isinstance(obj[key], list) else [obj[key]]
                ):
                    results.extend(find_area_selects(sub, f"{path}.{key}[{i}]"))
        for key in ("condition", "result_condition"):
            if key in obj:
                results.extend(find_area_selects(obj[key], f"{path}.{key}"))
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            results.extend(find_area_selects(item, f"{path}[{i}]"))
    return results


def main():
    with open("cards/abilities.json", encoding="utf-8") as f:
        data = json.load(f)

    entries = data.get("unique_abilities", [])
    total = 0
    custom_by_text = defaultdict(list)
    area_selects = []

    for entry in entries:
        # The "effect" key holds the parsed effect
        effect = entry.get("effect", {})
        cards = entry.get("cards", [])
        card_label = cards[0] if cards else "unknown"

        if not effect:
            continue

        customs = find_custom_actions(effect)
        for path, text, eff in customs:
            total += 1
            # Normalize for grouping
            text_clean = text.strip()[:60] or "(empty text)"
            custom_by_text[text_clean].append((card_label, path, text, eff))

        areas = find_area_selects(effect)
        for path, text, eff in areas:
            area_selects.append((card_label, path, text, eff))

    print(f"=== Custom Actions Report ===")
    print(f"Total 'custom' actions found: {total}")
    print()

    if custom_by_text:
        sorted_items = sorted(custom_by_text.items(), key=lambda x: -len(x[1]))
        print(f"{'Text fragment':<55} | {'Count':>5}")
        print("-" * 63)
        for text_clean, items in sorted_items:
            print(f"{text_clean:<55} | {len(items):>5}")
            for card_label, path, text, eff in items[:2]:
                print(f"    {card_label}")
                print(f"    path={path}")
            if len(items) > 2:
                print(f"    ... and {len(items) - 2} more")
            print()
    else:
        print("No custom actions found! All patterns matched.")
        print()

    print(
        f"=== Area Select Candidates (select with no source, looks like area selection) ==="
    )
    print(f"Total: {len(area_selects)}")
    print()
    if area_selects:
        for card_label, path, text, eff in area_selects:
            print(f"  {card_label}")
            print(f"  text: {text[:80]}")
            print(f"  path: {path}")
            print()


if __name__ == "__main__":
    main()

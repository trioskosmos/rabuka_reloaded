#!/usr/bin/env python3
"""Check abilities.json for missing group_names/characters filters.

Scans every ability entry in abilities.json. For each bracketed name 『X』 in the
text, it recursively walks the JSON structure to see if that name appears in any
filter field (group_names, exclude_group_names, characters, exclude_characters).
Reports mismatches.
"""

import json
import re
import sys
from collections import defaultdict

# Load abilities
with open("abilities.json", "r", encoding="utf-8") as f:
    data = json.load(f)

abilities = data["unique_abilities"]

# Extract all bracketed names from text
BRACKET_RE = re.compile(r"『([^』]+)』")

# Fields that contain filter names
FILTER_FIELDS = {
    "group_names",
    "exclude_group_names",
    "characters",
    "exclude_characters",
}


def find_bracketed(text: str) -> set:
    """Find all bracketed names in text."""
    return set(BRACKET_RE.findall(text))


def walk_for_filters(obj, depth=0):
    """Recursively walk a JSON object and collect all filter field values.

    Returns a set of all filter values found, plus the paths to them.
    Also returns a set of all condition_types found (to understand context).
    """
    filters = set()
    condition_types = set()

    if isinstance(obj, dict):
        for key, value in obj.items():
            if key in FILTER_FIELDS and isinstance(value, list):
                for item in value:
                    if isinstance(item, str):
                        filters.add((item, key))
            if key == "condition_type" and isinstance(value, str):
                condition_types.add(value)
            if key == "condition" and isinstance(value, (dict, list)):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
            if key == "effect" and isinstance(value, (dict, list)):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
            if key == "cost" and isinstance(value, (dict, list)):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
            if key == "select_action" and isinstance(value, dict):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
            if key == "look_action" and isinstance(value, dict):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
            if key == "followup_action" and isinstance(value, dict):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
            if key == "primary_effect" and isinstance(value, dict):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
            if key == "actions" and isinstance(value, list):
                for action in value:
                    sub_filters, sub_ct = walk_for_filters(action, depth + 1)
                    filters.update(sub_filters)
                    condition_types.update(sub_ct)
            if key == "effect_steps" and isinstance(value, list):
                for step in value:
                    sub_filters, sub_ct = walk_for_filters(step, depth + 1)
                    filters.update(sub_filters)
                    condition_types.update(sub_ct)
            if key == "options" and isinstance(value, list):
                for opt in value:
                    sub_filters, sub_ct = walk_for_filters(opt, depth + 1)
                    filters.update(sub_filters)
                    condition_types.update(sub_ct)
            # Check nested objects recursively for ALL keys
            if isinstance(value, (dict, list)):
                sub_filters, sub_ct = walk_for_filters(value, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)
    elif isinstance(obj, list):
        for item in obj:
            if isinstance(item, (dict, list)):
                sub_filters, sub_ct = walk_for_filters(item, depth + 1)
                filters.update(sub_filters)
                condition_types.update(sub_ct)

    return filters, condition_types


def text_might_need_filter(text: str) -> bool:
    """Check if text likely describes a filterable condition.

    Return False for texts that are purely informational or about
    the ability's own properties (like activation conditions, use limits, etc.).
    """
    # Texts about the card's own properties don't need filters for bracketed names
    # that appear in the text as descriptions rather than filter criteria.
    no_filter_keywords = [
        "カード名に",  # "card name includes" - different filter mechanism
    ]
    for kw in no_filter_keywords:
        if kw in text:
            return False
    return True


# Results tracking
total_abilities = len(abilities)
issues = []
issues_by_name = defaultdict(list)
abilities_with_bracketed = 0

for idx, ab in enumerate(abilities):
    full_text = ab.get("full_text", "")
    triggerless_text = ab.get("triggerless_text", "")

    # Strip template markers {{...}} from full_text for bracketed name search
    # But also search triggerless_text which has fewer markers
    cleaned_text = re.sub(r"\{\{[^}]*\}\}", "", full_text)

    # Find ALL bracketed names in both texts
    bracketed_full = find_bracketed(full_text)
    bracketed_triggerless = find_bracketed(triggerless_text)
    bracketed_cleaned = find_bracketed(cleaned_text)

    all_bracketed = bracketed_full | bracketed_triggerless | bracketed_cleaned

    if not all_bracketed:
        continue

    abilities_with_bracketed += 1

    # Walk the JSON structure to find all filters
    filters_found, condition_types = walk_for_filters(ab)

    # Collect all unique filter values and the fields they came from
    filter_values = set()  # just the strings
    field_map = defaultdict(set)  # string -> set of field names

    if isinstance(ab, dict):
        # Also check fields at the top level (condition, effect, cost may be top-level keys)
        pass  # already checked by walk_for_filters

    for val, field in filters_found:
        filter_values.add(val)
        field_map[val].add(field)

    # Check each bracketed name
    for name in all_bracketed:
        # Normalize: some names have variant forms
        name_normalized = name

        # Check if name appears in any filter field
        # Also check normalized variants (e.g., fullwidth/halfwidth)
        name_variants = {name}
        # Add normalized variants
        if "!" in name:
            name_variants.add(name.replace("!", "！"))
        if "！" in name:
            name_variants.add(name.replace("！", "!"))
        if "µ" in name:
            name_variants.add(name.replace("µ", "μ"))
        if "μ" in name:
            name_variants.add(name.replace("μ", "µ"))

        found = bool(name_variants & filter_values)

        if not found:
            # Check if the bracketed name might be handled by a different mechanism:
            # - References to card name (part of the ability's own name)
            # - Part of a description like "『スクールアイドル』" which might be card_type
            # - Card name in ability description like "『小原鞠莉』"

            issues.append(
                {
                    "index": idx,
                    "name": name,
                    "full_text": full_text[:120],
                    "triggerless_text": triggerless_text[:120],
                    "cards": ab.get("cards", [])[:3],
                    "condition_types": sorted(condition_types)
                    if condition_types
                    else [],
                    "action": ab.get("effect", {}).get("action", ab.get("action", "?")),
                    "filters_present": sorted(field_map.keys()) if field_map else [],
                }
            )
            issues_by_name[name].append(idx)

print(f"\n{'=' * 80}")
print(f"ABILITY GROUP FILTER CHECK REPORT")
print(f"{'=' * 80}")
print(f"Total unique abilities: {total_abilities}")
print(f"Abilities with bracketed names: {abilities_with_bracketed}")
print(f"Abilities with missing filters: {len(issues)}")
print(f"{'=' * 80}\n")

if issues:
    # Group issues by name
    print("\n=== ISSUES BY BRACKETED NAME ===\n")
    for name in sorted(
        issues_by_name.keys(), key=lambda n: (-len(issues_by_name[n]), n)
    ):
        name_issues = issues_by_name[name]
        print(f"\n--- 『{name}』: {len(name_issues)} issue(s) ---")
        for iss in name_issues:
            # Get the actual issue data
            pass  # We'll print in detail below

    print(f"\n\n=== DETAILED ISSUE LIST ===\n")
    for i, iss in enumerate(issues):
        print(f"[{i + 1}] Bracketed name: 『{iss['name']}』")
        print(f"    Text: {iss['full_text'][:100]}...")
        print(f"    Cards: {', '.join(iss['cards'][:3])}")
        print(f"    Action: {iss['action']}")
        print(f"    Condition types: {iss['condition_types']}")
        print(f"    Filters present: {iss['filters_present'][:5]}")
        print()

    # Summary by name
    print(f"\n{'=' * 80}")
    print("SUMMARY BY NAME")
    print(f"{'=' * 80}")
    for name in sorted(
        issues_by_name.keys(), key=lambda n: (-len(issues_by_name[n]), n)
    ):
        ies = issues_by_name[name]
        print(f"  『{name}』: {len(ies)} missing")
        for iss_idx in ies[:5]:
            iss = next(i for i in issues if i["index"] == iss_idx)
            print(f"    - {iss['full_text'][:80]}...")
        if len(ies) > 5:
            print(f"    ... and {len(ies) - 5} more")
else:
    print("No issues found! All bracketed names have corresponding filter fields.")

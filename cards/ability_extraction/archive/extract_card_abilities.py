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
    parse_ability,
    process_abilities,
)

TRIGGER_PATTERN = re.compile(r'\{\{([^|]+)\|([^}]+)\}\}')
# Also match patterns with / prefix like /{{...}}
SLASH_TRIGGER_PATTERN = re.compile(r'/\{\{([^|]+)\|([^}]+)\}\}')


def extract_trigger(text: str) -> tuple[list, str, str]:
    """Extract triggers and use limits from ability text and return (triggers, use_limit, effect)."""
    # Cost icon patterns to exclude from triggers
    cost_icon_patterns = [
        'icon_energy', 'heart', 'icon_blade', 'icon_b_all', 'icon_score', 'center'
    ]
    
    # Known trigger icon patterns (for debugging/validation)
    trigger_icon_patterns = [
        'kidou', 'jidou', 'toujyou', 'live_start', 'live_success', 
        'live_end', 'turn', 'center'  # center can be both cost and trigger
    ]
    
    # Use limit patterns (turn restrictions)
    use_limit_patterns = [
        'turn', 'ターン'
    ]
    
    triggers = []
    use_limit = None
    effect = text
    
    # First, remove / prefix trigger patterns
    slash_matches = SLASH_TRIGGER_PATTERN.findall(text)
    for match in slash_matches:
        icon_file = match[0]
        icon_text = match[1]
        slash_pattern = f"/{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(slash_pattern, '', 1)
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
        if before.strip() and before.strip() != '：':
            # Found non-trigger text, stop here
            break
        
        # Check if this is a cost icon (not a trigger)
        if any(cost_pattern in icon_file for cost_pattern in cost_icon_patterns):
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue
        
        # Check if this is a use limit (turn restriction)
        if any(use_limit_pattern in icon_file for use_limit_pattern in use_limit_patterns):
            use_limit = icon_text
            # Set once_per_turn if use_limit is ターン1回
            if use_limit == "ターン1回":
                use_limit = 1  # Convert to integer
            # Remove use limit from effect
            trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
            effect = effect.replace(trigger_pattern, '', 1)
            pos = match_start + len(trigger_pattern)
            continue
        
        # Check if we're inside quoted text
        # Count quotes before this position
        quote_count = text[:match_start].count('「') - text[:match_start].count('」')
        if quote_count > 0:
            # We're inside quoted text, skip
            pos = match_start + len(f"{{{{{icon_file}|{icon_text}}}}}")
            continue
        
        triggers.append(icon_text)
        # Remove this trigger icon from effect
        trigger_pattern = f"{{{{{icon_file}|{icon_text}}}}}"
        effect = effect.replace(trigger_pattern, '', 1)
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
    ability_lines = ability_text.split('\n')
    
    for i, line in enumerate(ability_lines):
        line = line.strip()
        if not line:
            continue
        
        # Check if this is a continuation line (starts with ・)
        if line.startswith('・'):
            # Append to previous ability
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            continue
        
        # Check if this is a parenthetical note (wrapped in parentheses)
        # These should be appended to the previous ability
        if (line.startswith('(') and line.endswith(')')) or (line.startswith('（') and line.endswith('）')):
            if abilities:
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
            else:
                # Standalone parenthetical note - treat as null ability (no-op)
                abilities.append({
                    "card_id": card_id,
                    "full_text": line,
                    "triggerless_text": "",
                    "use_limit": None,
                    "triggers": [],
                    "trigger_count": 0,
                    "ability_index": i,
                    "is_null": True,
                })
            continue
        
        # Check if this line starts with a trigger pattern (new ability)
        # If it doesn't have a trigger but the previous ability had one, it might be a continuation
        triggers, use_limit, effect = extract_trigger(line)
        
        # Check if this is a continuation of a previous ability (no trigger, but previous had trigger)
        # This handles cases like "回答がチョコミントの場合、..." which are conditional outcomes
        if not triggers and abilities and abilities[-1]["trigger_count"] > 0:
            # Check if this looks like a conditional outcome (starts with "回答が" or similar patterns)
            if line.startswith('回答が') or line.startswith('場合') or 'の場合' in line:
                # Append to previous ability
                abilities[-1]["full_text"] += "\n" + line
                abilities[-1]["triggerless_text"] += "\n" + line
                continue
        
        abilities.append({
            "card_id": card_id,
            "full_text": line,
            "triggerless_text": effect,
            "use_limit": use_limit,
            "once_per_turn": use_limit == 1 if use_limit else False,
            "triggers": triggers,
            "trigger_count": len(triggers),
            "ability_index": i,
        })
    
    return abilities


def extract_all_abilities(cards_file: Path) -> dict:
    """Extract all abilities from cards.json."""
    with open(cards_file, encoding='utf-8') as f:
        cards = json.load(f)
    
    all_abilities = []
    ability_groups = defaultdict(list)
    
    # Handle both dict and list formats
    if isinstance(cards, list):
        cards_dict = {card.get('card_no', str(i)): card for i, card in enumerate(cards)}
    else:
        cards_dict = cards
    
    for card_id, card in cards_dict.items():
        abilities = extract_abilities_from_card(card_id, card)
        for ability in abilities:
            all_abilities.append(ability)
            card_example = f"{card_id} | {card.get('name', '')} (ab#{ability['ability_index']})"
            ability_groups[ability["full_text"]].append(card_example)
    
    # Group abilities by full_text
    unique_abilities = []
    for full_text, card_examples in ability_groups.items():
        sample = next(a for a in all_abilities if a["full_text"] == full_text)
        
        # Parse semantic effect and cost
        effect_text = sample["triggerless_text"]
        
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
        except:
            effect = {"text": effect_text, "actions": []}
        
        unique_abilities.append({
            "full_text": full_text,
            "triggerless_text": sample["triggerless_text"],
            "card_count": len(card_examples),
            "cards": card_examples,
            "triggers": ', '.join(sample["triggers"]) if sample["triggers"] else None,
            "use_limit": sample["use_limit"],
            "is_null": sample.get("is_null", False),
            "cost": cost,
            "effect": effect,
        })
    
    # Sort by card count
    unique_abilities.sort(key=lambda x: -x["card_count"])
    
    return {
        "schema": "extracted_abilities.v1",
        "generated_at": datetime.now().isoformat(),
        "generated_by": "tools/ability_extraction/extract_card_abilities.py",
        "source_file": str(cards_file),
        "statistics": {
            "total_cards": len(cards_dict),
            "cards_with_abilities": len([c for c in cards_dict.values() if c.get("ability")]),
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
    
    cards_file = Path(__file__).parent.parent.parent / "cards.json"
    output_file = Path(__file__).parent.parent.parent / "abilities.json"
    
    print(f"Extracting abilities from {cards_file}...")
    result = extract_all_abilities(cards_file)
    
    print(f"Found {result['statistics']['total_abilities']} abilities across {result['statistics']['cards_with_abilities']} cards")
    print(f"Unique abilities: {result['statistics']['unique_abilities']}")
    
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    
    print(f"Output written to {output_file}")


if __name__ == "__main__":
    main()

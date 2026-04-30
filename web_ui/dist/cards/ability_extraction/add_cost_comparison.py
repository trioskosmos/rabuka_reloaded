#!/usr/bin/env python3
"""
Script to add cost_comparison field to abilities.json based on Japanese text patterns.
"""

import json
import re

def determine_cost_comparison(text):
    """Determine cost comparison type from Japanese text."""
    # Check for 以上 (or more)
    if re.search(r'\d+以上|以上\d+', text):
        return "min"
    # Check for 以下 (or less)
    elif re.search(r'\d+以下|以下\d+', text):
        return "max"
    # Check for 未満 (less than)
    elif re.search(r'\d+未満|未満\d+', text):
        return "below"
    # Check for 超 (more than)
    elif re.search(r'\d+超|超\d+', text):
        return "above"
    return None

def process_effect(effect, full_text):
    """Process an effect to add cost_comparison if cost_limit exists."""
    if effect.get('cost_limit') is not None and 'cost_comparison' not in effect:
        comparison = determine_cost_comparison(effect.get('text', ''))
        if comparison:
            effect['cost_comparison'] = comparison
            print(f"Added cost_comparison: {comparison} for: {effect['text'][:60]}...")
    
    # Recursively process nested effects
    if 'actions' in effect and isinstance(effect['actions'], list):
        for action in effect['actions']:
            process_effect(action, full_text)
    if 'options' in effect and isinstance(effect['options'], list):
        for option in effect['options']:
            process_effect(option, full_text)
    if 'condition' in effect and isinstance(effect['condition'], dict):
        if 'cost_limit' in effect['condition'] and 'cost_comparison' not in effect['condition']:
            comparison = determine_cost_comparison(effect['condition'].get('text', full_text))
            if comparison:
                effect['condition']['cost_comparison'] = comparison

def main():
    with open('abilities.json', 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    updated_count = 0
    for ability in data['unique_abilities']:
        full_text = ability.get('full_text', '')
        
        if 'effect' in ability and ability['effect']:
            effect = ability['effect']
            if effect.get('cost_limit') is not None and 'cost_comparison' not in effect:
                comparison = determine_cost_comparison(effect.get('text', full_text))
                if comparison:
                    effect['cost_comparison'] = comparison
                    updated_count += 1
            
            # Process nested structures
            process_effect(effect, full_text)
        
        # Also check cost in condition
        if 'condition' in ability and isinstance(ability['condition'], dict):
            if 'cost_limit' in ability['condition'] and 'cost_comparison' not in ability['condition']:
                comparison = determine_cost_comparison(ability['condition'].get('text', full_text))
                if comparison:
                    ability['condition']['cost_comparison'] = comparison
    
    with open('abilities.json', 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    
    print(f"Updated {updated_count} abilities with cost_comparison field")

if __name__ == '__main__':
    main()

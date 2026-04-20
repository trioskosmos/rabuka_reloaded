#!/usr/bin/env python3
"""
List all abilities for manual analysis.
Outputs each ability with its parsed components for review.
"""

import json
from pathlib import Path

def analyze_abilities(abilities_file: Path):
    """Analyze and list all abilities."""
    with open(abilities_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    unique_abilities = data['unique_abilities']
    
    print(f"Total unique abilities: {len(unique_abilities)}")
    print(f"Total abilities across all cards: {data['statistics']['total_abilities']}")
    print(f"Cards with abilities: {data['statistics']['cards_with_abilities']}")
    print("\n" + "=" * 100)
    
    for i, ability in enumerate(unique_abilities, 1):
        print(f"\n{'=' * 100}")
        print(f"ABILITY #{i}")
        print(f"{'=' * 100}")
        print(f"Card count: {ability['card_count']}")
        print(f"Cards: {', '.join(ability['cards'][:3])}{'...' if len(ability['cards']) > 3 else ''}")
        print(f"\nFull text:\n{ability['full_text']}")
        
        if ability.get('is_null'):
            print(f"\n*** NULL ABILITY (Note) ***")
            continue
        
        print(f"\nTriggers: {ability.get('triggers', 'None')}")
        print(f"Use limit: {ability.get('use_limit', 'None')}")
        
        if ability.get('cost'):
            cost = ability['cost']
            print(f"\nCOST:")
            print(f"  Text: {cost.get('text', 'N/A')}")
            print(f"  Type: {cost.get('type', 'N/A')}")
            print(f"  Source: {cost.get('source', 'N/A')}")
            print(f"  Destination: {cost.get('destination', 'N/A')}")
            print(f"  Card type: {cost.get('card_type', 'N/A')}")
            print(f"  Count: {cost.get('count', 'N/A')}")
            print(f"  Optional: {cost.get('optional', False)}")
            if cost.get('energy'):
                print(f"  Energy: {cost.get('energy')}")
        
        if ability.get('effect'):
            effect = ability['effect']
            print(f"\nEFFECT:")
            print(f"  Text: {effect.get('text', 'N/A')}")
            print(f"  Action: {effect.get('action', 'N/A')}")
            
            if effect.get('source'):
                print(f"  Source: {effect.get('source')}")
            if effect.get('destination'):
                print(f"  Destination: {effect.get('destination')}")
            if effect.get('count'):
                print(f"  Count: {effect.get('count')}")
            if effect.get('card_type'):
                print(f"  Card type: {effect.get('card_type')}")
            if effect.get('target'):
                print(f"  Target: {effect.get('target')}")
            
            if effect.get('condition'):
                print(f"  Condition: {effect.get('condition')}")
            
            if effect.get('choice_modifier'):
                print(f"  Choice modifier: {effect.get('choice_modifier')}")
            
            if effect.get('options'):
                print(f"  Options ({len(effect.get('options'))}):")
                for j, option in enumerate(effect.get('options'), 1):
                    print(f"    Option {j}: {option.get('action', 'N/A')} - {option.get('text', 'N/A')[:80]}...")
            
            if effect.get('actions'):
                print(f"  Actions ({len(effect.get('actions'))}):")
                for j, action in enumerate(effect.get('actions'), 1):
                    print(f"    Action {j}: {action.get('action', 'N/A')} - {action.get('text', 'N/A')[:80]}...")
        
        # Check for missing information
        missing = []
        if not ability.get('triggers') and not ability.get('is_null'):
            missing.append('triggers')
        if ability.get('cost') and ability['cost'].get('type') == 'custom':
            missing.append('cost type is custom')
        if ability.get('effect') and ability['effect'].get('action') == 'custom':
            missing.append('effect action is custom')
        
        if missing:
            print(f"\n*** MISSING/ISSUES: {', '.join(missing)} ***")
        
        input("\nPress Enter to continue to next ability...")

if __name__ == '__main__':
    abilities_file = Path(__file__).parent.parent / 'abilities.json'
    analyze_abilities(abilities_file)

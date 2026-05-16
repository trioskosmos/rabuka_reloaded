import json
import os
from pathlib import Path

def normalize(val):
    if val is None: return None
    if isinstance(val, list):
        return sorted([normalize(x) for x in val])
    if isinstance(val, dict):
        return {k: normalize(v) for k, v in val.items() if v is not None}
    return val

def compare():
    my_file = Path("test_parser/real_abilities.json")
    original_file = Path("cards/abilities.json")

    if not my_file.exists() or not original_file.exists():
        print("Files missing.")
        return

    with open(my_file, encoding='utf-8') as f:
        my_data = json.load(f)
    
    with open(original_file, encoding='utf-8') as f:
        original_data = json.load(f)

    my_map = {a['full_text']: a['parsed'] for a in my_data['abilities']}
    original_map = {a['full_text']: a for a in original_data['unique_abilities']}

    diffs = []
    common_texts = set(my_map.keys()) & set(original_map.keys())

    print(f"Comparing {len(common_texts)} common abilities...")

    for text in list(common_texts)[:20]: # Show first 20 for brief summary
        my_p = my_map[text]
        orig_p = original_map[text]
        
        # Example comparison: cost type and count
        my_cost = my_p.get('cost')
        orig_cost = orig_p.get('cost')
        
        # This is a very complex comparison since schemas differ.
        # I'll just print a few to show the "flavor" of differences.
        print(f"\nAbility: {text[:50]}...")
        print(f"  MY COST: {my_cost}")
        print(f"  ORIG COST: {orig_cost}")
        
    # Full count of matches/mismatches in a specific key
    cost_matches = 0
    for text in common_texts:
        my_p = my_map[text]
        orig_p = original_map[text]
        
        my_cost = my_p.get('cost')
        orig_cost = orig_p.get('cost')
        
        my_cost_type = None
        if isinstance(my_cost, dict):
            my_cost_type = my_cost.get('type')
        elif isinstance(my_cost, list) and my_cost:
            my_cost_type = "sequential_cost" if len(my_cost) > 1 else my_cost[0].get('type')

        orig_cost_type = None
        if isinstance(orig_cost, dict):
            orig_cost_type = orig_cost.get('type')
        
        if my_cost_type == orig_cost_type:
            cost_matches += 1

    print(f"\nCost Type Matches: {cost_matches}/{len(common_texts)}")

if __name__ == "__main__":
    compare()

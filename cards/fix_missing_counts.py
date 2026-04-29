import json
import sys

# Load abilities.json
with open(r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

def fix_missing_counts(obj, path=""):
    """Recursively fix missing count/dynamic_count in actions"""
    if isinstance(obj, dict):
        action = obj.get('action')
        
        # Check if this is a move_cards action with missing count
        if action == 'move_cards':
            count = obj.get('count')
            dynamic_count = obj.get('dynamic_count')
            source = obj.get('source', '')
            destination = obj.get('destination', '')
            any_number = obj.get('any_number', False)
            
            # Fix missing count based on source/destination patterns
            if count is None and dynamic_count is None:
                # Special sources that should have dynamic counts
                if source == 'looked_at_remaining':
                    obj['dynamic_count'] = {
                        'type': 'RemainingLookedAt'
                    }
                    print(f"Fixed {path}: Added dynamic_count for looked_at_remaining")
                elif any_number:
                    obj['dynamic_count'] = {
                        'type': 'PlayerChoice'
                    }
                    print(f"Fixed {path}: Added dynamic_count for any_number=true")
                elif source in ['selected_cards', 'revealed_cards']:
                    obj['dynamic_count'] = {
                        'type': 'RevealedCards'
                    }
                    print(f"Fixed {path}: Added dynamic_count for {source}")
                else:
                    # Default to 1 for other cases
                    obj['count'] = 1
                    print(f"Fixed {path}: Set default count=1 for {source}->{destination}")
        
        # Recurse into nested structures
        for key, value in obj.items():
            if key in ['actions', 'look_action', 'select_action', 'effect']:
                if isinstance(value, dict):
                    fix_missing_counts(value, f"{path}.{key}")
                elif isinstance(value, list):
                    for i, item in enumerate(value):
                        fix_missing_counts(item, f"{path}.{key}[{i}]")
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            fix_missing_counts(item, f"{path}[{i}]")

# Apply fixes
print("Fixing missing count/dynamic_count...")
fixes_applied = 0
original_count = len(abilities)

for ability in abilities:
    if ability.get('effect'):
        fix_missing_counts(ability['effect'], f"ability[{abilities.index(ability)}].effect")
    if ability.get('cost'):
        fix_missing_counts(ability['cost'], f"ability[{abilities.index(ability)}].cost")

# Save the fixed data
with open(r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'w', encoding='utf-8') as f:
    json.dump(data, f, ensure_ascii=False, indent=2)

print(f"\n✅ Fixed missing count/dynamic_count in abilities.json")
print(f"   Total abilities: {original_count}")
print(f"   File updated successfully")

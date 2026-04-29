import json

# Load abilities.json
with open(r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# All action types found in abilities.json
found_actions = set()

def collect_actions(obj, path=""):
    """Recursively collect all action types from JSON structure"""
    if isinstance(obj, dict):
        action = obj.get('action')
        if action:
            found_actions.add(action)
        
        # Recurse into nested structures
        for key, value in obj.items():
            if key in ['actions', 'look_action', 'select_action', 'effect']:
                if isinstance(value, dict):
                    collect_actions(value, f"{path}.{key}")
                elif isinstance(value, list):
                    for i, item in enumerate(value):
                        collect_actions(item, f"{path}.{key}[{i}]")
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            collect_actions(item, f"{path}[{i}]")

# Collect all action types
for ability in abilities:
    if ability.get('effect'):
        collect_actions(ability['effect'], f"ability[{abilities.index(ability)}].effect")
    if ability.get('cost'):
        collect_actions(ability['cost'], f"ability[{abilities.index(ability)}].cost")

# Engine handlers from executor.rs analysis
engine_handlers = {
    # Fully implemented
    'move_cards': True,
    'draw': True,
    'draw_card': True,
    'gain_resource': True,
    'modify_score': True,
    'modify_required_hearts': True,
    'set_required_hearts': True,
    'modify_required_hearts_global': True,
    'set_blade_type': True,
    'set_heart_type': True,
    'position_change': True,
    'place_energy_under_member': True,
    'modify_yell_count': True,
    'look_at': True,
    'sequential': True,
    'discard_until_count': True,
    'conditional_alternative': True,
    
    # Placeholder only (not implemented)
    'specify_heart_color': False,
    'reveal': False,
    'gain_ability': False,
    'select': False,
    'choice': False,
    'activation_cost': False,
    'shuffle': False,
    'draw_until_count': False,
    'appear': False,
    'modify_cost': False,
    'set_score': False,
    'set_cost': False,
    'set_blade_count': False,
    'modify_limit': False,
    'invalidate_ability': False,
    'choose_heart_type': False,
    'modify_required_hearts_success': False,
    'set_cost_to_use': False,
    'all_blade_timing': False,
    'set_card_identity_all_regions': False,
    'custom': False,
    're_yell': False,
    'restriction': False,
    'activation_restriction': False,
    'set_card_identity': False,
}

print("="*80)
print("ENGINE HANDLER ANALYSIS")
print("="*80)

# Sort actions by implementation status
missing_handlers = []
placeholder_handlers = []
implemented_handlers = []

for action in sorted(found_actions):
    if action not in engine_handlers:
        missing_handlers.append(action)
    elif not engine_handlers[action]:
        placeholder_handlers.append(action)
    else:
        implemented_handlers.append(action)

print(f"\n✅ FULLY IMPLEMENTED ({len(implemented_handlers)})")
for action in implemented_handlers:
    print(f"  {action}")

print(f"\n⚠️ PLACEHOLDER ONLY ({len(placeholder_handlers)})")
for action in placeholder_handlers:
    print(f"  {action} (just logs, no actual logic)")

print(f"\n❌ COMPLETELY MISSING ({len(missing_handlers)})")
for action in missing_handlers:
    print(f"  {action}")

print(f"\n" + "="*80)
print("SUMMARY")
print("="*80)
print(f"Total unique action types found: {len(found_actions)}")
print(f"Fully implemented: {len(implemented_handlers)}")
print(f"Placeholder only: {len(placeholder_handlers)}")
print(f"Completely missing: {len(missing_handlers)}")
print(f"Need implementation: {len(placeholder_handlers) + len(missing_handlers)}")

# Critical issues
print(f"\n🔴 CRITICAL ISSUES:")
critical_actions = ['look_and_select']
for action in critical_actions:
    if action in missing_handlers:
        print(f"  {action} - Used in many abilities but completely missing!")

print(f"\n📝 PLACEHOLDER ACTIONS (need full implementation):")
for action in placeholder_handlers:
    count = sum(1 for ability in abilities 
                if any(action == sub_action.get('action') 
                       for sub_action in [ability.get('effect', {})] 
                       if isinstance(sub_action, dict)))
    print(f"  {action} - Used in {count} abilities")

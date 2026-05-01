"""Fix identified issues in abilities.json"""
import json
import sys

abilities_path = r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json'

with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
fixes_made = []

# Action type mappings - map invalid actions to valid ones
ACTION_MAPPINGS = {
    'set_heart_type': 'gain_resource',
    'set_card_identity_all_regions': 'set_card_identity',
    'modify_required_hearts_global': 'custom',
    'activation_cost': 'custom',
    'modify_required_hearts': 'custom',
}

# Actions that should have counts
count_required_actions = {
    'move_cards', 'draw_card', 'gain_resource', 'change_state', 
    'place_energy_under_member', 'set_card_identity', 'reveal',
    'look_and_select', 'select', 'discard_until_count', 'draw_until_count'
}

def fix_action_type(action_obj, ability_idx, path):
    """Fix invalid action types"""
    action = action_obj.get('action', '')
    if action in ACTION_MAPPINGS:
        old_action = action
        action_obj['action'] = ACTION_MAPPINGS[action]
        fixes_made.append(f'{ability_idx} {path}: Changed action "{old_action}" -> "{action_obj["action"]}"')
        return True
    return False

def fix_count(action_obj, ability_idx, path):
    """Fix missing counts"""
    action = action_obj.get('action', '')
    count = action_obj.get('count')
    dynamic_count = action_obj.get('dynamic_count')
    
    # Skip if already has count or dynamic_count
    if count is not None or dynamic_count:
        return False
    
    # For actions that need count, set a default of 1
    if action in count_required_actions:
        action_obj['count'] = 1
        fixes_made.append(f'{ability_idx} {path}: Added missing count=1 for action "{action}"')
        return True
    
    return False

def fix_missing_source_in_cost(ability, ability_idx):
    """Fix missing source in cost"""
    cost = ability.get('cost')
    if cost and cost.get('type') == 'move_cards':
        if not cost.get('source'):
            # Try to infer source from text
            text = cost.get('text', '')
            if '手札' in text:
                cost['source'] = 'hand'
                fixes_made.append(f'{ability_idx} cost: Added missing source="hand"')
                return True
            elif '控え室' in text:
                cost['source'] = 'discard'
                fixes_made.append(f'{ability_idx} cost: Added missing source="discard"')
                return True
            elif 'デッキ' in text:
                cost['source'] = 'deck'
                fixes_made.append(f'{ability_idx} cost: Added missing source="deck"')
                return True
            elif 'ステージ' in text:
                cost['source'] = 'stage'
                fixes_made.append(f'{ability_idx} cost: Added missing source="stage"')
                return True
    return False

def process_action(action_obj, ability_idx, path='effect'):
    """Recursively process action objects"""
    if not isinstance(action_obj, dict):
        return
    
    # Fix this action
    fix_action_type(action_obj, ability_idx, path)
    fix_count(action_obj, ability_idx, path)
    
    # Process nested actions
    if 'actions' in action_obj and isinstance(action_obj['actions'], list):
        for i, sub in enumerate(action_obj['actions']):
            process_action(sub, ability_idx, f'{path}.actions[{i}]')
    
    # Process look_action / select_action
    if 'look_action' in action_obj:
        process_action(action_obj['look_action'], ability_idx, f'{path}.look_action')
    if 'select_action' in action_obj:
        process_action(action_obj['select_action'], ability_idx, f'{path}.select_action')
    if 'action' in action_obj and isinstance(action_obj['action'], dict):
        process_action(action_obj['action'], ability_idx, f'{path}.action')

# Process all abilities
for idx, ability in enumerate(abilities):
    # Fix effect
    if ability.get('effect'):
        process_action(ability['effect'], idx)
    
    # Fix cost
    if fix_missing_source_in_cost(ability, idx):
        pass  # Already logged

# Save fixed file
with open(abilities_path, 'w', encoding='utf-8') as f:
    json.dump(data, f, ensure_ascii=False, indent=2)

print(f'Fixed {len(fixes_made)} issues:')
for fix in fixes_made[:50]:
    print(f'  {fix}')
if len(fixes_made) > 50:
    print(f'  ... and {len(fixes_made) - 50} more')

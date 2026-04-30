import requests
import json
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

def get_all_cards():
    """Get complete card registry"""
    try:
        response = requests.get(f"{BASE_URL}/api/get_card_registry")
        if response.status_code == 200:
            data = response.json()
            # Handle different response formats
            if isinstance(data, list):
                return data
            elif isinstance(data, dict) and 'cards' in data:
                return data['cards']
            else:
                return data
        return []
    except Exception as e:
        print(f"Error getting card registry: {e}")
        return []

def analyze_all_abilities():
    """Analyze all abilities in the card database"""
    cards = get_all_cards()
    abilities_by_type = {
        'activation': [],
        'automatic': [],
        'continuous': [],
        'unknown': []
    }
    
    for card in cards:
        if not isinstance(card, dict):
            continue
            
        card_name = card.get('name', 'Unknown')
        card_id = card.get('id')
        card_type = card.get('type', 'Unknown')
        abilities = card.get('abilities', [])
        
        for ability in abilities:
            if not isinstance(ability, dict):
                continue
                
            ability_text = ability.get('text', '')
            trigger = ability.get('trigger', '')
            
            # Categorize by trigger type
            if trigger in ['Activation', 'kidou', 'kidou.png|']:
                abilities_by_type['activation'].append({
                    'card_name': card_name,
                    'card_id': card_id,
                    'card_type': card_type,
                    'ability_text': ability_text,
                    'trigger': trigger,
                    'full_ability': ability
                })
            elif trigger in ['Automatic', 'jidou', 'jidou.png|']:
                abilities_by_type['automatic'].append({
                    'card_name': card_name,
                    'card_id': card_id,
                    'card_type': card_type,
                    'ability_text': ability_text,
                    'trigger': trigger,
                    'full_ability': ability
                })
            elif trigger in ['Continuous', 'joki', 'joki.png|']:
                abilities_by_type['continuous'].append({
                    'card_name': card_name,
                    'card_id': card_id,
                    'card_type': card_type,
                    'ability_text': ability_text,
                    'trigger': trigger,
                    'full_ability': ability
                })
            else:
                abilities_by_type['unknown'].append({
                    'card_name': card_name,
                    'card_id': card_id,
                    'card_type': card_type,
                    'ability_text': ability_text,
                    'trigger': trigger,
                    'full_ability': ability
                })
    
    return abilities_by_type

def find_cards_with_abilities_in_state(state, abilities_by_type):
    """Find cards in current game state that have abilities"""
    cards_in_game = []
    
    # Check hand cards
    p1_hand = state.get('player1', {}).get('hand', {}).get('cards', [])
    for card in p1_hand:
        if isinstance(card, dict):
            card_id = card.get('id')
            # Find this card in our ability database
            for ability_type, ability_list in abilities_by_type.items():
                for ability_info in ability_list:
                    if ability_info['card_id'] == card_id:
                        cards_in_game.append({
                            'location': 'hand',
                            'player': 'P1',
                            'card': card,
                            'abilities': [ability_info]
                        })
    
    # Check stage cards
    p1_stage = state.get('player1', {}).get('stage', {})
    for pos, card in [('left', p1_stage.get('left_side')), ('center', p1_stage.get('center')), ('right', p1_stage.get('right_side'))]:
        if card and isinstance(card, dict) and card.get('name'):
            card_id = card.get('id')
            for ability_type, ability_list in abilities_by_type.items():
                for ability_info in ability_list:
                    if ability_info['card_id'] == card_id:
                        cards_in_game.append({
                            'location': f'stage_{pos}',
                            'player': 'P1',
                            'card': card,
                            'abilities': [ability_info]
                        })
    
    # Check other zones (discard, waiting room, etc.)
    p1_discard = state.get('player1', {}).get('discard', {}).get('cards', [])
    p1_waiting = state.get('player1', {}).get('waitroom', {}).get('cards', [])
    
    for card in p1_discard + p1_waiting:
        if isinstance(card, dict):
            card_id = card.get('id')
            for ability_type, ability_list in abilities_by_type.items():
                for ability_info in ability_list:
                    if ability_info['card_id'] == card_id:
                        location = 'discard' if card in p1_discard else 'waiting_room'
                        cards_in_game.append({
                            'location': location,
                            'player': 'P1',
                            'card': card,
                            'abilities': [ability_info]
                        })
    
    return cards_in_game

def analyze_ability_requirements(ability_info):
    """Analyze what an ability needs to function"""
    ability_text = ability_info['ability_text']
    full_ability = ability_info['full_ability']
    
    requirements = {
        'needs_stage': False,
        'needs_hand': False,
        'needs_discard': False,
        'needs_energy': False,
        'needs_heart_colors': [],
        'needs_specific_cards': [],
        'cost': 0,
        'self_cost': False,
        'exclude_self': False,
        'target_requirements': []
    }
    
    # Parse ability text for requirements
    text_lower = ability_text.lower()
    
    # Check for stage requirements
    if 'stage' in text_lower or 'stage' in ability_text:
        requirements['needs_stage'] = True
    
    # Check for hand requirements
    if 'hand' in text_lower:
        requirements['needs_hand'] = True
    
    # Check for discard/waiting room requirements
    if any(word in text_lower for word in ['discard', 'waiting room', 'graveyard']):
        requirements['needs_discard'] = True
    
    # Check for energy requirements
    if 'energy' in text_lower:
        requirements['needs_energy'] = True
    
    # Check for heart color requirements
    heart_colors = ['heart00', 'heart01', 'heart02', 'heart03', 'heart04', 'heart05', 'heart06']
    for color in heart_colors:
        if color in text_lower:
            requirements['needs_heart_colors'].append(color)
    
    # Check for specific card requirements
    if 'this member' in text_lower:
        requirements['needs_specific_cards'].append('self')
    elif 'member' in text_lower:
        requirements['needs_specific_cards'].append('member')
    elif 'live' in text_lower:
        requirements['needs_specific_cards'].append('live')
    
    # Parse ability structure for requirements
    if isinstance(full_ability, dict):
        requirements['cost'] = full_ability.get('cost', 0)
        requirements['self_cost'] = full_ability.get('self_cost', False)
        requirements['exclude_self'] = full_ability.get('exclude_self', False)
        
        # Check effects for requirements
        effects = full_ability.get('effects', [])
        for effect in effects:
            if isinstance(effect, dict):
                effect_type = effect.get('effect_type', '')
                if 'count' in effect:
                    count = effect['count']
                    if count > 0 and requirements['exclude_self']:
                        requirements['needs_stage'] = True
                        requirements['target_requirements'].append(f"{count} other cards on stage")
    
    return requirements

def test_ability_in_context(card_info, ability_info, state):
    """Test if an ability can be activated in current context"""
    requirements = analyze_ability_requirements(ability_info)
    location = card_info['location']
    
    test_results = {
        'can_activate': False,
        'missing_requirements': [],
        'available_actions': []
    }
    
    # Check location-based requirements
    if ability_info['trigger'] in ['Activation', 'kidou']:
        # Activation abilities need to be on stage or in hand with proper conditions
        if location not in ['hand', 'stage_left', 'stage_center', 'stage_right']:
            test_results['missing_requirements'].append("Activation ability not in valid location")
        else:
            test_results['can_activate'] = True
    elif ability_info['trigger'] in ['Automatic', 'jidou']:
        # Automatic abilities trigger on conditions
        test_results['can_activate'] = True  # Will trigger when conditions are met
    elif ability_info['trigger'] in ['Continuous', 'joki']:
        # Continuous abilities are always active
        test_results['can_activate'] = True
    
    # Check stage requirements
    if requirements['needs_stage']:
        p1_stage = state.get('player1', {}).get('stage', {})
        stage_cards = []
        for pos in ['left_side', 'center', 'right_side']:
            card = p1_stage.get(pos)
            if card and card.get('name'):
                stage_cards.append(card)
        
        if requirements['exclude_self']:
            # Need other cards besides self
            needed = 1  # Default to 1 other card
            for effect in requirements.get('target_requirements', []):
                if 'other cards on stage' in effect:
                    needed = int(effect.split()[0])  # Extract number from "X other cards"
            
            if len(stage_cards) <= 1:  # Only self or none
                test_results['missing_requirements'].append(f"Need {needed} other cards on stage (excluding self)")
            else:
                test_results['can_activate'] = True
        else:
            if len(stage_cards) == 0:
                test_results['missing_requirements'].append("Need cards on stage")
            else:
                test_results['can_activate'] = True
    
    # Check energy requirements
    if requirements['needs_energy']:
        p1_energy = state.get('player1', {}).get('energy', {}).get('cards', [])
        active_energy = len([e for e in p1_energy if isinstance(e, dict) and e.get('orientation') == 'Active'])
        
        if active_energy < requirements['cost']:
            test_results['missing_requirements'].append(f"Need {requirements['cost']} energy, have {active_energy}")
        else:
            test_results['can_activate'] = True
    
    return test_results

def comprehensive_ability_test():
    """Run comprehensive ability testing"""
    print("=== COMPREHENSIVE ABILITY TESTING ===")
    
    # Get all abilities from card database
    abilities_by_type = analyze_all_abilities()
    
    print(f"\nFound {len(abilities_by_type['activation'])} activation abilities")
    print(f"Found {len(abilities_by_type['automatic'])} automatic abilities")
    print(f"Found {len(abilities_by_type['continuous'])} continuous abilities")
    print(f"Found {len(abilities_by_type['unknown'])} unknown abilities")
    
    # Get current game state
    state, actions = get_state_and_actions()
    if not state:
        print("No game state available")
        return
    
    print(f"\nCurrent Phase: {state.get('phase', 'Unknown')}")
    print(f"Current Turn: {state.get('turn', 'Unknown')}")
    
    # Find cards with abilities in current state
    cards_with_abilities = find_cards_with_abilities_in_state(state, abilities_by_type)
    
    print(f"\nCards with abilities in game: {len(cards_with_abilities)}")
    
    # Test each ability
    test_results = []
    for card_info in cards_with_abilities:
        for ability_info in card_info['abilities']:
            print(f"\n--- Testing {card_info['card']['name']} ({card_info['location']}) ---")
            print(f"Ability: {ability_info['ability_text'][:100]}...")
            print(f"Trigger: {ability_info['trigger']}")
            
            result = test_ability_in_context(card_info, ability_info, state)
            
            print(f"Can activate: {result['can_activate']}")
            if result['missing_requirements']:
                print(f"Missing: {', '.join(result['missing_requirements'])}")
            
            test_results.append({
                'card': card_info['card']['name'],
                'location': card_info['location'],
                'ability': ability_info['ability_text'][:100],
                'trigger': ability_info['trigger'],
                'can_activate': result['can_activate'],
                'missing': result['missing_requirements']
            })
    
    # Find available ability actions
    ability_actions = [a for a in actions if 'ability' in a.get('action_type', '').lower()]
    
    print(f"\nAvailable ability actions: {len(ability_actions)}")
    for action in ability_actions:
        print(f"  - {action.get('action_type', '')}: {action.get('description', '')}")
    
    # Summary
    activatable_abilities = [r for r in test_results if r['can_activate']]
    print(f"\n=== SUMMARY ===")
    print(f"Total abilities found: {len(test_results)}")
    print(f"Can activate now: {len(activatable_abilities)}")
    print(f"Cannot activate: {len(test_results) - len(activatable_abilities)}")
    
    if activatable_abilities:
        print(f"\nActivatable abilities:")
        for result in activatable_abilities:
            print(f"  - {result['card']} ({result['location']}): {result['ability']}...")
    
    return test_results, ability_actions

if __name__ == "__main__":
    comprehensive_ability_test()

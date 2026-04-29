import requests
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

def get_card_details(card_id):
    """Get detailed card information including abilities"""
    try:
        # Get card registry to find card details
        response = requests.get(f"{BASE_URL}/api/get_card_registry")
        if response.status_code == 200:
            cards = response.json()
            for card in cards:
                if card.get('id') == card_id:
                    return card
        return None
    except Exception as e:
        print(f"Error getting card details: {e}")
        return None

def examine_current_state():
    """Examine current game state for ability testing"""
    state, actions = get_state_and_actions()
    if not state:
        print("No game state")
        return
        
    print(f"=== CURRENT STATE ANALYSIS ===")
    print(f"Phase: {state.get('phase', 'Unknown')}")
    print(f"Turn: {state.get('turn', 'Unknown')}")
    
    # Examine Player 1 hand for abilities
    p1_hand = state.get('player1', {}).get('hand', {}).get('cards', [])
    print(f"\nP1 Hand ({len(p1_hand)} cards):")
    
    for i, card in enumerate(p1_hand):
        card_name = card.get('name', 'Unknown')
        card_id = card.get('id')
        card_type = card.get('type', 'Unknown')
        
        print(f"  [{i}] {card_name} ({card_type}) - ID: {card_id}")
        
        # Get detailed card info including abilities
        card_details = get_card_details(card_id)
        if card_details:
            abilities = card_details.get('abilities', [])
            if abilities:
                print(f"      ABILITIES ({len(abilities)}):")
                for j, ability in enumerate(abilities):
                    ability_text = ability.get('text', 'No text')
                    ability_trigger = ability.get('trigger', 'Unknown')
                    print(f"        [{j}] {ability_trigger}: {ability_text}")
            else:
                print(f"      No abilities")
        else:
            print(f"      Could not fetch card details")
    
    # Examine stage members for abilities
    p1_stage = state.get('player1', {}).get('stage', {})
    stage_positions = ['left_side', 'center', 'right_side']
    stage_names = ['Left', 'Center', 'Right']
    
    print(f"\nP1 Stage:")
    for pos_name, pos_key in zip(stage_names, stage_positions):
        member = p1_stage.get(pos_key)
        if member and member.get('name'):
            card_name = member.get('name', 'Unknown')
            card_id = member.get('id')
            print(f"  {pos_name}: {card_name} - ID: {card_id}")
            
            # Get detailed card info including abilities
            card_details = get_card_details(card_id)
            if card_details:
                abilities = card_details.get('abilities', [])
                if abilities:
                    print(f"      ABILITIES ({len(abilities)}):")
                    for j, ability in enumerate(abilities):
                        ability_text = ability.get('text', 'No text')
                        ability_trigger = ability.get('trigger', 'Unknown')
                        print(f"        [{j}] {ability_trigger}: {ability_text}")
                        
                        # Check if ability can be activated now
                        if ability_trigger in ['Activation', 'kidou']:
                            print(f"          -> Can be activated manually")
                        elif ability_trigger in ['Automatic', 'jidou']:
                            print(f"          -> Triggers automatically")
                        elif ability_trigger in ['Continuous', 'joki']:
                            print(f"          -> Always active")
                else:
                    print(f"      No abilities")
            else:
                print(f"      Could not fetch card details")
    
    # Show available actions that might be ability-related
    print(f"\nAvailable Actions ({len(actions)}):")
    for i, action in enumerate(actions):
        action_type = action.get('action_type', '')
        description = action.get('description', '')
        
        if any(keyword in action_type.lower() for keyword in ['ability', 'activate', 'trigger']):
            print(f"  [{i}] {action_type}: {description} <-- ABILITY ACTION")
        else:
            print(f"  [{i}] {action_type}: {description}")

def test_stage_member_abilities():
    """Test abilities of stage members"""
    state, actions = get_state_and_actions()
    if not state:
        return
        
    p1_stage = state.get('player1', {}).get('stage', {})
    
    # Check if any stage member has activation abilities
    for pos_key in ['left_side', 'center', 'right_side']:
        member = p1_stage.get(pos_key)
        if member and member.get('name'):
            card_id = member.get('id')
            card_details = get_card_details(card_id)
            
            if card_details:
                abilities = card_details.get('abilities', [])
                for ability in abilities:
                    trigger = ability.get('trigger', '')
                    text = ability.get('text', '')
                    
                    if trigger in ['Activation', 'kidou']:
                        print(f"\n=== TESTING ACTIVATION ABILITY ===")
                        print(f"Card: {member.get('name', 'Unknown')}")
                        print(f"Ability: {text}")
                        
                        # Look for ability activation actions
                        ability_actions = [a for a in actions if 'ability' in a.get('action_type', '').lower()]
                        if ability_actions:
                            print(f"Found {len(ability_actions)} ability actions:")
                            for i, action in enumerate(ability_actions):
                                print(f"  [{i}] {action.get('action_type', '')}: {action.get('description', '')}")
                            
                            # Try the first ability action
                            if ability_actions:
                                action = ability_actions[0]
                                action_type = action.get('action_type', '')
                                action_idx = actions.index(action)
                                
                                print(f"\nTrying action: {action_type}")
                                state, actions, success = execute_action_and_get_state(action_idx, action_type)
                                
                                if success:
                                    print("Ability activation successful!")
                                    examine_current_state()
                                else:
                                    print("Ability activation failed")
                        else:
                            print("No ability activation actions available")

def play_more_cards():
    """Play more cards to get more abilities on stage"""
    state, actions = get_state_and_actions()
    if not state:
        return
        
    phase = state.get('phase', '')
    
    if 'Main' in phase:
        # Try to play another member
        idx, action = find_action_by_description(actions, 'Cost: Left: 2')
        if idx is not None:
            print("Playing another member to stage")
            state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
            if success:
                print("Successfully played member")
                examine_current_state()
                return True
        else:
            # Try any member
            idx, action = find_action_by_type(actions, 'play_member_to_stage')
            if idx is not None:
                print("Playing member to stage")
                state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
                if success:
                    print("Successfully played member")
                    examine_current_state()
                    return True
    
    return False

if __name__ == "__main__":
    examine_current_state()

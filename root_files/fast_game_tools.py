import requests
import json

BASE_URL = "http://localhost:8080"

def get_state_and_actions():
    """Single call to get both game state and available actions"""
    try:
        # Get game state
        state_response = requests.get(f"{BASE_URL}/api/game-state")
        if state_response.status_code != 200:
            return None, None
            
        state = state_response.json()
        
        # Get actions
        actions_response = requests.get(f"{BASE_URL}/api/actions")
        if actions_response.status_code != 200:
            return state, None
            
        actions = actions_response.json().get('actions', [])
        
        return state, actions
    except Exception as e:
        print(f"Error: {e}")
        return None, None

def execute_action_and_get_state(action_index, action_type):
    """Execute action and immediately get new state"""
    try:
        # Execute action
        payload = {
            "action_index": action_index,
            "action_type": action_type,
            "stage_area": None,
            "card_index": None,
            "card_indices": None,
            "card_no": None,
            "use_baton_touch": None
        }
        response = requests.post(f"{BASE_URL}/api/execute-action", json=payload)
        
        if response.status_code != 200:
            print(f"Action failed: {response.text}")
            return None, None, False
            
        result = response.json()
        
        # Get new state immediately
        new_state, new_actions = get_state_and_actions()
        
        return new_state, new_actions, True
        
    except Exception as e:
        print(f"Error: {e}")
        return None, None, False

def find_action_by_type(actions, action_type_substring):
    """Find first action matching type substring"""
    for i, action in enumerate(actions):
        if action_type_substring in action.get('action_type', ''):
            return i, action
    return None, None

def find_action_by_description(actions, description_substring):
    """Find first action matching description substring"""
    for i, action in enumerate(actions):
        if description_substring in action.get('description', ''):
            return i, action
    return None, None

def fast_play_game():
    """Fast automated game playing"""
    state, actions = get_state_and_actions()
    if not state:
        print("No game state")
        return
        
    print(f"Phase: {state.get('phase', 'Unknown')}")
    print(f"Actions available: {len(actions)}")
    
    # Dynamic action selection based on phase
    phase = state.get('phase', '')
    
    if 'RockPaperScissors' in phase:
        # Complete RPS sequence
        rps_actions = ['rock_choice', 'paper_choice', 'scissors_choice']
        for rps_action in rps_actions:
            idx, action = find_action_by_type(actions, rps_action)
            if idx is not None:
                print(f"Playing {rps_action}")
                state, actions, success = execute_action_and_get_state(idx, rps_action)
                if not success:
                    break
                print(f"New phase: {state.get('phase', 'Unknown') if state else 'None'}")
                if state and 'ChooseFirstAttacker' in state.get('phase', ''):
                    break
                    
    elif 'ChooseFirstAttacker' in phase:
        idx, action = find_action_by_type(actions, 'choose_first_attacker')
        if idx is not None:
            print("Choosing first attacker")
            state, actions, success = execute_action_and_get_state(idx, 'choose_first_attacker')
            
    elif 'Mulligan' in phase:
        idx, action = find_action_by_type(actions, 'skip_mulligan')
        if idx is not None:
            print("Skipping mulligan")
            state, actions, success = execute_action_and_get_state(idx, 'skip_mulligan')
            
    elif 'LiveCardSet' in phase:
        # Set a live card or confirm
        idx, action = find_action_by_type(actions, 'set_live_card')
        if idx is not None:
            print("Setting live card")
            state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
        else:
            idx, action = find_action_by_type(actions, 'confirm_live_card_set')
            if idx is not None:
                print("Confirming live card set")
                state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
    elif 'Main' in phase:
        # Try to play a cheap member to stage for ability testing
        # Try to find any playable member by checking actual energy requirements
        for i, action in enumerate(actions):
            if 'play_member_to_stage' in action.get('action_type', ''):
                # Try to execute and see if it works
                print(f"Trying to play: {action.get('description', '')}")
                state, actions, success = execute_action_and_get_state(i, action.get('action_type', ''))
                if success:
                    print("Successfully played member to stage!")
                    break
                else:
                    print("Failed - trying next member")
        else:
            # If no members can be played, pass
            idx, action = find_action_by_type(actions, 'pass')
            if idx is not None:
                print("No playable members, passing turn")
                state, actions, success = execute_action_and_get_state(idx, 'pass')
    
    # Show final state
    if state:
        print(f"\nFinal phase: {state.get('phase', 'Unknown')}")
        p1_hand = len(state.get('player1', {}).get('hand', {}).get('cards', []))
        p2_hand = len(state.get('player2', {}).get('hand', {}).get('cards', []))
        print(f"P1 hand: {p1_hand} cards, P2 hand: {p2_hand} cards")
        
        # Show stage members
        p1_stage = state.get('player1', {}).get('stage', {})
        stage_cards = [p1_stage.get('left_side'), p1_stage.get('center'), p1_stage.get('right_side')]
        stage_cards = [card for card in stage_cards if card and card.get('name')]
        if stage_cards:
            print(f"P1 stage: {[card.get('name', 'Unknown') for card in stage_cards]}")

if __name__ == "__main__":
    fast_play_game()

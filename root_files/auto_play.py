import requests
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

def get_to_main_phase():
    """Automatically play through phases to reach Main phase with multiple cards on stage"""
    state, actions = get_state_and_actions()
    if not state:
        print("No game state")
        return
        
    phase = state.get('phase', '')
    print(f"Current phase: {phase}")
    
    # Handle different phases
    while True:
        state, actions = get_state_and_actions()
        if not state:
            break
            
        phase = state.get('phase', '')
        print(f"\nPhase: {phase}")
        
        if 'LiveCardSet' in phase:
            # Set a live card
            idx, action = find_action_by_type(actions, 'set_live_card')
            if idx is not None:
                print("Setting live card")
                state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
                continue
            else:
                # Find confirm action
                idx, action = find_action_by_type(actions, 'confirm_live_card_set')
                if idx is not None:
                    print("Confirming live card set")
                    state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
                    continue
                    
        elif 'Performance' in phase:
            # Pass through performance
            idx, action = find_action_by_type(actions, 'pass')
            if idx is not None:
                print("Passing performance")
                state, actions, success = execute_action_and_get_state(idx, 'pass')
                continue
                
        elif 'Main' in phase:
            # We're in main phase - check stage composition
            p1_stage = state.get('player1', {}).get('stage', {})
            stage_cards = []
            for pos in ['left_side', 'center', 'right_side']:
                card = p1_stage.get(pos)
                if card and card.get('name'):
                    stage_cards.append(card.get('name', 'Unknown'))
            
            print(f"Stage currently has {len(stage_cards)} cards: {stage_cards}")
            
            if len(stage_cards) >= 2:
                print("Ready for ability testing!")
                break
            else:
                # Play more cards to stage
                idx, action = find_action_by_type(actions, 'play_member_to_stage')
                if idx is not None:
                    print(f"Playing {action.get('description', '')}")
                    state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
                    continue
                else:
                    print("No more cards to play")
                    break
        else:
            # Unknown phase - try to find pass or continue
            idx, action = find_action_by_type(actions, 'pass')
            if idx is not None:
                print("Passing unknown phase")
                state, actions, success = execute_action_and_get_state(idx, 'pass')
                continue
            else:
                print("No actions available")
                break

if __name__ == "__main__":
    get_to_main_phase()

#!/usr/bin/env python3
"""
New Game Starter - Starts a fresh game and gets to main phase for analysis
"""

import requests
import json
import time
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

def start_new_game():
    """Start a completely new game"""
    try:
        # Try to start new game endpoint
        response = requests.post(f"{BASE_URL}/api/new-game")
        if response.status_code == 200:
            print("New game started successfully")
            return True
        else:
            print(f"Failed to start new game: {response.status_code}")
            return False
    except Exception as e:
        print(f"Error starting new game: {e}")
        return False

def play_to_main_phase():
    """Play through game phases to reach main phase with cards on stage"""
    print("=== Playing to Main Phase ===")
    
    # Start new game first
    if not start_new_game():
        print("Could not start new game")
        return None
    
    time.sleep(1)  # Give server time to process
    
    state, actions = get_state_and_actions()
    if not state:
        print("No game state after starting new game")
        return None
    
    print(f"Starting phase: {state.get('phase', 'Unknown')}")
    
    # Play through phases automatically
    max_turns = 50  # Prevent infinite loops
    turn_count = 0
    
    while turn_count < max_turns:
        state, actions = get_state_and_actions()
        if not state:
            break
            
        phase = state.get('phase', '')
        print(f"\nTurn {turn_count}: Phase: {phase}")
        
        # Handle different phases
        if 'RockPaperScissors' in phase:
            # Play RPS
            rps_actions = ['rock_choice', 'paper_choice', 'scissors_choice']
            for rps_action in rps_actions:
                idx, action = find_action_by_type(actions, rps_action)
                if idx is not None:
                    print(f"  Playing {rps_action}")
                    state, actions, success = execute_action_and_get_state(idx, rps_action)
                    if success and state and 'ChooseFirstAttacker' in state.get('phase', ''):
                        break
                        
        elif 'ChooseFirstAttacker' in phase:
            idx, action = find_action_by_type(actions, 'choose_first_attacker')
            if idx is not None:
                print("  Choosing first attacker")
                state, actions, success = execute_action_and_get_state(idx, 'choose_first_attacker')
                
        elif 'Mulligan' in phase:
            idx, action = find_action_by_type(actions, 'skip_mulligan')
            if idx is not None:
                print("  Skipping mulligan")
                state, actions, success = execute_action_and_get_state(idx, 'skip_mulligan')
            else:
                # Try mulligan if skip not available
                idx, action = find_action_by_type(actions, 'mulligan')
                if idx is not None:
                    print("  Taking mulligan")
                    state, actions, success = execute_action_and_get_state(idx, 'mulligan')
                    
        elif 'LiveCardSet' in phase:
            # Set live card
            idx, action = find_action_by_type(actions, 'set_live_card')
            if idx is not None:
                print("  Setting live card")
                state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
            else:
                # Confirm live card set
                idx, action = find_action_by_type(actions, 'confirm_live_card_set')
                if idx is not None:
                    print("  Confirming live card set")
                    state, actions, success = execute_action_and_get_state(idx, action.get('action_type', ''))
                    
        elif 'Performance' in phase:
            # Pass performance
            idx, action = find_action_by_type(actions, 'pass')
            if idx is not None:
                print("  Passing performance")
                state, actions, success = execute_action_and_get_state(idx, 'pass')
                
        elif 'Main' in phase:
            print("  Reached Main phase!")
            # Try to play some members to stage for ability testing
            p1_stage = state.get('player1', {}).get('stage', {})
            stage_cards = []
            for pos in ['left_side', 'center', 'right_side']:
                card = p1_stage.get(pos)
                if card and card.get('name'):
                    stage_cards.append(card.get('name', 'Unknown'))
            
            print(f"  Stage currently has {len(stage_cards)} cards: {stage_cards}")
            
            # Try to play members if stage is empty
            if len(stage_cards) < 2:
                print("  Trying to play members to stage...")
                for i, action in enumerate(actions):
                    if 'play_member_to_stage' in action.get('action_type', ''):
                        print(f"    Trying: {action.get('description', '')}")
                        state, actions, success = execute_action_and_get_state(i, action.get('action_type', ''))
                        if success:
                            print("    Successfully played member!")
                            break
                        else:
                            print("    Failed - trying next")
            
            # Check final stage composition
            p1_stage = state.get('player1', {}).get('stage', {})
            final_stage_cards = []
            for pos in ['left_side', 'center', 'right_side']:
                card = p1_stage.get(pos)
                if card and card.get('name'):
                    final_stage_cards.append({
                        'position': pos,
                        'name': card.get('name', 'Unknown'),
                        'cost': card.get('cost', 0),
                        'abilities': card.get('abilities', [])
                    })
            
            print(f"  Final stage: {final_stage_cards}")
            return state
            
        elif 'End' in phase:
            print("  Game ended")
            break
            
        else:
            print(f"  Unknown phase: {phase}")
            # Try to find pass action
            idx, action = find_action_by_type(actions, 'pass')
            if idx is not None:
                print("  Passing unknown phase")
                state, actions, success = execute_action_and_get_state(idx, 'pass')
            else:
                print("  No actions available, breaking")
                break
        
        turn_count += 1
        time.sleep(0.5)  # Small delay between actions
    
    print(f"Finished after {turn_count} turns")
    return state

def analyze_current_state(state):
    """Analyze the current game state"""
    if not state:
        print("No state to analyze")
        return
    
    print("\n=== Game State Analysis ===")
    print(f"Phase: {state.get('phase', 'Unknown')}")
    print(f"Current Player: {state.get('current_player', 'Unknown')}")
    
    # Analyze player states
    for player_key in ['player1', 'player2']:
        player = state.get(player_key, {})
        print(f"\n{player_key.upper()}:")
        
        # Hand
        hand = player.get('hand', {}).get('cards', [])
        print(f"  Hand: {len(hand)} cards")
        
        # Stage
        stage = player.get('stage', {})
        stage_members = []
        for pos in ['left_side', 'center', 'right_side']:
            card = stage.get(pos)
            if card and card.get('name'):
                stage_members.append({
                    'position': pos,
                    'name': card.get('name', 'Unknown'),
                    'cost': card.get('cost', 0),
                    'state': card.get('state', 'active')
                })
        print(f"  Stage: {len(stage_members)} members")
        for member in stage_members:
            print(f"    {member['position']}: {member['name']} (Cost: {member['cost']}, State: {member['state']})")
        
        # Energy
        energy = player.get('energy_zone', {}).get('cards', [])
        active_energy = sum(1 for card in energy if card.get('state') == 'active')
        wait_energy = sum(1 for card in energy if card.get('state') == 'wait')
        print(f"  Energy: {len(energy)} total ({active_energy} active, {wait_energy} wait)")
        
        # Live cards
        live_cards = player.get('live_card_zone', {}).get('cards', [])
        print(f"  Live Cards: {len(live_cards)}")
        
        # Success live cards
        success_live = player.get('success_live_zone', {}).get('cards', [])
        print(f"  Success Live Cards: {len(success_live)}")
        
        # Deck
        deck = player.get('deck', {}).get('cards', [])
        print(f"  Deck: {len(deck)} cards")

def main():
    """Main function"""
    print("=== New Game Starter ===")
    
    # Play to main phase
    state = play_to_main_phase()
    
    # Analyze the state
    analyze_current_state(state)
    
    return state

if __name__ == "__main__":
    main()

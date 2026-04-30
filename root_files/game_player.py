#!/usr/bin/env python3
"""
Simple LLM game player - reads from web server and selects actions
"""

import requests
import time
import json

BASE_URL = "http://localhost:8080"

def get_game_state():
    """Get current game state from web server"""
    response = requests.get(f"{BASE_URL}/api/game-state")
    return response.json() if response.status_code == 200 else None

def get_actions():
    """Get available actions from web server"""
    response = requests.get(f"{BASE_URL}/api/actions")
    data = response.json() if response.status_code == 200 else {}
    return data.get('actions', []) if isinstance(data, dict) else []

def execute_action(index, action_type):
    """Execute selected action via web server"""
    payload = {
        "action_index": index,
        "action_type": action_type,
        "stage_area": None,
        "card_index": None,
        "card_indices": None,
        "card_no": None,
        "use_baton_touch": None
    }
    response = requests.post(f"{BASE_URL}/api/execute-action", json=payload)
    if response.status_code == 200:
        return response.json()
    else:
        print(f"  ERROR: HTTP {response.status_code} - {response.text}")
        return None

def init_game():
    """Initialize new game via web server"""
    # Use available deck files from server
    payload = {"deck1": "muse_cup.txt", "deck2": "aqours_cup.txt"}
    response = requests.post(f"{BASE_URL}/api/init", json=payload)
    return response.status_code == 200

def display_card_with_ability(card, index=""):
    """Display card with ability information"""
    if isinstance(card, dict):
        name = card.get('name', 'Unknown')
        card_no = card.get('card_no', '')
        cost = card.get('cost', 0)
        ability = card.get('ability', '')
        card_type = card.get('type', 'Unknown')
        heart = card.get('base_heart', {})
        blade = card.get('blade', 0)
        
        output = f"    [{index}] {name} ({card_no}) Type: {card_type} Cost: {cost}"
        if heart:
            output += f" Hearts: {heart}"
        if blade:
            output += f" Blade: {blade}"
        if ability:
            output += f"\n        Ability: {ability[:150]}{'...' if len(ability) > 150 else ''}"
        return output
    else:
        return f"    [{index}] {card}"

def analyze_game_state(state):
    """Simple game state display - manual analysis should be done by me"""
    # Just return basic phase info - I should analyze manually
    phase = state.get('phase', '')
    turn = state.get('turn', 0)
    return [f"Turn {turn}, Phase: {phase}"]

def show_ability_texts(state):
    """Just show ability texts from cards"""
    p1 = state.get('player1', {})
    p2 = state.get('player2', {})
    
    print("\n=== ABILITY TEXTS ===")
    
    # Check hand cards
    for i, card in enumerate(p1.get('hand', [])):
        if isinstance(card, dict) and card.get('ability'):
            print(f"P1 Hand [{i}]: {card.get('name', 'Unknown')} - {card.get('ability', '')}")
    
    for i, card in enumerate(p2.get('hand', [])):
        if isinstance(card, dict) and card.get('ability'):
            print(f"P2 Hand [{i}]: {card.get('name', 'Unknown')} - {card.get('ability', '')}")
    
    # Check stage cards
    for i, card in enumerate(p1.get('stage', [])):
        if isinstance(card, dict) and card != -1 and card.get('ability'):
            position = ['Left', 'Center', 'Right'][i]
            print(f"P1 Stage ({position}): {card.get('name', 'Unknown')} - {card.get('ability', '')}")

def predict_action_outcome(state, action_index, action_type, description):
    """Simple prediction - manual analysis should be done by me"""
    # This is just a placeholder - I should manually analyze each action
    # based on the game state shown
    return ["Manual analysis needed - check game state above"]

def main():
    print("=== LLM GAME PLAYER ===")
    print("Focus: Reading from web server and selecting actions")
    
    # Wait for server
    while not get_game_state():
        print("Waiting for server...")
        time.sleep(1)
    
    print("Server ready...")
    # Only initialize if no game state exists
    state = get_game_state()
    if not state or state.get('phase') == 'RockPaperScissors' and state.get('player1', {}).get('hand', {}).get('cards') == []:
        print("Initializing new game...")
        init_game()
    else:
        print("Continuing existing game...")
    
    # === MY CHOICE - EDIT THIS BETWEEN RUNS ===
    # Will handle multiple actions in sequence
    
    # Game problems list
    problems = []
    
    while True:
        state = get_game_state()
        if not state:
            print("Lost connection")
            break
        
        # Stop at turn 10
        if state.get('turn', 0) >= 10:
            print("\n=== TURN LIMIT REACHED (10) ===")
            print("PROBLEMS IDENTIFIED:")
            for i, problem in enumerate(problems, 1):
                print(f"  {i}. {problem}")
            break
        
        print(f"\n=== TURN {state.get('turn', 'N/A')} | PHASE {state.get('phase', 'N/A')} ===")
        
        # Game state analysis
        analysis = analyze_game_state(state)
        print(f"\n=== GAME ANALYSIS ===")
        for insight in analysis:
            print(f"  - {insight}")
        
        p1 = state.get('player1', {})
        p2 = state.get('player2', {})
        
        # Check for problems
        if len(p1.get('hand', [])) > 0 and all(isinstance(c, str) for c in p1.get('hand', [])):
            problems.append("Cards in hand showing as strings instead of objects")
        
        if len(p1.get('energy_zone', [])) == 0 and state.get('turn', 0) > 1:
            problems.append("No energy cards generated after multiple turns")
        
        if len(p1.get('deck', [])) == 0:
            problems.append("Main deck not initialized")
        
        # Show ability texts
        show_ability_texts(state)
        
        print(f"\nPLAYER 1:")
        print(f"  Hand: {len(p1.get('hand', []))} cards")
        for i, card in enumerate(p1.get('hand', [])):
            print(display_card_with_ability(card, i))
        
        energy = p1.get('energy_zone', [])
        active_energy = len([e for e in energy if isinstance(e, dict)])
        print(f"  Energy: {active_energy}/{len(energy)} active cards")
        for i, card in enumerate(energy):
            if isinstance(card, dict):
                print(f"    [{i}] {card.get('name', 'Unknown')} ({card.get('card_no', '')})")
        
        stage = p1.get('stage', [])
        active_stage = [x for x in stage if x != -1 and isinstance(x, dict)]
        print(f"  Stage: {len(active_stage)}/3 members")
        for i, card in enumerate(stage):
            if card != -1 and isinstance(card, dict):
                position = ['Left', 'Center', 'Right'][i]
                print(f"    [{position}] {card.get('name', 'Unknown')} ({card.get('card_no', '')})")
        
        print(f"  Deck: {len(p1.get('deck', []))} cards")
        print(f"  Life Zone: {len(p1.get('life_zone', []))} cards")
        print(f"  Waiting Room: {len(p1.get('waiting_room', []))} cards")
        
        print(f"\nPLAYER 2:")
        print(f"  Hand: {len(p2.get('hand', []))} cards")
        for i, card in enumerate(p2.get('hand', [])):
            print(display_card_with_ability(card, i))
        
        energy = p2.get('energy_zone', [])
        active_energy = len([e for e in energy if isinstance(e, dict)])
        print(f"  Energy: {active_energy}/{len(energy)} active cards")
        for i, card in enumerate(energy):
            if isinstance(card, dict):
                print(f"    [{i}] {card.get('name', 'Unknown')} ({card.get('card_no', '')})")
        
        stage = p2.get('stage', [])
        active_stage = [x for x in stage if x != -1 and isinstance(x, dict)]
        print(f"  Stage: {len(active_stage)}/3 members")
        for i, card in enumerate(stage):
            if card != -1 and isinstance(card, dict):
                position = ['Left', 'Center', 'Right'][i]
                print(f"    [{position}] {card.get('name', 'Unknown')} ({card.get('card_no', '')})")
        
        print(f"  Deck: {len(p2.get('deck', []))} cards")
        print(f"  Life Zone: {len(p2.get('life_zone', []))} cards")
        print(f"  Waiting Room: {len(p2.get('waiting_room', []))} cards")
        
        actions = get_actions()
        print(f"\nACTIONS ({len(actions)}):")
        for i, a in enumerate(actions):
            print(f"  [{i}] {a.get('action_type', 'unknown')}: {a.get('description', '')}")
            # Show full action details for debugging
            print(f"      Full action: {a}")
            
            # Show predictions for this action
            predictions = predict_action_outcome(state, i, a.get('action_type', ''), a.get('description', ''))
            for pred in predictions:
                print(f"      Prediction: {pred}")
        
        if not actions:
            print("No actions, waiting...")
            time.sleep(1)
            continue
        
        # Dynamic action sequence based on current phase
        action_sequence = []
        current_phase = state.get('phase', '')
        
        if 'RockPaperScissors' in current_phase:
            action_sequence = [
                (2, "scissors_choice"), # P2: Scissors (complete RPS)
            ]
        elif 'ChooseFirstAttacker' in current_phase:
            action_sequence = [
                (0, "choose_first_attacker"), # Choose first attacker
            ]
        elif 'Mulligan' in current_phase:
            # Find skip_mulligan action
            skip_idx = next((i for i, a in enumerate(actions) if 'skip_mulligan' in a.get('action_type', '')), None)
            if skip_idx is not None:
                action_sequence = [(skip_idx, "skip_mulligan")]
        elif 'Main' in current_phase:
            # Find a cheap member to play
            cheap_idx = next((i for i, a in enumerate(actions) if 'play_member_to_stage' in a.get('action_type', '') and 'Cost: Left: 2' in a.get('description', '')), None)
            if cheap_idx is not None:
                action_sequence = [(cheap_idx, "play_member_to_stage")]
            else:
                # Find pass action
                pass_idx = next((i for i, a in enumerate(actions) if 'pass' in a.get('action_type', '')), None)
                if pass_idx is not None:
                    action_sequence = [(pass_idx, "pass")]
        
        for idx, act_type in action_sequence:
            if idx >= len(actions):
                print(f"  Action {idx} not available, stopping sequence")
                break
                
            chosen_action = actions[idx]
            actual_action_type = chosen_action.get('action_type', '')
            
            print(f"\n>>> MY CHOICE: [{idx}] {actual_action_type}")
            
            # Show what should happen
            predictions = predict_action_outcome(state, idx, actual_action_type, chosen_action.get('description', ''))
            for pred in predictions:
                print(f"    - {pred}")
            
            # Execute action
            result = execute_action(idx, actual_action_type)
            
            print(f"  Raw response: {result}")
            
            # Check if result indicates success
            is_success = False
            if result:
                if isinstance(result, dict):
                    # Check for success field or error field
                    if 'success' in result:
                        is_success = result['success']
                    elif 'error' in result:
                        print(f"  Server error: {result['error']}")
                    else:
                        # If no success/error field, assume success if no error
                        is_success = True
                else:
                    # Non-dict response, assume success
                    is_success = True
            
            if is_success:
                print("  Result: Success")
                
                # Analyze why it succeeded
                success_reasoning = []
                if "rock_choice" in actual_action_type or "paper_choice" in actual_action_type or "scissors_choice" in actual_action_type:
                    success_reasoning.append("RPS choice recorded")
                    success_reasoning.append("Both players must choose to advance phase")
                elif "choose_first_attacker" in actual_action_type:
                    success_reasoning.append("First attacker chosen")
                    success_reasoning.append("Game will advance to Mulligan phase")
                
                print("  Success Analysis:")
                for reason in success_reasoning:
                    print(f"    - {reason}")
                    
                # Get new state after action
                state = get_game_state()
                if not state:
                    print("Lost connection after action")
                    break
                    
                # Get new actions
                actions = get_actions()
                print(f"\nAFTER ACTION ({len(actions)}):")
                for i, a in enumerate(actions):
                    print(f"  [{i}] {a.get('action_type', 'unknown')}: {a.get('description', '')}")
                    
            else:
                print("  Result: Failed")
                
                # Analyze why it failed
                failure_reasoning = []
                if "choose_first_attacker" in actual_action_type:
                    failure_reasoning.append("Could not choose first attacker")
                    failure_reasoning.append("Possible: RPS phase not complete or invalid action")
                
                print("  Failure Analysis:")
                for reason in failure_reasoning:
                    print(f"    - {reason}")
                    
                problems.append(f"Action failed: {chosen_action.get('description', '')}")
                break
        
        # Continue with next phase
        continue
        
if __name__ == "__main__":
    main()

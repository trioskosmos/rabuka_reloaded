import requests
from fast_game_tools import get_state_and_actions

BASE_URL = "http://localhost:8080"

def investigate_cost_issue():
    """Investigate why all members cost 15 energy despite showing lower costs"""
    state, actions = get_state_and_actions()
    if not state:
        print("No game state available")
        return
    
    print("=== COST INVESTIGATION ===")
    print(f"Current Phase: {state.get('phase', 'Unknown')}")
    print(f"Turn: {state.get('turn', 'Unknown')}")
    
    # Check energy state
    p1_energy = state.get('player1', {}).get('energy', {}).get('cards', [])
    active_energy = len([e for e in p1_energy if isinstance(e, dict) and e.get('orientation') == 'Active'])
    total_energy = len(p1_energy)
    
    print(f"\nEnergy State:")
    print(f"  Total energy cards: {total_energy}")
    print(f"  Active energy: {active_energy}")
    print(f"  Energy cards: {p1_energy}")
    
    # Analyze action costs
    print(f"\n=== ACTION COST ANALYSIS ===")
    
    play_actions = [a for a in actions if 'play_member_to_stage' in a.get('action_type', '')]
    print(f"Found {len(play_actions)} play_member_to_stage actions")
    
    for i, action in enumerate(play_actions):
        description = action.get('description', '')
        action_type = action.get('action_type', '')
        parameters = action.get('parameters', {})
        
        print(f"\nAction {i+1}:")
        print(f"  Type: {action_type}")
        print(f"  Description: {description}")
        print(f"  Parameters: {parameters}")
        
        # Extract cost from description
        import re
        cost_patterns = [
            r'Cost: Left: (\d+)',
            r'Cost: Center: (\d+)',
            r'Cost: Right: (\d+)'
        ]
        
        costs = []
        for pattern in cost_patterns:
            match = re.search(pattern, description)
            if match:
                costs.append(int(match.group(1)))
        
        if costs:
            print(f"  Description costs: {costs}")
        
        # Check parameters for cost
        if parameters:
            base_cost = parameters.get('base_cost')
            final_cost = parameters.get('final_cost')
            print(f"  Base cost: {base_cost}")
            print(f"  Final cost: {final_cost}")
            
            # Check available areas
            available_areas = parameters.get('available_areas', [])
            if available_areas:
                print(f"  Available areas:")
                for area in available_areas:
                    print(f"    {area}")
        
        # Try to execute the action to see the actual error
        print(f"  Testing execution...")
        try:
            payload = {
                "action_index": actions.index(action),
                "action_type": action_type,
                "stage_area": None,
                "card_index": None,
                "card_indices": None,
                "card_no": None,
                "use_baton_touch": None
            }
            
            response = requests.post(f"{BASE_URL}/api/execute-action", json=payload)
            if response.status_code != 200:
                error_text = response.text
                print(f"  Execution failed: {error_text}")
                
                # Extract actual cost from error message
                import re
                cost_match = re.search(r'Could not pay (\d+) energy', error_text)
                if cost_match:
                    actual_cost = int(cost_match.group(1))
                    print(f"  Actual cost required: {actual_cost}")
        except Exception as e:
            print(f"  Exception during execution: {e}")
    
    # Check if there are any cheaper actions
    print(f"\n=== LOOKING FOR CHEAPER ACTIONS ===")
    
    cheap_actions = []
    for action in actions:
        action_type = action.get('action_type', '')
        if action_type == 'pass':
            cheap_actions.append(action)
        elif 'use_ability' in action_type:
            cheap_actions.append(action)
        elif action_type in ['rock_choice', 'paper_choice', 'scissors_choice']:
            cheap_actions.append(action)
        elif action_type in ['choose_first_attacker', 'choose_second_attacker']:
            cheap_actions.append(action)
        elif 'skip_mulligan' in action_type:
            cheap_actions.append(action)
        elif 'set_live_card' in action_type:
            cheap_actions.append(action)
    
    print(f"Found {len(cheap_actions)} potentially cheap/free actions:")
    for action in cheap_actions:
        print(f"  - {action.get('action_type', '')}: {action.get('description', '')}")
    
    # Investigate the cost calculation issue
    print(f"\n=== COST CALCULATION INVESTIGATION ===")
    print("The issue appears to be:")
    print("1. Description shows 'Cost: Left: 2' but actual cost is 15")
    print("2. This suggests the cost calculation is wrong in the engine")
    print("3. Need to check how costs are calculated vs displayed")
    
    return {
        'energy_available': active_energy,
        'total_energy': total_energy,
        'play_actions': len(play_actions),
        'cheap_actions': len(cheap_actions),
        'cost_issue_detected': True
    }

def check_engine_cost_calculation():
    """Check how the engine calculates costs"""
    print("\n=== ENGINE COST CALCULATION ANALYSIS ===")
    
    # Look for patterns in the cost discrepancy
    print("Observations:")
    print("- All members show costs like 'Cost: Left: 2' in description")
    print("- But execution fails with 'Could not pay 15 energy'")
    print("- This suggests the engine is calculating cost differently")
    print("- Possible causes:")
    print("  1. Cost is calculated as sum of all area costs (2+2+2=6, not 15)")
    print("  2. Cost calculation includes additional factors")
    print("  3. Cost display is wrong, not the calculation")
    print("  4. There's a bug in cost calculation logic")
    
    print("\nNext steps:")
    print("1. Check engine code for cost calculation")
    print("2. Verify cost calculation vs display")
    print("3. Fix the cost calculation issue")
    print("4. Test with corrected costs")

if __name__ == "__main__":
    investigation = investigate_cost_issue()
    check_engine_cost_calculation()

import requests
import json
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

def analyze_game_state_for_abilities():
    """Analyze current game state to find potential ability patterns"""
    state, actions = get_state_and_actions()
    if not state:
        return None, None
    
    # Get all cards in different zones
    p1_hand = state.get('player1', {}).get('hand', {}).get('cards', [])
    p1_stage = state.get('player1', {}).get('stage', {})
    p1_discard = state.get('player1', {}).get('discard', {}).get('cards', [])
    p1_waiting = state.get('player1', {}).get('waitroom', {}).get('cards', [])
    p1_energy = state.get('player1', {}).get('energy', {}).get('cards', [])
    
    # Count cards in each zone
    stage_cards = []
    for pos in ['left_side', 'center', 'right_side']:
        card = p1_stage.get(pos)
        if card and card.get('name'):
            stage_cards.append(card)
    
    analysis = {
        'phase': state.get('phase', 'Unknown'),
        'turn': state.get('turn', 0),
        'hand_count': len(p1_hand),
        'stage_count': len(stage_cards),
        'discard_count': len(p1_discard),
        'waiting_count': len(p1_waiting),
        'energy_count': len(p1_energy),
        'active_energy': len([e for e in p1_energy if isinstance(e, dict) and e.get('orientation') == 'Active']),
        'hand_cards': p1_hand,
        'stage_cards': stage_cards,
        'discard_cards': p1_discard,
        'waiting_cards': p1_waiting,
        'available_actions': actions
    }
    
    return analysis, state

def find_ability_actions(actions):
    """Find all ability-related actions"""
    ability_actions = []
    
    for action in actions:
        action_type = action.get('action_type', '').lower()
        description = action.get('description', '')
        
        if any(keyword in action_type for keyword in ['ability', 'activate', 'use_ability']):
            ability_actions.append(action)
        elif 'ability' in description.lower():
            ability_actions.append(action)
        elif '{{kidou' in description or '{{jidou' in description or '{{joki' in description:
            ability_actions.append(action)
    
    return ability_actions

def analyze_action_patterns(actions):
    """Analyze patterns in available actions to infer ability types"""
    patterns = {
        'activation_abilities': [],
        'automatic_triggers': [],
        'continuous_effects': [],
        'cost_requirements': {},
        'target_requirements': {}
    }
    
    for action in actions:
        description = action.get('description', '')
        action_type = action.get('action_type', '')
        
        # Look for activation abilities
        if '{{kidou' in description or 'Activation' in description:
            patterns['activation_abilities'].append({
                'action': action,
                'description': description,
                'cost': extract_cost_from_description(description)
            })
        
        # Look for automatic triggers
        elif '{{jidou' in description or 'Automatic' in description:
            patterns['automatic_triggers'].append({
                'action': action,
                'description': description
            })
        
        # Look for continuous effects
        elif '{{joki' in description or 'Continuous' in description:
            patterns['continuous_effects'].append({
                'action': action,
                'description': description
            })
        
        # Extract cost information
        cost = extract_cost_from_description(description)
        if cost > 0:
            patterns['cost_requirements'][action_type] = cost
        
        # Extract target requirements
        target_info = extract_target_info(description)
        if target_info:
            patterns['target_requirements'][action_type] = target_info
    
    return patterns

def extract_cost_from_description(description):
    """Extract cost information from action description"""
    import re
    
    # Look for cost patterns
    cost_patterns = [
        r'Cost: (\d+)',
        r'cost: (\d+)',
        r'(\d+) energy',
        r'energy: (\d+)'
    ]
    
    for pattern in cost_patterns:
        match = re.search(pattern, description)
        if match:
            return int(match.group(1))
    
    return 0

def extract_target_info(description):
    """Extract target information from action description"""
    target_info = {
        'needs_stage': False,
        'needs_hand': False,
        'needs_discard': False,
        'exclude_self': False,
        'target_count': 0
    }
    
    desc_lower = description.lower()
    
    if 'stage' in desc_lower:
        target_info['needs_stage'] = True
    if 'hand' in desc_lower:
        target_info['needs_hand'] = True
    if 'discard' in desc_lower or 'waiting' in desc_lower:
        target_info['needs_discard'] = True
    if 'exclude_self' in desc_lower or 'excluding self' in desc_lower:
        target_info['exclude_self'] = True
    
    # Extract count patterns
    import re
    count_patterns = [
        r'(\d+) cards?',
        r'(\d+) members?',
        r'need (\d+)',
        r'require (\d+)'
    ]
    
    for pattern in count_patterns:
        match = re.search(pattern, description)
        if match:
            target_info['target_count'] = int(match.group(1))
            break
    
    return target_info

def simulate_ability_scenarios(analysis):
    """Simulate different scenarios to test abilities"""
    scenarios = []
    
    # Scenario 1: Test activation abilities with multiple cards on stage
    if analysis['stage_count'] >= 2:
        scenarios.append({
            'name': 'Multi-card stage activation',
            'description': 'Test activation abilities that require multiple cards on stage',
            'conditions_met': True,
            'expected_abilities': len([a for a in analysis['available_actions'] if 'use_ability' in a.get('action_type', '')])
        })
    else:
        scenarios.append({
            'name': 'Multi-card stage activation',
            'description': 'Need more cards on stage for activation abilities',
            'conditions_met': False,
            'requirements': f'Need at least 2 cards on stage, have {analysis["stage_count"]}'
        })
    
    # Scenario 2: Test abilities with sufficient energy
    if analysis['active_energy'] >= 2:
        scenarios.append({
            'name': 'Energy-based abilities',
            'description': 'Test abilities that require energy',
            'conditions_met': True,
            'available_energy': analysis['active_energy']
        })
    else:
        scenarios.append({
            'name': 'Energy-based abilities',
            'description': 'Need more energy for ability activation',
            'conditions_met': False,
            'requirements': f'Need at least 2 energy, have {analysis["active_energy"]}'
        })
    
    # Scenario 3: Test hand-based abilities
    if analysis['hand_count'] >= 5:
        scenarios.append({
            'name': 'Hand-based abilities',
            'description': 'Test abilities that work from hand',
            'conditions_met': True,
            'hand_size': analysis['hand_count']
        })
    else:
        scenarios.append({
            'name': 'Hand-based abilities',
            'description': 'Need more cards in hand',
            'conditions_met': False,
            'requirements': f'Need at least 5 cards in hand, have {analysis["hand_count"]}'
        })
    
    return scenarios

def create_ability_test_plan():
    """Create a comprehensive ability testing plan"""
    print("=== SMART ABILITY TESTING SYSTEM ===")
    
    # Analyze current game state
    analysis, state = analyze_game_state_for_abilities()
    if not analysis:
        print("No game state available")
        return
    
    print(f"\n=== GAME STATE ANALYSIS ===")
    print(f"Phase: {analysis['phase']}")
    print(f"Turn: {analysis['turn']}")
    print(f"Hand: {analysis['hand_count']} cards")
    print(f"Stage: {analysis['stage_count']} cards")
    print(f"Discard: {analysis['discard_count']} cards")
    print(f"Waiting: {analysis['waiting_count']} cards")
    print(f"Energy: {analysis['active_energy']}/{analysis['energy_count']} active")
    
    # Show cards in each zone
    print(f"\n=== CARDS BY ZONE ===")
    
    if analysis['stage_cards']:
        print(f"Stage Cards:")
        for i, card in enumerate(analysis['stage_cards']):
            print(f"  [{i}] {card.get('name', 'Unknown')} (ID: {card.get('id', 'Unknown')})")
    
    if analysis['hand_cards']:
        print(f"Hand Cards (first 5):")
        for i, card in enumerate(analysis['hand_cards'][:5]):
            print(f"  [{i}] {card.get('name', 'Unknown')} (ID: {card.get('id', 'Unknown')})")
    
    # Find ability actions
    ability_actions = find_ability_actions(analysis['available_actions'])
    print(f"\n=== ABILITY ACTIONS ===")
    print(f"Found {len(ability_actions)} ability-related actions")
    
    for i, action in enumerate(ability_actions):
        print(f"  [{i}] {action.get('action_type', '')}")
        print(f"      {action.get('description', '')}")
    
    # Analyze action patterns
    patterns = analyze_action_patterns(analysis['available_actions'])
    print(f"\n=== ABILITY PATTERNS ===")
    print(f"Activation abilities: {len(patterns['activation_abilities'])}")
    print(f"Automatic triggers: {len(patterns['automatic_triggers'])}")
    print(f"Continuous effects: {len(patterns['continuous_effects'])}")
    
    # Show cost requirements
    if patterns['cost_requirements']:
        print(f"\nCost Requirements:")
        for action_type, cost in patterns['cost_requirements'].items():
            print(f"  {action_type}: {cost} energy")
    
    # Show target requirements
    if patterns['target_requirements']:
        print(f"\nTarget Requirements:")
        for action_type, target in patterns['target_requirements'].items():
            print(f"  {action_type}: {target}")
    
    # Simulate scenarios
    scenarios = simulate_ability_scenarios(analysis)
    print(f"\n=== TESTING SCENARIOS ===")
    
    for scenario in scenarios:
        status = "READY" if scenario['conditions_met'] else "NOT READY"
        print(f"\n{scenario['name']}: {status}")
        print(f"  {scenario['description']}")
        if not scenario['conditions_met']:
            print(f"  Requirements: {scenario.get('requirements', 'Unknown')}")
        else:
            print(f"  Can test now!")
    
    # Generate testing recommendations
    print(f"\n=== TESTING RECOMMENDATIONS ===")
    
    if analysis['stage_count'] < 2:
        print("1. Play more cards to stage to enable activation abilities")
    
    if analysis['active_energy'] < 3:
        print("2. Activate more energy cards to enable cost-based abilities")
    
    if ability_actions:
        print(f"3. Test the {len(ability_actions)} available ability actions")
        for i, action in enumerate(ability_actions):
            print(f"   - Try action {i}: {action.get('description', '')}")
    else:
        print("3. No ability actions available - need to meet requirements first")
    
    return analysis, patterns, scenarios

def test_available_abilities():
    """Test currently available abilities"""
    analysis, _, _ = create_ability_test_plan()
    
    if not analysis:
        return
    
    ability_actions = find_ability_actions(analysis['available_actions'])
    
    if not ability_actions:
        print("\n=== NO ABILITIES TO TEST ===")
        print("Need to meet ability requirements first")
        return
    
    print(f"\n=== TESTING {len(ability_actions)} ABILITIES ===")
    
    for i, action in enumerate(ability_actions):
        print(f"\n--- Testing Ability {i+1} ---")
        print(f"Action: {action.get('action_type', '')}")
        print(f"Description: {action.get('description', '')}")
        
        # Get action parameters
        action_params = action.get('parameters', {})
        if action_params:
            print(f"Parameters: {action_params}")
        
        # Execute the ability
        action_type = action.get('action_type', '')
        action_idx = analysis['available_actions'].index(action)
        
        # Build payload with parameters
        payload = {
            "action_index": action_idx,
            "action_type": action_type,
            "stage_area": action_params.get('stage_area'),
            "card_index": action_params.get('card_index'),
            "card_indices": action_params.get('card_indices'),
            "card_no": action_params.get('card_no'),
            "use_baton_touch": action_params.get('use_baton_touch'),
            "card_id": action_params.get('card_id')
        }
        
        try:
            response = requests.post(f"{BASE_URL}/api/execute-action", json=payload)
            if response.status_code == 200:
                print("SUCCESS: Ability executed!")
                
                # Get new state
                new_state, new_actions = get_state_and_actions()
                if new_state:
                    print(f"New phase: {new_state.get('phase', 'Unknown')}")
                    
                    # Check for changes
                    if new_state.get('turn') != analysis['turn']:
                        print(f"Turn advanced: {analysis['turn']} -> {new_state.get('turn')}")
                    
                    # Check hand/stage changes
                    new_hand_count = len(new_state.get('player1', {}).get('hand', {}).get('cards', []))
                    new_stage_cards = []
                    for pos in ['left_side', 'center', 'right_side']:
                        card = new_state.get('player1', {}).get('stage', {}).get(pos)
                        if card and card.get('name'):
                            new_stage_cards.append(card)
                    
                    if new_hand_count != analysis['hand_count']:
                        print(f"Hand changed: {analysis['hand_count']} -> {new_hand_count}")
                    
                    if len(new_stage_cards) != analysis['stage_count']:
                        print(f"Stage changed: {analysis['stage_count']} -> {len(new_stage_cards)}")
                
            else:
                print(f"FAILED: {response.text}")
        except Exception as e:
            print(f"ERROR: {e}")

if __name__ == "__main__":
    test_available_abilities()

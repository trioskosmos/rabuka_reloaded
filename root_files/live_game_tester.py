import requests
import json
import time
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

class LiveGameTester:
    def __init__(self):
        self.game_history = []
        self.ability_tests = []
        self.action_predictions = []
        self.issues_found = []
        
    def run_live_game_test(self):
        """Run comprehensive live game testing"""
        print("=== LIVE GAME TESTING SYSTEM ===")
        
        # 1. Test cost calculation fix
        print("\n1. TESTING COST CALCULATION FIX")
        cost_test_result = self.test_cost_calculation_fix()
        
        # 2. Progress through game phases
        print("\n2. PROGRESSING THROUGH GAME PHASES")
        phase_progression = self.progress_through_phases()
        
        # 3. Test abilities when available
        print("\n3. TESTING ABILITIES")
        ability_test_results = self.test_abilities_live()
        
        # 4. Verify action predictions
        print("\n4. VERIFYING ACTION PREDICTIONS")
        prediction_results = self.verify_action_predictions()
        
        # 5. Generate live test report
        print("\n5. GENERATING LIVE TEST REPORT")
        live_report = self.generate_live_test_report(cost_test_result, phase_progression, ability_test_results, prediction_results)
        
        return {
            'cost_test': cost_test_result,
            'phase_progression': phase_progression,
            'ability_tests': ability_test_results,
            'predictions': prediction_results,
            'report': live_report
        }
    
    def test_cost_calculation_fix(self):
        """Test the cost calculation fix"""
        print("Testing cost calculation fix...")
        
        # Get initial state
        state, actions = get_state_and_actions()
        if not state:
            return {'status': 'failed', 'reason': 'No game state available'}
        
        # Progress to Main phase to test card costs
        progression_result = self.progress_to_main_phase()
        if not progression_result['success']:
            return {'status': 'failed', 'reason': 'Could not reach Main phase'}
        
        # Get Main phase state
        state, actions = get_state_and_actions()
        if not state:
            return {'status': 'failed', 'reason': 'No game state in Main phase'}
        
        # Find play_member_to_stage actions
        play_actions = [a for a in actions if 'play_member_to_stage' in a.get('action_type', '')]
        
        if not play_actions:
            return {'status': 'failed', 'reason': 'No play_member_to_stage actions available'}
        
        # Test first few play actions
        cost_test_results = []
        for i, action in enumerate(play_actions[:3]):
            print(f"Testing play action {i+1}: {action.get('description', '')}")
            
            # Extract expected cost from description
            description = action.get('description', '')
            import re
            cost_match = re.search(r'Cost: [^:]+: (\d+)', description)
            expected_cost = int(cost_match.group(1)) if cost_match else 0
            
            # Check energy availability
            p1_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            
            test_result = {
                'action_index': actions.index(action),
                'description': description,
                'expected_cost': expected_cost,
                'available_energy': p1_energy,
                'can_afford': p1_energy >= expected_cost
            }
            
            if test_result['can_afford']:
                # Try to execute the action
                print(f"  Trying to play card (cost: {expected_cost}, energy: {p1_energy})")
                result, analysis, success = execute_action_and_get_state(test_result['action_index'], action.get('action_type', ''))
                
                test_result['execution_result'] = result
                test_result['execution_success'] = success
                test_result['status'] = 'success' if success else 'failed'
                
                if success:
                    print(f"  SUCCESS: Card played with cost {expected_cost}")
                    cost_test_results.append(test_result)
                    break  # Test successful, move on
                else:
                    print(f"  FAILED: {result}")
                    test_result['error'] = result
            else:
                print(f"  CANNOT AFFORD: Need {expected_cost} energy, have {p1_energy}")
                test_result['status'] = 'cannot_afford'
            
            cost_test_results.append(test_result)
        
        return {
            'status': 'completed',
            'results': cost_test_results,
            'fix_working': any(r['status'] == 'success' for r in cost_test_results)
        }
    
    def progress_to_main_phase(self):
        """Progress through phases to reach Main phase"""
        print("Progressing to Main phase...")
        
        phase_sequence = ['RockPaperScissors', 'ChooseFirstAttacker', 'MulliganP1Turn', 'MulliganP2Turn', 'Main']
        current_phase = None
        
        for _ in range(20):  # Max 20 actions to prevent infinite loop
            state, actions = get_state_and_actions()
            if not state:
                return {'success': False, 'reason': 'No game state available'}
            
            current_phase = state.get('phase', '')
            print(f"Current phase: {current_phase}")
            
            if current_phase == 'Main':
                return {'success': True, 'phase': current_phase}
            
            # Select appropriate action for current phase
            action_index, action_type = self.select_phase_action(current_phase, actions)
            
            if action_index is not None:
                print(f"  Executing: {action_type}")
                result, analysis, success = execute_action_and_get_state(action_index, action_type)
                
                if not success:
                    print(f"  Failed: {result}")
                    return {'success': False, 'reason': f'Action failed: {result}'}
            else:
                print(f"  No suitable action found for phase {current_phase}")
                return {'success': False, 'reason': f'No action for phase {current_phase}'}
        
        return {'success': False, 'reason': f'Could not reach Main phase, stuck at {current_phase}'}
    
    def select_phase_action(self, phase, actions):
        """Select appropriate action for current phase"""
        if phase == 'RockPaperScissors':
            # Choose rock
            for i, action in enumerate(actions):
                if 'rock_choice' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        elif phase == 'ChooseFirstAttacker':
            # Choose first attacker
            for i, action in enumerate(actions):
                if 'choose_first_attacker' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        elif phase in ['MulliganP1Turn', 'MulliganP2Turn']:
            # Skip mulligan
            for i, action in enumerate(actions):
                if 'skip_mulligan' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        return None, None
    
    def progress_through_phases(self):
        """Progress through multiple phases and document transitions"""
        print("Progressing through phases and documenting transitions...")
        
        phase_history = []
        
        for _ in range(30):  # Max 30 actions
            state, actions = get_state_and_actions()
            if not state:
                break
            
            current_phase = state.get('phase', '')
            turn = state.get('turn', 0)
            
            # Record phase state
            phase_record = {
                'turn': turn,
                'phase': current_phase,
                'p1_hand': len(state.get('player1', {}).get('hand', {}).get('cards', [])),
                'p1_stage': len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')]),
                'p1_energy': len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active']),
                'p2_hand': len(state.get('player2', {}).get('hand', {}).get('cards', [])),
                'p2_stage': len([c for c in [state.get('player2', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')]),
                'p2_energy': len([e for e in state.get('player2', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active']),
                'actions_available': len(actions)
            }
            
            phase_history.append(phase_record)
            
            # Select and execute action
            action_index, action_type = self.select_action_for_phase(current_phase, actions, state)
            
            if action_index is not None:
                result, analysis, success = execute_action_and_get_state(action_index, action_type)
                
                if not success:
                    print(f"Action failed: {result}")
                    break
            else:
                print(f"No action available for phase {current_phase}")
                break
        
        return {
            'status': 'completed',
            'phase_history': phase_history,
            'total_phases': len(phase_history)
        }
    
    def select_action_for_phase(self, phase, actions, state):
        """Select action for current phase with strategic consideration"""
        if phase == 'RockPaperScissors':
            # Choose rock
            for i, action in enumerate(actions):
                if 'rock_choice' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        elif phase == 'ChooseFirstAttacker':
            # Choose first attacker for tempo advantage
            for i, action in enumerate(actions):
                if 'choose_first_attacker' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        elif phase in ['MulliganP1Turn', 'MulliganP2Turn']:
            # Skip mulligan to get to Main phase faster
            for i, action in enumerate(actions):
                if 'skip_mulligan' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        elif phase == 'Main':
            # Try to play a member to stage
            p1_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            
            # If we have few stage cards, try to play one
            if p1_stage < 2:
                for i, action in enumerate(actions):
                    if 'play_member_to_stage' in action.get('action_type', ''):
                        # Check if we can afford it
                        description = action.get('description', '')
                        import re
                        cost_match = re.search(r'Cost: [^:]+: (\d+)', description)
                        cost = int(cost_match.group(1)) if cost_match else 0
                        
                        if p1_energy >= cost:
                            return i, action.get('action_type', '')
            
            # Otherwise pass
            for i, action in enumerate(actions):
                if 'pass' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        elif 'LiveCardSet' in phase:
            # Set live card
            for i, action in enumerate(actions):
                if 'set_live_card' in action.get('action_type', ''):
                    return i, action.get('action_type', '')
        
        return None, None
    
    def test_abilities_live(self):
        """Test abilities in live game"""
        print("Testing abilities in live game...")
        
        ability_test_results = []
        
        # Try to find and test abilities multiple times
        for attempt in range(5):
            state, actions = get_state_and_actions()
            if not state:
                break
            
            # Look for ability actions
            ability_actions = []
            for i, action in enumerate(actions):
                action_type = action.get('action_type', '').lower()
                description = action.get('description', '')
                
                if 'ability' in action_type or 'use_ability' in action_type or '{{kidou' in description or '{{jidou' in description:
                    ability_actions.append((i, action))
            
            if ability_actions:
                print(f"Found {len(ability_actions)} ability actions")
                
                for action_index, action in ability_actions:
                    print(f"Testing ability: {action.get('description', '')}")
                    
                    # Extract ability info
                    ability_info = self.extract_ability_info(action)
                    
                    # Test the ability
                    test_result = self.test_single_ability(action_index, action, ability_info)
                    test_result['attempt'] = attempt + 1
                    ability_test_results.append(test_result)
                    
                    if test_result['success']:
                        print(f"  SUCCESS: {test_result['result']}")
                    else:
                        print(f"  FAILED: {test_result['result']}")
            else:
                print(f"No abilities found in attempt {attempt + 1}")
            
            # Take a turn to potentially trigger more abilities
            if attempt < 4:
                self.take_turn()
        
        return {
            'status': 'completed',
            'results': ability_test_results,
            'total_tests': len(ability_test_results),
            'successful_tests': len([r for r in ability_test_results if r['success']])
        }
    
    def extract_ability_info(self, action):
        """Extract ability information from action"""
        description = action.get('description', '')
        
        ability_info = {
            'trigger_type': 'unknown',
            'predicted_effect': 'unknown',
            'requirements': {}
        }
        
        # Extract trigger type
        if '{{kidou' in description:
            ability_info['trigger_type'] = 'Activation'
        elif '{{jidou' in description:
            ability_info['trigger_type'] = 'Automatic'
        elif '{{joki' in description:
            ability_info['trigger_type'] = 'Continuous'
        
        # Extract predicted effect
        desc_lower = description.lower()
        if 'draw' in desc_lower:
            ability_info['predicted_effect'] = 'Draw cards'
        elif 'damage' in desc_lower or 'blade' in desc_lower:
            ability_info['predicted_effect'] = 'Deal damage'
        elif 'heal' in desc_lower or 'life' in desc_lower:
            ability_info['predicted_effect'] = 'Gain life'
        elif 'energy' in desc_lower:
            ability_info['predicted_effect'] = 'Manipulate energy'
        elif 'stage' in desc_lower:
            ability_info['predicted_effect'] = 'Manipulate stage'
        elif 'discard' in desc_lower:
            ability_info['predicted_effect'] = 'Discard cards'
        elif 'search' in desc_lower:
            ability_info['predicted_effect'] = 'Search deck'
        
        return ability_info
    
    def test_single_ability(self, action_index, action, ability_info):
        """Test a single ability"""
        # Get state before action
        before_state, _ = get_state_and_actions()
        if not before_state:
            return {'success': False, 'result': 'No game state available'}
        
        # Execute ability
        result, analysis, success = execute_action_and_get_state(action_index, action.get('action_type', ''))
        
        # Get state after action
        after_state, _ = get_state_and_actions()
        
        # Analyze the effect
        effect_analysis = self.analyze_ability_effect(before_state, after_state, ability_info)
        
        return {
            'success': success,
            'result': result,
            'ability_info': ability_info,
            'effect_analysis': effect_analysis,
            'before_state': before_state,
            'after_state': after_state
        }
    
    def analyze_ability_effect(self, before_state, after_state, ability_info):
        """Analyze the actual effect of an ability"""
        analysis = {
            'hand_changed': False,
            'stage_changed': False,
            'energy_changed': False,
            'life_changed': False,
            'discard_changed': False,
            'actual_effect': 'unknown'
        }
        
        # Check hand changes
        before_hand = len(before_state.get('player1', {}).get('hand', {}).get('cards', []))
        after_hand = len(after_state.get('player1', {}).get('hand', {}).get('cards', []))
        if before_hand != after_hand:
            analysis['hand_changed'] = True
            if after_hand > before_hand:
                analysis['actual_effect'] = 'Drew cards'
            else:
                analysis['actual_effect'] = 'Discarded cards'
        
        # Check stage changes
        before_stage = len([c for c in [before_state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        after_stage = len([c for c in [after_state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        if before_stage != after_stage:
            analysis['stage_changed'] = True
            analysis['actual_effect'] = 'Stage manipulation'
        
        # Check energy changes
        before_energy = len([e for e in before_state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
        after_energy = len([e for e in after_state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
        if before_energy != after_energy:
            analysis['energy_changed'] = True
            analysis['actual_effect'] = 'Energy manipulation'
        
        # Check life changes
        before_life = len(before_state.get('player1', {}).get('life_zone', {}).get('cards', []))
        after_life = len(after_state.get('player1', {}).get('life_zone', {}).get('cards', []))
        if before_life != after_life:
            analysis['life_changed'] = True
            analysis['actual_effect'] = 'Life manipulation'
        
        return analysis
    
    def take_turn(self):
        """Take a simple turn to progress the game"""
        state, actions = get_state_and_actions()
        if not state:
            return
        
        phase = state.get('phase', '')
        
        # Simple action based on phase
        if phase == 'Main':
            # Pass the turn
            for i, action in enumerate(actions):
                if 'pass' in action.get('action_type', ''):
                    execute_action_and_get_state(i, action.get('action_type', ''))
                    break
        elif 'LiveCardSet' in phase:
            # Set live card
            for i, action in enumerate(actions):
                if 'set_live_card' in action.get('action_type', ''):
                    execute_action_and_get_state(i, action.get('action_type', ''))
                    break
    
    def verify_action_predictions(self):
        """Verify action predictions against actual outcomes"""
        print("Verifying action predictions...")
        
        prediction_results = []
        
        # Test several actions and their predictions
        for _ in range(5):
            state, actions = get_state_and_actions()
            if not state:
                break
            
            # Select an action to test
            action_index, action = self.select_action_for_prediction(actions, state)
            
            if action_index is not None:
                # Make prediction
                prediction = self.predict_action_outcome(action, state)
                
                # Execute action
                before_state = state
                result, analysis, success = execute_action_and_get_state(action_index, action.get('action_type', ''))
                
                # Get after state
                after_state, _ = get_state_and_actions()
                
                # Verify prediction
                verification = self.verify_prediction(prediction, before_state, after_state, success)
                
                prediction_results.append({
                    'action': action,
                    'prediction': prediction,
                    'result': result,
                    'success': success,
                    'verification': verification
                })
        
        return {
            'status': 'completed',
            'results': prediction_results,
            'total_tests': len(prediction_results),
            'accurate_predictions': len([r for r in prediction_results if r['verification']['accurate']])
        }
    
    def select_action_for_prediction(self, actions, state):
        """Select an action for prediction testing"""
        # Prefer play_member_to_stage actions for testing
        for i, action in enumerate(actions):
            if 'play_member_to_stage' in action.get('action_type', ''):
                return i, action
        
        # Fall back to pass actions
        for i, action in enumerate(actions):
            if 'pass' in action.get('action_type', ''):
                return i, action
        
        return None, None
    
    def predict_action_outcome(self, action, state):
        """Predict the outcome of an action"""
        action_type = action.get('action_type', '')
        description = action.get('description', '')
        
        prediction = {
            'predicted_outcome': 'unknown',
            'confidence': 0.5,
            'reasoning': []
        }
        
        if 'pass' in action_type:
            phase = state.get('phase', '')
            prediction['predicted_outcome'] = f'Turn ends, phase advances from {phase}'
            prediction['confidence'] = 0.9
            prediction['reasoning'].append('Pass action always advances phase')
        
        elif 'play_member_to_stage' in action_type:
            # Extract cost
            import re
            cost_match = re.search(r'Cost: [^:]+: (\d+)', description)
            cost = int(cost_match.group(1)) if cost_match else 0
            
            p1_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            
            if p1_energy >= cost:
                prediction['predicted_outcome'] = f'Member played to stage, {cost} energy spent, stage: {p1_stage}->{p1_stage+1}'
                prediction['confidence'] = 0.8
                prediction['reasoning'].append(f'Sufficient energy ({p1_energy} >= {cost})')
            else:
                prediction['predicted_outcome'] = f'Action fails - insufficient energy (need {cost}, have {p1_energy})'
                prediction['confidence'] = 0.9
                prediction['reasoning'].append(f'Insufficient energy ({p1_energy} < {cost})')
        
        return prediction
    
    def verify_prediction(self, prediction, before_state, after_state, success):
        """Verify if prediction was accurate"""
        verification = {
            'accurate': False,
            'discrepancies': [],
            'actual_changes': {}
        }
        
        if not success:
            # Check if failure was predicted
            if 'fails' in prediction['predicted_outcome'].lower():
                verification['accurate'] = True
            else:
                verification['discrepancies'].append('Predicted success but action failed')
        else:
            # Check actual changes
            before_phase = before_state.get('phase', '')
            after_phase = after_state.get('phase', '')
            
            before_stage = len([c for c in [before_state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            after_stage = len([c for c in [after_state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            
            before_energy = len([e for e in before_state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            after_energy = len([e for e in after_state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            
            verification['actual_changes'] = {
                'phase': f'{before_phase} -> {after_phase}',
                'stage': f'{before_stage} -> {after_stage}',
                'energy': f'{before_energy} -> {after_energy}'
            }
            
            # Check if prediction matches actual outcome
            predicted_outcome = prediction['predicted_outcome'].lower()
            
            if 'stage:' in predicted_outcome:
                if f'{before_stage}->{after_stage}' in predicted_outcome:
                    verification['accurate'] = True
                else:
                    verification['discrepancies'].append(f'Stage change mismatch: predicted {predicted_outcome}, actual {before_stage}->{after_stage}')
            
            elif 'phase advances' in predicted_outcome:
                if before_phase != after_phase:
                    verification['accurate'] = True
                else:
                    verification['discrepancies'].append('Phase did not advance as predicted')
        
        return verification
    
    def generate_live_test_report(self, cost_test, phase_progression, ability_tests, predictions):
        """Generate comprehensive live test report"""
        report = []
        report.append("# LIVE GAME TESTING REPORT")
        report.append(f"Generated: {time.strftime('%Y-%m-%d %H:%M:%S')}")
        report.append("")
        
        # Executive Summary
        report.append("## EXECUTIVE SUMMARY")
        report.append(f"- **Cost Calculation Fix**: {'Working' if cost_test.get('fix_working') else 'Not Working'}")
        report.append(f"- **Phase Progression**: {phase_progression.get('total_phases', 0)} phases tested")
        report.append(f"- **Ability Tests**: {ability_tests.get('total_tests', 0)} tests, {ability_tests.get('successful_tests', 0)} successful")
        report.append(f"- **Action Predictions**: {predictions.get('total_tests', 0)} tests, {predictions.get('accurate_predictions', 0)} accurate")
        report.append("")
        
        # Cost Calculation Test Results
        report.append("## COST CALCULATION TEST RESULTS")
        if cost_test.get('status') == 'completed':
            results = cost_test.get('results', [])
            report.append(f"**Status**: {cost_test.get('status')}")
            report.append(f"**Fix Working**: {cost_test.get('fix_working')}")
            report.append("")
            
            for result in results:
                report.append(f"### Test {results.index(result) + 1}")
                report.append(f"**Description**: {result.get('description', 'N/A')}")
                report.append(f"**Expected Cost**: {result.get('expected_cost', 'N/A')}")
                report.append(f"**Available Energy**: {result.get('available_energy', 'N/A')}")
                report.append(f"**Can Afford**: {result.get('can_afford', 'N/A')}")
                report.append(f"**Status**: {result.get('status', 'N/A')}")
                if result.get('execution_result'):
                    report.append(f"**Execution Result**: {result.get('execution_result')}")
                report.append("")
        else:
            report.append(f"**Status**: {cost_test.get('status')}")
            report.append(f"**Reason**: {cost_test.get('reason', 'Unknown')}")
            report.append("")
        
        # Phase Progression Results
        report.append("## PHASE PROGRESSION RESULTS")
        if phase_progression.get('status') == 'completed':
            history = phase_progression.get('phase_history', [])
            report.append(f"**Status**: {phase_progression.get('status')}")
            report.append(f"**Total Phases**: {phase_progression.get('total_phases', 0)}")
            report.append("")
            
            # Show last few phase entries
            for entry in history[-5:]:
                report.append(f"### Turn {entry.get('turn', 'N/A')} - {entry.get('phase', 'N/A')}")
                report.append(f"**P1**: Hand {entry.get('p1_hand', 0)}, Stage {entry.get('p1_stage', 0)}, Energy {entry.get('p1_energy', 0)}")
                report.append(f"**P2**: Hand {entry.get('p2_hand', 0)}, Stage {entry.get('p2_stage', 0)}, Energy {entry.get('p2_energy', 0)}")
                report.append("")
        else:
            report.append(f"**Status**: {phase_progression.get('status')}")
            report.append("")
        
        # Ability Test Results
        report.append("## ABILITY TEST RESULTS")
        if ability_tests.get('status') == 'completed':
            results = ability_tests.get('results', [])
            report.append(f"**Status**: {ability_tests.get('status')}")
            report.append(f"**Total Tests**: {ability_tests.get('total_tests', 0)}")
            report.append(f"**Successful Tests**: {ability_tests.get('successful_tests', 0)}")
            report.append("")
            
            for result in results:
                report.append(f"### Test {results.index(result) + 1}")
                report.append(f"**Ability Type**: {result.get('ability_info', {}).get('trigger_type', 'N/A')}")
                report.append(f"**Predicted Effect**: {result.get('ability_info', {}).get('predicted_effect', 'N/A')}")
                report.append(f"**Success**: {result.get('success', 'N/A')}")
                report.append(f"**Result**: {result.get('result', 'N/A')}")
                
                effect_analysis = result.get('effect_analysis', {})
                if effect_analysis.get('actual_effect') != 'unknown':
                    report.append(f"**Actual Effect**: {effect_analysis.get('actual_effect')}")
                report.append("")
        else:
            report.append(f"**Status**: {ability_tests.get('status')}")
            report.append("")
        
        # Action Prediction Results
        report.append("## ACTION PREDICTION RESULTS")
        if predictions.get('status') == 'completed':
            results = predictions.get('results', [])
            report.append(f"**Status**: {predictions.get('status')}")
            report.append(f"**Total Tests**: {predictions.get('total_tests', 0)}")
            report.append(f"**Accurate Predictions**: {predictions.get('accurate_predictions', 0)}")
            report.append("")
            
            for result in results:
                report.append(f"### Test {results.index(result) + 1}")
                report.append(f"**Action**: {result.get('action', {}).get('action_type', 'N/A')}")
                report.append(f"**Predicted Outcome**: {result.get('prediction', {}).get('predicted_outcome', 'N/A')}")
                report.append(f"**Confidence**: {result.get('prediction', {}).get('confidence', 'N/A')}")
                report.append(f"**Success**: {result.get('success', 'N/A')}")
                report.append(f"**Prediction Accurate**: {result.get('verification', {}).get('accurate', 'N/A')}")
                
                discrepancies = result.get('verification', {}).get('discrepancies', [])
                if discrepancies:
                    report.append(f"**Discrepancies**: {', '.join(discrepancies)}")
                report.append("")
        else:
            report.append(f"**Status**: {predictions.get('status')}")
            report.append("")
        
        # Issues Found
        if self.issues_found:
            report.append("## ISSUES FOUND")
            for issue in self.issues_found:
                report.append(f"### {issue.get('type', 'Unknown')}")
                report.append(f"**Description**: {issue.get('description', 'N/A')}")
                report.append(f"**Severity**: {issue.get('severity', 'N/A')}")
                report.append("")
        
        # Conclusion
        report.append("## CONCLUSION")
        report.append("Live game testing has been completed. Key findings:")
        report.append("")
        
        if cost_test.get('fix_working'):
            report.append("1. **Cost Calculation Fix**: Successfully working - cards can be played with correct costs")
        else:
            report.append("1. **Cost Calculation Fix**: Not working - further investigation needed")
        
        if ability_tests.get('successful_tests', 0) > 0:
            report.append(f"2. **Ability Testing**: {ability_tests.get('successful_tests', 0)} abilities tested successfully")
        else:
            report.append("2. **Ability Testing**: No abilities tested successfully - may need different game conditions")
        
        if predictions.get('accurate_predictions', 0) > 0:
            report.append(f"3. **Action Predictions**: {predictions.get('accurate_predictions', 0)} predictions accurate")
        else:
            report.append("3. **Action Predictions**: No accurate predictions - prediction model needs improvement")
        
        report.append("")
        report.append("The game engine is functional but may need additional improvements for full compliance.")
        
        # Save report
        report_text = "\n".join(report)
        with open('live_game_test_report.md', 'w', encoding='utf-8') as f:
            f.write(report_text)
        
        return report_text

def run_live_game_test():
    """Run comprehensive live game testing"""
    tester = LiveGameTester()
    
    print("=== LIVE GAME TESTING SYSTEM ===")
    
    # Run all tests
    results = tester.run_live_game_test()
    
    # Print summary
    print(f"\n=== LIVE TEST SUMMARY ===")
    print(f"Cost Calculation Fix: {'Working' if results['cost_test'].get('fix_working') else 'Not Working'}")
    print(f"Phase Progression: {results['phase_progression'].get('total_phases', 0)} phases")
    print(f"Ability Tests: {results['ability_tests'].get('total_tests', 0)} total, {results['ability_tests'].get('successful_tests', 0)} successful")
    print(f"Action Predictions: {results['predictions'].get('total_tests', 0)} total, {results['predictions'].get('accurate_predictions', 0)} accurate")
    print(f"Report: live_game_test_report.md")
    
    return tester, results

if __name__ == "__main__":
    run_live_game_test()

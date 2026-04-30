import requests
import json
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

class GameAnalyzer:
    def __init__(self):
        self.game_history = []
        self.ability_tests = []
        self.problems_found = []
        self.winning_patterns = []
        
    def analyze_current_state(self):
        """Comprehensive analysis of current game state"""
        state, actions = get_state_and_actions()
        if not state:
            return None
            
        analysis = {
            'turn': state.get('turn', 0),
            'phase': state.get('phase', 'Unknown'),
            'player1': self.analyze_player(state.get('player1', {}), 'P1'),
            'player2': self.analyze_player(state.get('player2', {}), 'P2'),
            'available_actions': actions,
            'phase_transitions': self.analyze_phase_transitions(state),
            'winning_conditions': self.analyze_winning_conditions(state)
        }
        
        return analysis, state, actions
    
    def analyze_player(self, player_data, player_name):
        """Analyze individual player state"""
        hand = player_data.get('hand', {}).get('cards', [])
        stage = player_data.get('stage', {})
        energy = player_data.get('energy', {}).get('cards', [])
        discard = player_data.get('discard', {}).get('cards', [])
        waiting = player_data.get('waitroom', {}).get('cards', [])
        
        # Count active energy
        active_energy = len([e for e in energy if isinstance(e, dict) and e.get('orientation') == 'Active'])
        
        # Analyze stage composition
        stage_cards = []
        for pos, card in [('left', stage.get('left_side')), ('center', stage.get('center')), ('right', stage.get('right_side'))]:
            if card and isinstance(card, dict) and card.get('name'):
                stage_cards.append({
                    'position': pos,
                    'name': card.get('name'),
                    'id': card.get('id'),
                    'card_type': card.get('type'),
                    'hearts': card.get('base_heart'),
                    'blade': card.get('blade')
                })
        
        return {
            'name': player_name,
            'hand_count': len(hand),
            'hand_cards': hand[:5],  # First 5 cards for analysis
            'stage_count': len(stage_cards),
            'stage_cards': stage_cards,
            'energy_total': len(energy),
            'energy_active': active_energy,
            'discard_count': len(discard),
            'waiting_count': len(waiting),
            'deck_count': player_data.get('main_deck_count', 0),
            'life_cards': player_data.get('life_zone', {}).get('cards', [])
        }
    
    def analyze_phase_transitions(self, state):
        """Analyze phase transition patterns"""
        current_phase = state.get('phase', 'Unknown')
        transitions = {
            'current_phase': current_phase,
            'expected_next': self.predict_next_phase(current_phase),
            'phase_requirements': self.get_phase_requirements(current_phase),
            'available_actions': len(state.get('legal_actions', []))
        }
        return transitions
    
    def predict_next_phase(self, current_phase):
        """Predict what phase should come next"""
        phase_flow = {
            'RockPaperScissors': 'ChooseFirstAttacker',
            'ChooseFirstAttacker': 'MulliganP1Turn',
            'MulliganP1Turn': 'MulliganP2Turn',
            'MulliganP2Turn': 'Main',
            'Main': 'LiveCardSetP1Turn',
            'LiveCardSetP1Turn': 'LiveCardSetP2Turn',
            'LiveCardSetP2Turn': 'Performance',
            'Performance': 'Main'
        }
        return phase_flow.get(current_phase, 'Unknown')
    
    def get_phase_requirements(self, phase):
        """Get requirements for current phase"""
        requirements = {
            'RockPaperScissors': 'Both players must make RPS choices',
            'ChooseFirstAttacker': 'RPS winner must choose first/second attacker',
            'MulliganP1Turn': 'P1 may exchange cards from hand',
            'MulliganP2Turn': 'P2 may exchange cards from hand',
            'Main': 'Play members to stage, use abilities',
            'LiveCardSetP1Turn': 'P1 must set live card for performance',
            'LiveCardSetP2Turn': 'P2 must set live card for performance',
            'Performance': 'Execute live card performance'
        }
        return requirements.get(phase, 'Unknown requirements')
    
    def analyze_winning_conditions(self, state):
        """Analyze current winning conditions and progress"""
        p1_life = len(state.get('player1', {}).get('life_zone', {}).get('cards', []))
        p2_life = len(state.get('player2', {}).get('life_zone', {}).get('cards', []))
        
        return {
            'p1_life_count': p1_life,
            'p2_life_count': p2_life,
            'life_advantage': p1_life - p2_life,
            'winning_threshold': 7,  # Typical life total
            'critical_phase': self.is_critical_phase(state.get('phase', '')),
            'tempo_advantage': self.analyze_tempo(state)
        }
    
    def is_critical_phase(self, phase):
        """Determine if current phase is critical for winning"""
        critical_phases = ['Main', 'Performance']
        return phase in critical_phases
    
    def analyze_tempo(self, state):
        """Analyze tempo advantage"""
        p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        p2_stage = len([c for c in [state.get('player2', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        
        return {
            'p1_stage_advantage': p1_stage - p2_stage,
            'p1_tempo': 'ahead' if p1_stage > p2_stage else 'behind' if p1_stage < p2_stage else 'even'
        }
    
    def predict_action_outcomes(self, actions, state):
        """Predict outcomes for available actions"""
        predictions = []
        
        for action in actions:
            action_type = action.get('action_type', '')
            description = action.get('description', '')
            
            prediction = {
                'action': action,
                'predicted_outcome': self.predict_single_action(action, state),
                'confidence': self.calculate_prediction_confidence(action, state),
                'risks': self.identify_action_risks(action, state),
                'opportunities': self.identify_action_opportunities(action, state)
            }
            
            predictions.append(prediction)
        
        return predictions
    
    def predict_single_action(self, action, state):
        """Predict outcome of a single action"""
        action_type = action.get('action_type', '')
        description = action.get('description', '')
        
        if 'pass' in action_type:
            return 'Turn ends, phase advances to next phase'
        elif 'play_member_to_stage' in action_type:
            cost_match = self.extract_cost(description)
            energy_available = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            
            if cost_match and energy_available >= cost_match:
                return f'Member played to stage, {cost_match} energy spent'
            else:
                return 'Action will fail - insufficient energy'
        elif 'set_live_card' in action_type:
            return 'Live card set for performance phase'
        elif 'use_ability' in action_type:
            return 'Ability activated, effect depends on ability type and requirements'
        elif 'rock_choice' in action_type or 'paper_choice' in action_type or 'scissors_choice' in action_type:
            return 'RPS choice recorded, waiting for opponent choice'
        elif 'choose_first_attacker' in action_type:
            return 'First/second attacker chosen, game advances to Mulligan'
        else:
            return 'Unknown action outcome'
    
    def extract_cost(self, description):
        """Extract cost from action description"""
        import re
        cost_patterns = [
            r'Cost: Left: (\d+)',
            r'Cost: Center: (\d+)',
            r'Cost: Right: (\d+)',
            r'cost: (\d+)'
        ]
        
        for pattern in cost_patterns:
            match = re.search(pattern, description)
            if match:
                return int(match.group(1))
        return 0
    
    def calculate_prediction_confidence(self, action, state):
        """Calculate confidence in prediction"""
        action_type = action.get('action_type', '')
        
        # High confidence for simple actions
        if action_type in ['pass', 'rock_choice', 'paper_choice', 'scissors_choice']:
            return 0.95
        
        # Medium confidence for play actions (depends on energy)
        if 'play_member_to_stage' in action_type:
            return 0.8
        
        # Lower confidence for complex actions
        if 'use_ability' in action_type:
            return 0.6
        
        # Low confidence for unknown actions
        return 0.4
    
    def identify_action_risks(self, action, state):
        """Identify potential risks of an action"""
        risks = []
        action_type = action.get('action_type', '')
        
        if 'play_member_to_stage' in action_type:
            cost = self.extract_cost(action.get('description', ''))
            energy_available = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            
            if cost > energy_available:
                risks.append('Insufficient energy - action will fail')
            
            if cost > energy_available * 0.7:
                risks.append('High energy cost may limit future options')
        
        if 'pass' in action_type:
            current_phase = state.get('phase', '')
            if current_phase == 'Main':
                risks.append('Passing Main phase may lose tempo advantage')
        
        return risks
    
    def identify_action_opportunities(self, action, state):
        """Identify opportunities from an action"""
        opportunities = []
        action_type = action.get('action_type', '')
        
        if 'play_member_to_stage' in action_type:
            opportunities.append('Increase stage presence for tempo advantage')
            opportunities.append('May enable activation abilities')
        
        if 'set_live_card' in action_type:
            opportunities.append('Prepare for performance phase scoring')
        
        if 'use_ability' in action_type:
            opportunities.append('Activate card effects for advantage')
        
        return opportunities
    
    def execute_and_analyze_action(self, action_index, action_type):
        """Execute action and analyze results"""
        # Get state before action
        before_state, before_actions = get_state_and_actions()
        if not before_state:
            return None, None, False
        
        # Execute action
        success = False
        result = None
        
        try:
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
            if response.status_code == 200:
                success = True
                result = response.json()
            else:
                result = f"HTTP {response.status_code}: {response.text}"
        except Exception as e:
            result = f"Exception: {e}"
        
        # Get state after action
        after_state, after_actions = get_state_and_actions()
        
        # Analyze changes
        analysis = self.analyze_action_impact(before_state, after_state, success)
        
        return result, analysis, success
    
    def analyze_action_impact(self, before_state, after_state, success):
        """Analyze the impact of an action"""
        if not success or not after_state:
            return {
                'success': False,
                'phase_changed': False,
                'hand_changed': False,
                'stage_changed': False,
                'energy_changed': False,
                'life_changed': False
            }
        
        analysis = {
            'success': True,
            'phase_changed': before_state.get('phase') != after_state.get('phase'),
            'hand_changed': len(before_state.get('player1', {}).get('hand', {}).get('cards', [])) != len(after_state.get('player1', {}).get('hand', {}).get('cards', [])),
            'stage_changed': self.count_stage_cards(before_state.get('player1', {})) != self.count_stage_cards(after_state.get('player1', {})),
            'energy_changed': len(before_state.get('player1', {}).get('energy', {}).get('cards', [])) != len(after_state.get('player1', {}).get('energy', {}).get('cards', [])),
            'life_changed': len(before_state.get('player1', {}).get('life_zone', {}).get('cards', [])) != len(after_state.get('player1', {}).get('life_zone', {}).get('cards', []))
        }
        
        # Add specific changes
        if analysis['phase_changed']:
            analysis['phase_transition'] = f"{before_state.get('phase')} -> {after_state.get('phase')}"
        
        if analysis['stage_changed']:
            before_count = self.count_stage_cards(before_state.get('player1', {}))
            after_count = self.count_stage_cards(after_state.get('player1', {}))
            analysis['stage_change'] = f"Stage: {before_count} -> {after_count} cards"
        
        return analysis
    
    def count_stage_cards(self, player):
        """Count cards on stage"""
        stage = player.get('stage', {})
        count = 0
        for pos in ['left_side', 'center', 'right_side']:
            card = stage.get(pos)
            if card and isinstance(card, dict) and card.get('name'):
                count += 1
        return count
    
    def find_and_test_abilities(self):
        """Find and test abilities in current game state"""
        state, actions = get_state_and_actions()
        if not state:
            return []
        
        ability_tests = []
        
        # Look for ability actions
        for i, action in enumerate(actions):
            action_type = action.get('action_type', '').lower()
            description = action.get('description', '')
            
            if 'ability' in action_type or 'use_ability' in action_type or '{{kidou' in description or '{{jidou' in description:
                test_result = self.test_ability(action, i, state)
                ability_tests.append(test_result)
        
        return ability_tests
    
    def test_ability(self, action, action_index, state):
        """Test a specific ability"""
        action_type = action.get('action_type', '')
        description = action.get('description', '')
        
        # Extract ability information
        ability_info = {
            'action': action,
            'action_index': action_index,
            'description': description,
            'trigger_type': self.extract_trigger_type(description),
            'predicted_effect': self.predict_ability_effect(description),
            'requirements': self.extract_ability_requirements(description)
        }
        
        # Check if requirements are met
        requirements_met = self.check_ability_requirements(ability_info['requirements'], state)
        ability_info['requirements_met'] = requirements_met
        
        if requirements_met:
            # Execute ability
            result, analysis, success = self.execute_and_analyze_action(action_index, action_type)
            ability_info['execution_result'] = result
            ability_info['execution_analysis'] = analysis
            ability_info['execution_success'] = success
        else:
            ability_info['execution_result'] = 'Requirements not met'
            ability_info['execution_success'] = False
        
        return ability_info
    
    def extract_trigger_type(self, description):
        """Extract trigger type from ability description"""
        if '{{kidou' in description:
            return 'Activation'
        elif '{{jidou' in description:
            return 'Automatic'
        elif '{{joki' in description:
            return 'Continuous'
        else:
            return 'Unknown'
    
    def predict_ability_effect(self, description):
        """Predict what effect the ability will have"""
        desc_lower = description.lower()
        
        if 'draw' in desc_lower:
            return 'Draw cards'
        elif 'damage' in desc_lower or 'blade' in desc_lower:
            return 'Deal damage'
        elif 'heal' in desc_lower or 'life' in desc_lower:
            return 'Gain life'
        elif 'energy' in desc_lower:
            return 'Manipulate energy'
        elif 'stage' in desc_lower:
            return 'Manipulate stage'
        else:
            return 'Unknown effect'
    
    def extract_ability_requirements(self, description):
        """Extract requirements from ability description"""
        requirements = {
            'needs_stage': False,
            'needs_hand': False,
            'needs_energy': 0,
            'exclude_self': False,
            'target_count': 0
        }
        
        desc_lower = description.lower()
        
        if 'stage' in desc_lower:
            requirements['needs_stage'] = True
        if 'hand' in desc_lower:
            requirements['needs_hand'] = True
        if 'energy' in desc_lower:
            requirements['needs_energy'] = self.extract_cost(description)
        if 'exclude_self' in desc_lower or 'excluding self' in desc_lower:
            requirements['exclude_self'] = True
        
        # Extract target count
        import re
        count_match = re.search(r'(\d+)\s+(?:cards?|members?)', description)
        if count_match:
            requirements['target_count'] = int(count_match.group(1))
        
        return requirements
    
    def check_ability_requirements(self, requirements, state):
        """Check if ability requirements are met"""
        # Check energy requirements
        if requirements['needs_energy'] > 0:
            active_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            if active_energy < requirements['needs_energy']:
                return False
        
        # Check stage requirements
        if requirements['needs_stage']:
            stage_cards = self.count_stage_cards(state.get('player1', {}))
            if requirements['exclude_self']:
                # Need other cards besides self
                if stage_cards <= requirements['target_count']:
                    return False
            elif stage_cards == 0:
                return False
        
        return True
    
    def generate_documentation(self):
        """Generate comprehensive game documentation"""
        analysis, state, actions = self.analyze_current_state()
        if not analysis:
            return "No game state available"
        
        doc = []
        doc.append("# GAME ANALYSIS DOCUMENTATION")
        doc.append(f"Generated: Turn {analysis['turn']}, Phase {analysis['phase']}")
        doc.append("")
        
        # Game state overview
        doc.append("## GAME STATE OVERVIEW")
        doc.append(f"- **Turn**: {analysis['turn']}")
        doc.append(f"- **Phase**: {analysis['phase']}")
        doc.append(f"- **P1 Life**: {analysis['winning_conditions']['p1_life_count']}")
        doc.append(f"- **P2 Life**: {analysis['winning_conditions']['p2_life_count']}")
        doc.append(f"- **Life Advantage**: {analysis['winning_conditions']['life_advantage']}")
        doc.append("")
        
        # Player analysis
        doc.append("## PLAYER ANALYSIS")
        for player in [analysis['player1'], analysis['player2']]:
            doc.append(f"### {player['name']}")
            doc.append(f"- **Hand**: {player['hand_count']} cards")
            doc.append(f"- **Stage**: {player['stage_count']} cards")
            doc.append(f"- **Energy**: {player['energy_active']}/{player['energy_total']} active")
            doc.append(f"- **Discard**: {player['discard_count']} cards")
            doc.append(f"- **Waiting**: {player['waiting_count']} cards")
            doc.append("")
        
        # Available actions
        doc.append("## AVAILABLE ACTIONS")
        predictions = self.predict_action_outcomes(actions, state)
        for i, pred in enumerate(predictions):
            action = pred['action']
            doc.append(f"### {i+1}. {action.get('action_type', 'Unknown')}")
            doc.append(f"**Description**: {action.get('description', 'No description')}")
            doc.append(f"**Predicted Outcome**: {pred['predicted_outcome']}")
            doc.append(f"**Confidence**: {pred['confidence']:.0%}")
            if pred['risks']:
                doc.append(f"**Risks**: {', '.join(pred['risks'])}")
            if pred['opportunities']:
                doc.append(f"**Opportunities**: {', '.join(pred['opportunities'])}")
            doc.append("")
        
        # Ability tests
        ability_tests = self.find_and_test_abilities()
        if ability_tests:
            doc.append("## ABILITY TESTS")
            for test in ability_tests:
                doc.append(f"### {test['trigger_type']} Ability")
                doc.append(f"**Description**: {test['description']}")
                doc.append(f"**Requirements Met**: {test['requirements_met']}")
                doc.append(f"**Predicted Effect**: {test['predicted_effect']}")
                if test.get('execution_result'):
                    doc.append(f"**Execution Result**: {test['execution_result']}")
                    doc.append(f"**Execution Success**: {test['execution_success']}")
                doc.append("")
        
        # Winning strategy analysis
        doc.append("## WINNING STRATEGY ANALYSIS")
        doc.append(f"**Critical Phase**: {analysis['winning_conditions']['critical_phase']}")
        doc.append(f"**Tempo Advantage**: {analysis['winning_conditions']['tempo_advantage']['p1_tempo']}")
        doc.append(f"**Stage Advantage**: {analysis['winning_conditions']['tempo_advantage']['p1_stage_advantage']}")
        doc.append("")
        
        # Problems found
        if self.problems_found:
            doc.append("## PROBLEMS FOUND")
            for problem in self.problems_found:
                doc.append(f"- **{problem['type']}**: {problem['description']}")
                if problem.get('fix'):
                    doc.append(f"  - **Fix**: {problem['fix']}")
            doc.append("")
        
        return "\n".join(doc)

def run_comprehensive_analysis():
    """Run comprehensive game analysis and documentation"""
    analyzer = GameAnalyzer()
    
    print("=== COMPREHENSIVE GAME ANALYSIS ===")
    
    # Analyze current state
    analysis, state, actions = analyzer.analyze_current_state()
    if not analysis:
        print("No game state available")
        return
    
    print(f"\n=== CURRENT STATE ===")
    print(f"Turn: {analysis['turn']}, Phase: {analysis['phase']}")
    print(f"P1: {analysis['player1']['hand_count']} hand, {analysis['player1']['stage_count']} stage, {analysis['player1']['energy_active']}/{analysis['player1']['energy_total']} energy")
    print(f"P2: {analysis['player2']['hand_count']} hand, {analysis['player2']['stage_count']} stage, {analysis['player2']['energy_active']}/{analysis['player2']['energy_total']} energy")
    
    # Test abilities
    print(f"\n=== ABILITY TESTING ===")
    ability_tests = analyzer.find_and_test_abilities()
    print(f"Found {len(ability_tests)} abilities to test")
    
    for test in ability_tests:
        print(f"\n--- {test['trigger_type']} Ability ---")
        print(f"Description: {test['description']}")
        print(f"Requirements met: {test['requirements_met']}")
        print(f"Predicted effect: {test['predicted_effect']}")
        
        if test.get('execution_result'):
            print(f"Execution result: {test['execution_result']}")
            print(f"Execution success: {test['execution_success']}")
    
    # Generate documentation
    print(f"\n=== GENERATING DOCUMENTATION ===")
    documentation = analyzer.generate_documentation()
    
    # Save documentation
    with open('game_analysis_documentation.md', 'w', encoding='utf-8') as f:
        f.write(documentation)
    
    print("Documentation saved to game_analysis_documentation.md")
    
    # Show summary
    print(f"\n=== ANALYSIS SUMMARY ===")
    print(f"Total actions available: {len(actions)}")
    print(f"Abilities found: {len(ability_tests)}")
    print(f"Problems identified: {len(analyzer.problems_found)}")
    
    return analyzer, analysis, ability_tests

if __name__ == "__main__":
    run_comprehensive_analysis()

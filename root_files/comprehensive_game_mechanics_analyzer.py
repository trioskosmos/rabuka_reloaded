#!/usr/bin/env python3
"""
Comprehensive Game Mechanics Analyzer for Rabuka Card Game
This tool analyzes the game state, rules, abilities, and their implementation to:
1. Understand how the game works fundamentally
2. Verify ability texts match actual behavior
3. Identify discrepancies between rules and implementation
4. Provide predictive capabilities for game actions
"""

import requests
import json
import re
from typing import Dict, Any, List, Optional, Tuple
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

class GameMechanicsAnalyzer:
    def __init__(self):
        self.game_rules = self.load_rules()
        self.qa_data = self.load_qa_data()
        self.card_database = self.load_card_database()
        self.ability_patterns = self.load_ability_patterns()
        
    def load_rules(self) -> Dict[str, Any]:
        """Load and parse the game rules from rules.txt"""
        try:
            with open('engine/rules/rules.txt', 'r', encoding='utf-8') as f:
                rules_content = f.read()
            
            # Parse key rule sections
            rules = {
                'victory_conditions': self.extract_victory_conditions(rules_content),
                'phases': self.extract_phase_flow(rules_content),
                'card_types': self.extract_card_types(rules_content),
                'zones': self.extract_game_zones(rules_content),
                'actions': self.extract_specific_actions(rules_content),
                'abilities': self.extract_ability_rules(rules_content)
            }
            return rules
        except Exception as e:
            print(f"Error loading rules: {e}")
            return {}
    
    def load_qa_data(self) -> List[Dict[str, Any]]:
        """Load QA data for reference"""
        try:
            with open('cards/qa_data.json', 'r', encoding='utf-8') as f:
                return json.load(f)
        except Exception as e:
            print(f"Error loading QA data: {e}")
            return []
    
    def load_card_database(self) -> Dict[str, Any]:
        """Load card database for ability analysis"""
        try:
            with open('card_id_mapping.json', 'r', encoding='utf-8') as f:
                return json.load(f)
        except Exception as e:
            print(f"Error loading card database: {e}")
            return {}
    
    def load_ability_patterns(self) -> Dict[str, Any]:
        """Load ability parsing patterns from parser.py"""
        # This would be extracted from the parser.py analysis
        return {
            'source_patterns': ['hand', 'deck', 'discard', 'stage', 'energy_zone'],
            'destination_patterns': ['hand', 'deck', 'discard', 'stage', 'energy_zone'],
            'action_types': ['move', 'draw', 'shuffle', 'reveal', 'wait', 'active'],
            'condition_types': ['count', 'location', 'state', 'temporal', 'comparison'],
            'ability_types': ['auto', 'activate', 'constant']
        }
    
    def extract_victory_conditions(self, rules_text: str) -> List[str]:
        """Extract victory conditions from rules"""
        conditions = []
        # Look for victory-related sections
        if '1.2.1' in rules_text:
            # Extract the victory condition text
            lines = rules_text.split('\n')
            for i, line in enumerate(lines):
                if '1.2.1' in line and '3' in line and '2' in line:
                    # Victory condition about 3+ cards vs 2 or fewer
                    conditions.append("3+ successful live cards vs opponent's 2 or fewer")
                    break
        return conditions
    
    def extract_phase_flow(self, rules_text: str) -> List[str]:
        """Extract game phase flow from rules"""
        phases = []
        # Look for phase-related sections
        if '7.2' in rules_text or '7.3' in rules_text:
            phases.extend(['RockPaperScissors', 'ChooseFirstAttacker', 'LiveCardSet', 
                          'MainPhase', 'PerformancePhase', 'LivePhase', 'EndPhase'])
        return phases
    
    def extract_card_types(self, rules_text: str) -> List[str]:
        """Extract card types from rules"""
        types = []
        if '2.2.2' in rules_text:
            types.extend(['member', 'live', 'energy'])
        return types
    
    def extract_game_zones(self, rules_text: str) -> List[str]:
        """Extract game zones from rules"""
        zones = []
        # Look for zone definitions in section 4
        if '4.5' in rules_text: zones.append('member_area')
        if '4.6' in rules_text: zones.append('live_card_zone') 
        if '4.7' in rules_text: zones.append('energy_zone')
        if '4.8' in rules_text: zones.append('main_deck')
        if '4.9' in rules_text: zones.append('energy_deck')
        if '4.10' in rules_text: zones.append('success_live_zone')
        if '4.11' in rules_text: zones.append('hand')
        if '4.12' in rules_text: zones.append('discard')
        return zones
    
    def extract_specific_actions(self, rules_text: str) -> List[str]:
        """Extract specific actions from rules"""
        actions = []
        # Look for section 5
        if '5.2' in rules_text: actions.append('activate/wait')
        if '5.3' in rules_text: actions.append('face_up/down')
        if '5.4' in rules_text: actions.append('place')
        if '5.5' in rules_text: actions.append('shuffle')
        if '5.6' in rules_text: actions.append('draw')
        if '5.7' in rules_text: actions.append('look_at_top')
        if '5.8' in rules_text: actions.append('swap')
        if '5.9' in rules_text: actions.append('pay_energy')
        if '5.10' in rules_text: actions.append('place_under_member')
        return actions
    
    def extract_ability_rules(self, rules_text: str) -> List[str]:
        """Extract ability-related rules"""
        rules = []
        # Look for ability timing and resolution rules
        if '9.' in rules_text:
            rules.append('ability_timing_and_resolution')
        return rules
    
    def get_current_game_state(self) -> Optional[Dict[str, Any]]:
        """Get current game state from server"""
        try:
            response = requests.get(f"{BASE_URL}/api/game-state")
            if response.status_code == 200:
                return response.json()
        except Exception as e:
            print(f"Error getting game state: {e}")
        return None
    
    def get_available_actions(self) -> List[Dict[str, Any]]:
        """Get available actions from server"""
        try:
            response = requests.get(f"{BASE_URL}/api/actions")
            if response.status_code == 200:
                return response.json().get('actions', [])
        except Exception as e:
            print(f"Error getting actions: {e}")
        return []
    
    def analyze_game_state(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Comprehensive analysis of current game state"""
        analysis = {
            'phase': state.get('phase', 'Unknown'),
            'turn_info': self.extract_turn_info(state),
            'player_states': self.analyze_player_states(state),
            'zone_analysis': self.analyze_zones(state),
            'action_predictions': self.predict_actions(state),
            'winning_conditions': self.check_winning_conditions(state)
        }
        return analysis
    
    def extract_turn_info(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Extract turn-related information"""
        return {
            'current_player': state.get('current_player'),
            'turn_count': state.get('turn_count', 0),
            'phase': state.get('phase'),
            'sub_phase': state.get('sub_phase')
        }
    
    def analyze_player_states(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze both player states"""
        analysis = {}
        for player_key in ['player1', 'player2']:
            player = state.get(player_key, {})
            analysis[player_key] = {
                'hand_size': len(player.get('hand', {}).get('cards', [])),
                'stage_members': self.analyze_stage(player.get('stage', {})),
                'energy_count': self.count_energy(player.get('energy_zone', {})),
                'live_cards': len(player.get('live_card_zone', {}).get('cards', [])),
                'success_live_cards': len(player.get('success_live_zone', {}).get('cards', [])),
                'deck_size': len(player.get('deck', {}).get('cards', []))
            }
        return analysis
    
    def analyze_stage(self, stage: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze stage composition"""
        members = []
        for position in ['left_side', 'center', 'right_side']:
            card = stage.get(position)
            if card and card.get('name'):
                members.append({
                    'position': position,
                    'name': card.get('name'),
                    'cost': card.get('cost', 0),
                    'hearts': card.get('hearts', []),
                    'blades': card.get('blades', 0),
                    'state': card.get('state', 'active'),
                    'abilities': card.get('abilities', [])
                })
        return {
            'count': len(members),
            'members': members,
            'total_cost': sum(m['cost'] for m in members),
            'total_blades': sum(m['blades'] for m in members)
        }
    
    def count_energy(self, energy_zone: Dict[str, Any]) -> Dict[str, Any]:
        """Count energy cards by state"""
        cards = energy_zone.get('cards', [])
        active = sum(1 for card in cards if card.get('state') == 'active')
        wait = sum(1 for card in cards if card.get('state') == 'wait')
        return {
            'total': len(cards),
            'active': active,
            'wait': wait
        }
    
    def analyze_zones(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze all game zones"""
        return {
            'hand_analysis': self.analyze_hands(state),
            'deck_analysis': self.analyze_decks(state),
            'discard_analysis': self.analyze_discards(state),
            'energy_analysis': self.analyze_energy_zones(state)
        }
    
    def analyze_hands(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze hand contents"""
        analysis = {}
        for player_key in ['player1', 'player2']:
            hand = state.get(player_key, {}).get('hand', {}).get('cards', [])
            analysis[player_key] = {
                'count': len(hand),
                'card_types': self.categorize_cards(hand),
                'playable_members': self.count_playable_members(hand, state.get(player_key, {}))
            }
        return analysis
    
    def categorize_cards(self, cards: List[Dict[str, Any]]) -> Dict[str, int]:
        """Categorize cards by type"""
        types = {'member': 0, 'live': 0, 'energy': 0}
        for card in cards:
            card_type = card.get('card_type', '').lower()
            if card_type in types:
                types[card_type] += 1
        return types
    
    def count_playable_members(self, hand: List[Dict[str, Any]], player_state: Dict[str, Any]) -> int:
        """Count playable member cards based on available energy"""
        energy_active = player_state.get('energy_zone', {}).get('cards', [])
        active_energy = sum(1 for card in energy_active if card.get('state') == 'active')
        
        playable = 0
        for card in hand:
            if card.get('card_type') == 'member' and card.get('cost', 0) <= active_energy:
                playable += 1
        return playable
    
    def analyze_decks(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze deck contents"""
        analysis = {}
        for player_key in ['player1', 'player2']:
            deck = state.get(player_key, {}).get('deck', {}).get('cards', [])
            analysis[player_key] = {
                'count': len(deck),
                'card_types': self.categorize_cards(deck)
            }
        return analysis
    
    def analyze_discards(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze discard zones"""
        analysis = {}
        for player_key in ['player1', 'player2']:
            discard = state.get(player_key, {}).get('discard', {}).get('cards', [])
            analysis[player_key] = {
                'count': len(discard),
                'card_types': self.categorize_cards(discard)
            }
        return analysis
    
    def analyze_energy_zones(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze energy zones"""
        analysis = {}
        for player_key in ['player1', 'player2']:
            energy_zone = state.get(player_key, {}).get('energy_zone', {}).get('cards', [])
            analysis[player_key] = self.count_energy({'cards': energy_zone})
        return analysis
    
    def predict_actions(self, state: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Predict likely next actions based on game state"""
        predictions = []
        phase = state.get('phase', '')
        
        if 'RockPaperScissors' in phase:
            predictions.append({'action': 'play_rps', 'likelihood': 1.0, 'reason': 'RPS phase required'})
        elif 'ChooseFirstAttacker' in phase:
            predictions.append({'action': 'choose_first_attacker', 'likelihood': 1.0, 'reason': 'Must choose attacker'})
        elif 'Mulligan' in phase:
            predictions.extend([
                {'action': 'skip_mulligan', 'likelihood': 0.7, 'reason': 'Common strategy'},
                {'action': 'mulligan', 'likelihood': 0.3, 'reason': 'If hand is poor'}
            ])
        elif 'LiveCardSet' in phase:
            predictions.append({'action': 'set_live_card', 'likelihood': 1.0, 'reason': 'Must set live card'})
        elif 'Main' in phase:
            # Analyze main phase options
            p1_state = state.get('player1', {})
            playable_members = self.count_playable_members(
                p1_state.get('hand', {}).get('cards', []), 
                p1_state
            )
            if playable_members > 0:
                predictions.append({'action': 'play_member', 'likelihood': 0.8, 'reason': f'{playable_members} playable members'})
            predictions.append({'action': 'pass', 'likelihood': 0.2, 'reason': 'Alternative option'})
        elif 'Performance' in phase:
            predictions.append({'action': 'pass_performance', 'likelihood': 0.6, 'reason': 'Common strategy'})
            predictions.append({'action': 'use_ability', 'likelihood': 0.4, 'reason': 'If abilities available'})
        
        return predictions
    
    def check_winning_conditions(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Check if winning conditions are met"""
        analysis = {'p1_winning': False, 'p2_winning': False, 'reason': ''}
        
        p1_success = len(state.get('player1', {}).get('success_live_zone', {}).get('cards', []))
        p2_success = len(state.get('player2', {}).get('success_live_zone', {}).get('cards', []))
        
        # Check victory condition: 3+ vs 2 or fewer
        if p1_success >= 3 and p2_success <= 2:
            analysis['p1_winning'] = True
            analysis['reason'] = f'P1 has {p1_success} success cards vs P2\'s {p2_success}'
        elif p2_success >= 3 and p1_success <= 2:
            analysis['p2_winning'] = True
            analysis['reason'] = f'P2 has {p2_success} success cards vs P1\'s {p1_success}'
        elif p1_success >= 3 and p2_success >= 3:
            analysis['reason'] = f'Draw: both have 3+ cards ({p1_success} vs {p2_success})'
        
        return analysis
    
    def analyze_ability_implementation(self, card_id: str) -> Dict[str, Any]:
        """Analyze how a specific card's abilities are implemented"""
        # Get card data - card_id_mapping.json has a different structure
        card_data = self.card_database.get(card_id, 0)
        if isinstance(card_data, int):
            # The mapping just maps to an index, need to get actual card data differently
            return {'error': f'Card {card_id} mapping found but actual card data not accessible with current structure'}
        
        if not card_data:
            return {'error': f'Card {card_id} not found'}
        
        abilities = card_data.get('abilities', [])
        analysis = {
            'card_id': card_id,
            'card_name': card_data.get('name', 'Unknown'),
            'abilities': []
        }
        
        for ability in abilities:
            ability_analysis = {
                'text': ability.get('text', ''),
                'type': ability.get('type', ''),
                'parsed_structure': self.parse_ability_text(ability.get('text', '')),
                'implementation_gaps': self.check_implementation_gaps(ability),
                'qa_references': self.find_qa_references(ability.get('text', ''))
            }
            analysis['abilities'].append(ability_analysis)
        
        return analysis
    
    def parse_ability_text(self, text: str) -> Dict[str, Any]:
        """Parse ability text using the parser patterns"""
        # This would integrate with the parser.py functionality
        structure = {
            'has_cost': ':' in text,
            'has_condition': any(marker in text for marker in ['if', 'when', 'if', 'case']),
            'components': self.extract_ability_components(text)
        }
        return structure
    
    def extract_ability_components(self, text: str) -> List[Dict[str, Any]]:
        """Extract individual components from ability text"""
        components = []
        # Simplified component extraction
        if 'draw' in text:
            components.append({'type': 'draw', 'target': 'self'})
        if 'shuffle' in text:
            components.append({'type': 'shuffle', 'target': 'deck'})
        if 'wait' in text or 'active' in text:
            components.append({'type': 'state_change', 'effect': 'wait/active'})
        return components
    
    def check_implementation_gaps(self, ability: Dict[str, Any]) -> List[str]:
        """Check for gaps between ability text and implementation"""
        gaps = []
        text = ability.get('text', '')
        
        # Check for complex patterns that might not be implemented
        if 'complex condition' in text.lower():
            gaps.append('Complex conditional logic may not be fully implemented')
        if 'multiple effects' in text.lower():
            gaps.append('Multiple effect resolution order may need verification')
        if 'optional' in text.lower():
            gaps.append('Optional effect handling needs testing')
        
        return gaps
    
    def find_qa_references(self, ability_text: str) -> List[Dict[str, Any]]:
        """Find related QA entries for an ability"""
        references = []
        for qa in self.qa_data:
            # Simple text matching - could be improved
            if any(word in ability_text for word in qa.get('question', '').split()[:5]):
                references.append({
                    'id': qa.get('id'),
                    'question': qa.get('question', ''),
                    'answer': qa.get('answer', ''),
                    'relevance': 'high'
                })
        return references
    
    def test_ability_execution(self, card_id: str, ability_index: int = 0) -> Dict[str, Any]:
        """Test ability execution in actual game"""
        # This would set up a game state and test the ability
        return {
            'test_result': 'not_implemented',
            'note': 'Ability execution testing requires game state setup'
        }
    
    def generate_comprehensive_report(self) -> Dict[str, Any]:
        """Generate comprehensive analysis report"""
        current_state = self.get_current_game_state()
        if not current_state:
            return {'error': 'No game state available'}
        
        return {
            'timestamp': str(datetime.now()),
            'game_state_analysis': self.analyze_game_state(current_state),
            'rules_compliance': self.check_rules_compliance(current_state),
            'ability_analysis': self.analyze_all_abilities(),
            'recommendations': self.generate_recommendations(current_state),
            'discrepancies': self.identify_discrepancies()
        }
    
    def check_rules_compliance(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Check if current state complies with rules"""
        compliance = {
            'valid_phase': state.get('phase') in self.rules.get('phases', []),
            'valid_zone_counts': self.validate_zone_counts(state),
            'valid_turn_sequence': self.validate_turn_sequence(state)
        }
        return compliance
    
    def validate_zone_counts(self, state: Dict[str, Any]) -> bool:
        """Validate zone counts against rules"""
        # Check hand limits, deck sizes, etc.
        return True  # Simplified
    
    def validate_turn_sequence(self, state: Dict[str, Any]) -> bool:
        """Validate turn sequence"""
        # Check if phase transitions are valid
        return True  # Simplified
    
    def analyze_all_abilities(self) -> Dict[str, Any]:
        """Analyze all card abilities"""
        return {
            'total_cards': len(self.card_database),
            'abilities_analyzed': 0,  # Would be populated by actual analysis
            'common_patterns': self.identify_common_ability_patterns(),
            'complex_abilities': self.identify_complex_abilities()
        }
    
    def identify_common_ability_patterns(self) -> List[str]:
        """Identify common ability patterns"""
        return ['draw_cards', 'shuffle_deck', 'state_change', 'card_movement']
    
    def identify_complex_abilities(self) -> List[str]:
        """Identify complex abilities that need special attention"""
        return ['multi_step_abilities', 'conditional_abilities', 'optional_abilities']
    
    def generate_recommendations(self, state: Dict[str, Any]) -> List[str]:
        """Generate strategic recommendations"""
        recommendations = []
        
        # Analyze current position
        p1_success = len(state.get('player1', {}).get('success_live_zone', {}).get('cards', []))
        p2_success = len(state.get('player2', {}).get('success_live_zone', {}).get('cards', []))
        
        if p1_success < p2_success:
            recommendations.append("Focus on succeeding live cards to catch up")
        elif p1_success > p2_success:
            recommendations.append("Maintain advantage, consider defensive play")
        
        phase = state.get('phase', '')
        if 'Main' in phase:
            recommendations.append("Consider playing members with useful abilities")
        
        return recommendations
    
    def identify_discrepancies(self) -> List[Dict[str, Any]]:
        """Identify discrepancies between rules, QA, and implementation"""
        discrepancies = []
        
        # Compare QA answers with what might be implemented
        for qa in self.qa_data[:10]:  # Check first 10 for now
            question = qa.get('question', '')
            answer = qa.get('answer', '')
            
            # Look for specific implementation issues
            if 'cost' in question.lower() and '0' in answer:
                discrepancies.append({
                    'type': 'cost_calculation',
                    'qa_id': qa.get('id'),
                    'issue': 'Cost calculation edge case',
                    'expected': answer
                })
        
        return discrepancies

def main():
    """Main analysis function"""
    analyzer = GameMechanicsAnalyzer()
    
    print("=== Comprehensive Game Mechanics Analysis ===")
    
    # Get current game state
    state = analyzer.get_current_game_state()
    if state:
        print(f"Current phase: {state.get('phase')}")
        
        # Analyze current state
        analysis = analyzer.analyze_game_state(state)
        print(f"Turn info: {analysis['turn_info']}")
        print(f"Player states: {analysis['player_states']}")
        print(f"Winning conditions: {analysis['winning_conditions']}")
        
        # Show action predictions
        predictions = analyzer.predict_actions(state)
        print(f"Predicted actions: {predictions}")
    else:
        print("No game state available - server may not be running")
    
    # Analyze some sample abilities
    print("\n=== Ability Analysis ===")
    sample_cards = ['PL!N-bp1-001-R', 'PL!-bp1-001-R']  # Sample card IDs
    for card_id in sample_cards:
        if card_id in analyzer.card_database:
            ability_analysis = analyzer.analyze_ability_implementation(card_id)
            print(f"Card {card_id}: {ability_analysis.get('card_name', 'Unknown')}")
            for ability in ability_analysis.get('abilities', []):
                print(f"  - {ability.get('text', '')[:50]}...")
    
    # Generate recommendations
    if state:
        recommendations = analyzer.generate_recommendations(state)
        print(f"\nRecommendations: {recommendations}")

if __name__ == "__main__":
    main()

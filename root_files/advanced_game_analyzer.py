import requests
import json
from fast_game_tools import get_state_and_actions, execute_action_and_get_state, find_action_by_type, find_action_by_description

BASE_URL = "http://localhost:8080"

class AdvancedGameAnalyzer:
    def __init__(self):
        self.game_history = []
        self.action_predictions = []
        self.ability_verifications = []
        self.rules_compliance = []
        self.problems_fixed = []
        
    def analyze_game_state_deep(self):
        """Deep analysis of current game state with winning strategy focus"""
        state, actions = get_state_and_actions()
        if not state:
            return None, None, None
            
        # Comprehensive state analysis
        analysis = {
            'turn': state.get('turn', 0),
            'phase': state.get('phase', 'Unknown'),
            'players': {
                'p1': self.analyze_player_deep(state.get('player1', {}), 'P1'),
                'p2': self.analyze_player_deep(state.get('player2', {}), 'P2')
            },
            'phase_analysis': self.analyze_phase_deep(state),
            'winning_analysis': self.analyze_winning_conditions_deep(state),
            'tempo_analysis': self.analyze_tempo_deep(state),
            'resource_analysis': self.analyze_resources_deep(state),
            'strategic_position': self.analyze_strategic_position(state)
        }
        
        return analysis, state, actions
    
    def analyze_player_deep(self, player_data, player_name):
        """Deep player analysis with strategic metrics"""
        hand = player_data.get('hand', {}).get('cards', [])
        stage = player_data.get('stage', {})
        energy = player_data.get('energy', {}).get('cards', [])
        discard = player_data.get('discard', {}).get('cards', [])
        waiting = player_data.get('waitroom', {}).get('cards', [])
        life = player_data.get('life_zone', {}).get('cards', [])
        
        # Stage composition analysis
        stage_cards = []
        for pos, card in [('left', stage.get('left_side')), ('center', stage.get('center')), ('right', stage.get('right_side'))]:
            if card and isinstance(card, dict) and card.get('name'):
                stage_cards.append({
                    'position': pos,
                    'name': card.get('name'),
                    'id': card.get('id'),
                    'card_type': card.get('type'),
                    'hearts': card.get('base_heart'),
                    'blade': card.get('blade'),
                    'strategic_value': self.calculate_card_strategic_value(card)
                })
        
        # Energy analysis
        active_energy = len([e for e in energy if isinstance(e, dict) and e.get('orientation') == 'Active'])
        total_energy = len(energy)
        energy_efficiency = active_energy / total_energy if total_energy > 0 else 0
        
        # Hand quality analysis
        hand_quality = self.analyze_hand_quality(hand)
        
        return {
            'name': player_name,
            'hand_count': len(hand),
            'hand_quality': hand_quality,
            'hand_cards': hand,
            'stage_count': len(stage_cards),
            'stage_cards': stage_cards,
            'stage_power': sum(card.get('strategic_value', 0) for card in stage_cards),
            'energy_total': total_energy,
            'energy_active': active_energy,
            'energy_efficiency': energy_efficiency,
            'discard_count': len(discard),
            'waiting_count': len(waiting),
            'life_count': len(life),
            'deck_count': player_data.get('main_deck_count', 0),
            'resource_position': self.calculate_resource_position(active_energy, len(hand), len(stage_cards))
        }
    
    def calculate_card_strategic_value(self, card):
        """Calculate strategic value of a card"""
        if not isinstance(card, dict):
            return 0
            
        value = 0
        
        # Base value from blade
        value += card.get('blade', 0)
        
        # Value from hearts
        hearts = card.get('base_heart')
        if hearts and isinstance(hearts, dict):
            value += len(hearts) * 2  # Each heart color worth 2 points
        
        # Card type value
        card_type = card.get('type', '').lower()
        if 'member' in card_type:
            value += 3  # Members are valuable for stage presence
        elif 'live' in card_type:
            value += 5  # Live cards are valuable for performance
        
        return value
    
    def analyze_hand_quality(self, hand_cards):
        """Analyze quality of hand cards"""
        if not hand_cards:
            return {'score': 0, 'analysis': 'Empty hand'}
        
        total_value = 0
        member_count = 0
        live_count = 0
        energy_count = 0
        
        for card in hand_cards:
            if isinstance(card, dict):
                total_value += self.calculate_card_strategic_value(card)
                card_type = card.get('type', '').lower()
                if 'member' in card_type:
                    member_count += 1
                elif 'live' in card_type:
                    live_count += 1
                elif 'energy' in card_type:
                    energy_count += 1
        
        quality_score = total_value / len(hand_cards) if hand_cards else 0
        
        return {
            'score': quality_score,
            'total_value': total_value,
            'member_count': member_count,
            'live_count': live_count,
            'energy_count': energy_count,
            'analysis': f'Quality score {quality_score:.1f} with {member_count} members, {live_count} lives, {energy_count} energy'
        }
    
    def calculate_resource_position(self, energy, hand_size, stage_size):
        """Calculate overall resource position"""
        # Weighted scoring: energy (40%), hand (30%), stage (30%)
        energy_score = min(energy / 10, 1.0) * 0.4  # Max 10 energy considered optimal
        hand_score = min(hand_size / 8, 1.0) * 0.3    # Max 8 cards considered optimal
        stage_score = min(stage_size / 3, 1.0) * 0.3   # Max 3 stage cards considered optimal
        
        total_score = energy_score + hand_score + stage_score
        
        if total_score >= 0.8:
            position = 'Excellent'
        elif total_score >= 0.6:
            position = 'Good'
        elif total_score >= 0.4:
            position = 'Average'
        else:
            position = 'Poor'
        
        return {
            'score': total_score,
            'position': position,
            'components': {
                'energy': energy_score,
                'hand': hand_score,
                'stage': stage_score
            }
        }
    
    def analyze_phase_deep(self, state):
        """Deep phase analysis with strategic implications"""
        current_phase = state.get('phase', 'Unknown')
        
        phase_strategies = {
            'RockPaperScissors': {
                'importance': 'Low',
                'strategic_focus': 'RPS luck, no real strategy',
                'key_actions': ['rock_choice', 'paper_choice', 'scissors_choice'],
                'optimal_play': 'Choose randomly, no strategic advantage'
            },
            'ChooseFirstAttacker': {
                'importance': 'Medium',
                'strategic_focus': 'Tempo advantage selection',
                'key_actions': ['choose_first_attacker', 'choose_second_attacker'],
                'optimal_play': 'Choose first if you have strong early plays, second if you need information'
            },
            'MulliganP1Turn': {
                'importance': 'Medium',
                'strategic_focus': 'Hand optimization',
                'key_actions': ['select_mulligan', 'confirm_mulligan', 'skip_mulligan'],
                'optimal_play': 'Keep cards that match your strategy, mulligan expensive cards you can\'t play early'
            },
            'MulliganP2Turn': {
                'importance': 'Medium',
                'strategic_focus': 'Hand optimization with P1 information',
                'key_actions': ['select_mulligan', 'confirm_mulligan', 'skip_mulligan'],
                'optimal_play': 'Mulligan more aggressively if P1 kept good cards'
            },
            'Main': {
                'importance': 'High',
                'strategic_focus': 'Stage presence, tempo control, ability activation',
                'key_actions': ['play_member_to_stage', 'use_ability', 'pass'],
                'optimal_play': 'Play members to establish tempo, use abilities when advantageous, pass when no good plays'
            },
            'LiveCardSetP1Turn': {
                'importance': 'High',
                'strategic_focus': 'Performance preparation',
                'key_actions': ['set_live_card'],
                'optimal_play': 'Set live card that maximizes performance potential'
            },
            'LiveCardSetP2Turn': {
                'importance': 'High',
                'strategic_focus': 'Performance preparation',
                'key_actions': ['set_live_card'],
                'optimal_play': 'Set live card that counters P1 or maximizes your scoring'
            },
            'Performance': {
                'importance': 'Critical',
                'strategic_focus': 'Scoring, winning the game',
                'key_actions': ['execute_performance'],
                'optimal_play': 'Execute performance to maximize scoring and win condition'
            }
        }
        
        strategy = phase_strategies.get(current_phase, {
            'importance': 'Unknown',
            'strategic_focus': 'Unknown',
            'key_actions': [],
            'optimal_play': 'Unknown'
        })
        
        return {
            'current_phase': current_phase,
            'importance': strategy['importance'],
            'strategic_focus': strategy['strategic_focus'],
            'key_actions': strategy['key_actions'],
            'optimal_play': strategy['optimal_play'],
            'phase_number': self.get_phase_number(current_phase)
        }
    
    def get_phase_number(self, phase):
        """Get phase number for progression tracking"""
        phase_order = {
            'RockPaperScissors': 1,
            'ChooseFirstAttacker': 2,
            'MulliganP1Turn': 3,
            'MulliganP2Turn': 4,
            'Main': 5,
            'LiveCardSetP1Turn': 6,
            'LiveCardSetP2Turn': 7,
            'Performance': 8
        }
        return phase_order.get(phase, 0)
    
    def analyze_winning_conditions_deep(self, state):
        """Deep analysis of winning conditions and paths"""
        p1_life = len(state.get('player1', {}).get('life_zone', {}).get('cards', []))
        p2_life = len(state.get('player2', {}).get('life_zone', {}).get('cards', []))
        
        p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        p2_stage = len([c for c in [state.get('player2', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        
        # Calculate winning probabilities
        p1_win_prob = self.calculate_win_probability(p1_life, p2_life, p1_stage, p2_stage)
        p2_win_prob = 1 - p1_win_prob
        
        # Identify winning paths
        p1_paths = self.identify_winning_paths(p1_life, p2_life, p1_stage, p2_stage, 'P1')
        p2_paths = self.identify_winning_paths(p2_life, p1_life, p2_stage, p1_stage, 'P2')
        
        return {
            'p1_life': p1_life,
            'p2_life': p2_life,
            'life_advantage': p1_life - p2_life,
            'p1_stage': p1_stage,
            'p2_stage': p2_stage,
            'stage_advantage': p1_stage - p2_stage,
            'p1_win_probability': p1_win_prob,
            'p2_win_probability': p2_win_prob,
            'p1_winning_paths': p1_paths,
            'p2_winning_paths': p2_paths,
            'critical_turn': self.assess_critical_turn(state.get('turn', 0)),
            'winning_threshold': 7  # Standard life total
        }
    
    def calculate_win_probability(self, my_life, opp_life, my_stage, opp_stage):
        """Calculate win probability based on current state"""
        # Simple probability model based on life and stage advantage
        life_advantage = my_life - opp_life
        stage_advantage = my_stage - opp_stage
        
        # Base probability from life
        if my_life >= 7:
            life_prob = 0.9
        elif my_life >= 5:
            life_prob = 0.7
        elif my_life >= 3:
            life_prob = 0.5
        else:
            life_prob = 0.3
        
        # Stage advantage modifier
        stage_modifier = stage_advantage * 0.1  # Each stage card adds 10%
        
        # Combine probabilities
        total_prob = min(max(life_prob + stage_modifier, 0.1), 0.9)
        
        return total_prob
    
    def identify_winning_paths(self, my_life, opp_life, my_stage, opp_stage, player):
        """Identify potential winning paths"""
        paths = []
        
        # Path 1: Life advantage
        if my_life > opp_life:
            paths.append({
                'name': 'Life Advantage',
                'description': f'Win by maintaining life advantage ({my_life} vs {opp_life})',
                'requirements': ['Maintain life total', 'Prevent opponent scoring'],
                'probability': 'High' if my_life >= 5 else 'Medium'
            })
        
        # Path 2: Stage dominance
        if my_stage > opp_stage:
            paths.append({
                'name': 'Stage Dominance',
                'description': f'Win through stage control ({my_stage} vs {opp_stage} cards)',
                'requirements': ['Maintain stage presence', 'Use abilities effectively'],
                'probability': 'High' if my_stage >= 2 else 'Medium'
            })
        
        # Path 3: Comeback through performance
        if my_life < opp_life:
            paths.append({
                'name': 'Performance Comeback',
                'description': f'Comeback through performance scoring (need {opp_life - my_life + 1} life)',
                'requirements': ['Set up strong performance', 'Execute high-scoring live cards'],
                'probability': 'Medium'
            })
        
        # Path 4: Tempo victory
        if my_stage >= 2:
            paths.append({
                'name': 'Tempo Victory',
                'description': f'Win through tempo advantage and pressure',
                'requirements': ['Maintain stage pressure', 'Control game flow'],
                'probability': 'High'
            })
        
        return paths
    
    def assess_critical_turn(self, turn):
        """Assess if current turn is critical"""
        if turn <= 3:
            return 'Early - Setup phase'
        elif turn <= 6:
            return 'Mid-early - Establish tempo'
        elif turn <= 10:
            return 'Mid-game - Critical decisions'
        elif turn <= 15:
            return 'Late-mid - Push for advantage'
        else:
            return 'Late - Endgame decisions'
    
    def analyze_tempo_deep(self, state):
        """Deep tempo analysis"""
        p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        p2_stage = len([c for c in [state.get('player2', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        
        tempo_advantage = p1_stage - p2_stage
        
        return {
            'p1_stage_count': p1_stage,
            'p2_stage_count': p2_stage,
            'tempo_advantage': tempo_advantage,
            'tempo_leader': 'P1' if tempo_advantage > 0 else 'P2' if tempo_advantage < 0 else 'Even',
            'tempo_pressure': 'High' if abs(tempo_advantage) >= 2 else 'Medium' if abs(tempo_advantage) == 1 else 'Low',
            'tempo_control': self.assess_tempo_control(tempo_advantage, state.get('turn', 0))
        }
    
    def assess_tempo_control(self, advantage, turn):
        """Assess level of tempo control"""
        if advantage >= 2:
            return 'Dominant control'
        elif advantage == 1:
            return 'Slight control'
        elif advantage == 0:
            return 'Even contest'
        elif advantage == -1:
            return 'Slightly behind'
        else:
            return 'Losing tempo'
    
    def analyze_resources_deep(self, state):
        """Deep resource analysis"""
        p1_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
        p2_energy = len([e for e in state.get('player2', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
        
        p1_hand = len(state.get('player1', {}).get('hand', {}).get('cards', []))
        p2_hand = len(state.get('player2', {}).get('hand', {}).get('cards', []))
        
        return {
            'energy_advantage': p1_energy - p2_energy,
            'hand_advantage': p1_hand - p2_hand,
            'p1_energy_efficiency': p1_energy / 12 if p1_energy > 0 else 0,  # Assuming 12 max energy
            'p2_energy_efficiency': p2_energy / 12 if p2_energy > 0 else 0,
            'resource_balance': self.assess_resource_balance(p1_energy, p1_hand, p2_energy, p2_hand)
        }
    
    def assess_resource_balance(self, p1_energy, p1_hand, p2_energy, p2_hand):
        """Assess overall resource balance"""
        p1_total = p1_energy + p1_hand
        p2_total = p2_energy + p2_hand
        
        if p1_total > p2_total + 3:
            return 'P1 dominant'
        elif p1_total > p2_total:
            return 'P1 advantage'
        elif p2_total > p1_total + 3:
            return 'P2 dominant'
        elif p2_total > p1_total:
            return 'P2 advantage'
        else:
            return 'Even'
    
    def analyze_strategic_position(self, state):
        """Analyze overall strategic position"""
        p1_analysis = self.analyze_player_deep(state.get('player1', {}), 'P1')
        p2_analysis = self.analyze_player_deep(state.get('player2', {}), 'P2')
        
        # Calculate overall position scores
        p1_score = (
            p1_analysis['resource_position']['score'] * 0.4 +
            (p1_analysis['stage_power'] / 10) * 0.3 +
            (p1_analysis['hand_quality']['score'] / 10) * 0.3
        )
        
        p2_score = (
            p2_analysis['resource_position']['score'] * 0.4 +
            (p2_analysis['stage_power'] / 10) * 0.3 +
            (p2_analysis['hand_quality']['score'] / 10) * 0.3
        )
        
        position_advantage = p1_score - p2_score
        
        return {
            'p1_position_score': p1_score,
            'p2_position_score': p2_score,
            'position_advantage': position_advantage,
            'overall_leader': 'P1' if position_advantage > 0.1 else 'P2' if position_advantage < -0.1 else 'Even',
            'game_state': 'P1 Dominant' if position_advantage > 0.2 else 'P2 Dominant' if position_advantage < -0.2 else 'Balanced',
            'recommended_focus': self.get_recommended_focus(position_advantage, state.get('phase', ''))
        }
    
    def get_recommended_focus(self, advantage, phase):
        """Get recommended strategic focus based on position"""
        if advantage > 0.2:
            return 'Maintain advantage, press for win'
        elif advantage > 0:
            return 'Build on small advantage'
        elif advantage > -0.2:
            return 'Fight for tempo, find opening'
        else:
            return 'Defensive play, look for comeback opportunity'
    
    def predict_action_outcomes_deep(self, actions, state):
        """Deep prediction of action outcomes with reasoning"""
        predictions = []
        
        for action in actions:
            action_type = action.get('action_type', '')
            description = action.get('description', '')
            
            prediction = {
                'action': action,
                'predicted_outcome': self.predict_action_outcome_detailed(action, state),
                'reasoning': self.explain_action_reasoning(action, state),
                'confidence': self.calculate_prediction_confidence(action, state),
                'risks': self.identify_action_risks_detailed(action, state),
                'opportunities': self.identify_action_opportunities_detailed(action, state),
                'strategic_value': self.calculate_strategic_value(action, state),
                'alternatives': self.suggest_alternatives(action, state)
            }
            
            predictions.append(prediction)
        
        return predictions
    
    def predict_action_outcome_detailed(self, action, state):
        """Detailed prediction of action outcome"""
        action_type = action.get('action_type', '')
        description = action.get('description', '')
        
        if 'pass' in action_type:
            phase = state.get('phase', '')
            next_phase = self.predict_next_phase(phase)
            return f'Turn ends, phase advances from {phase} to {next_phase}'
        elif 'play_member_to_stage' in action_type:
            return self.predict_play_member_outcome(action, state)
        elif 'set_live_card' in action_type:
            return 'Live card set for performance phase, affects scoring potential'
        elif 'use_ability' in action_type:
            return 'Ability activated, effect depends on ability type and game state'
        elif 'rock_choice' in action_type or 'paper_choice' in action_type or 'scissors_choice' in action_type:
            return 'RPS choice recorded, 50% win chance, waiting for opponent'
        elif 'choose_first_attacker' in action_type:
            return 'First/second attacker chosen, affects tempo for rest of turn'
        elif 'skip_mulligan' in action_type:
            return 'Keep current hand, proceed to Main phase'
        else:
            return f'Unknown action: {action_type} - unpredictable outcome'
    
    def predict_play_member_outcome(self, action, state):
        """Predict outcome of playing member to stage"""
        description = action.get('description', '')
        p1_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
        p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        
        # Extract cost from description
        import re
        cost_match = re.search(r'Cost: [^:]+: (\d+)', description)
        cost = int(cost_match.group(1)) if cost_match else 0
        
        if p1_energy >= cost:
            stage_after = p1_stage + 1
            energy_after = p1_energy - cost
            return f'Member played to stage, {cost} energy spent. Stage: {p1_stage}->{stage_after}, Energy: {p1_energy}->{energy_after}'
        else:
            return f'Action will fail - insufficient energy (need {cost}, have {p1_energy})'
    
    def explain_action_reasoning(self, action, state):
        """Explain reasoning behind action prediction"""
        action_type = action.get('action_type', '')
        
        reasoning = []
        
        if 'pass' in action_type:
            phase = state.get('phase', '')
            if phase == 'Main':
                reasoning.append('Passing in Main phase loses tempo advantage')
                reasoning.append('May be necessary if no good plays available')
            else:
                reasoning.append('Passing advances phase normally')
        
        elif 'play_member_to_stage' in action_type:
            reasoning.append('Playing members increases stage presence')
            reasoning.append('Stage presence enables activation abilities')
            reasoning.append('More stage cards = more tempo advantage')
        
        elif 'use_ability' in action_type:
            reasoning.append('Abilities provide strategic advantages')
            reasoning.append('Effect depends on ability type and requirements')
            reasoning.append('May require specific game conditions')
        
        return reasoning
    
    def calculate_prediction_confidence(self, action, state):
        """Calculate confidence in prediction"""
        action_type = action.get('action_type', '')
        
        # High confidence for simple, deterministic actions
        if action_type in ['pass', 'rock_choice', 'paper_choice', 'scissors_choice']:
            return 0.95
        
        # Medium confidence for play actions (depends on energy)
        if 'play_member_to_stage' in action_type:
            return 0.85
        
        # Lower confidence for complex actions
        if 'use_ability' in action_type:
            return 0.6
        
        # Low confidence for unknown actions
        return 0.4
    
    def identify_action_risks_detailed(self, action, state):
        """Identify detailed risks of an action"""
        risks = []
        action_type = action.get('action_type', '')
        description = action.get('description', '')
        
        if 'play_member_to_stage' in action_type:
            # Extract cost
            import re
            cost_match = re.search(r'Cost: [^:]+: (\d+)', description)
            cost = int(cost_match.group(1)) if cost_match else 0
            
            p1_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            
            if cost > p1_energy:
                risks.append('Insufficient energy - action will fail')
            elif cost > p1_energy * 0.7:
                risks.append('High energy cost may limit future options')
            
            # Check stage space
            p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            if p1_stage >= 3:
                risks.append('Stage is full - may need baton touch')
        
        if 'pass' in action_type:
            phase = state.get('phase', '')
            if phase == 'Main':
                risks.append('Passing Main phase may lose tempo advantage')
                risks.append('Gives opponent free initiative')
        
        return risks
    
    def identify_action_opportunities_detailed(self, action, state):
        """Identify detailed opportunities from an action"""
        opportunities = []
        action_type = action.get('action_type', '')
        
        if 'play_member_to_stage' in action_type:
            opportunities.append('Increase stage presence for tempo advantage')
            opportunities.append('May enable activation abilities')
            opportunities.append('Adds strategic value to board')
            
            # Check if this enables abilities
            p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            if p1_stage == 0:
                opportunities.append('First stage card - enables many abilities')
            elif p1_stage == 1:
                opportunities.append('Second stage card - enables multi-card abilities')
        
        if 'set_live_card' in action_type:
            opportunities.append('Prepare for performance phase scoring')
            opportunities.append('May trigger performance-related abilities')
        
        if 'use_ability' in action_type:
            opportunities.append('Activate card effects for strategic advantage')
            opportunities.append('May swing game state in your favor')
        
        return opportunities
    
    def calculate_strategic_value(self, action, state):
        """Calculate strategic value of an action"""
        action_type = action.get('action_type', '')
        
        # Base strategic values
        base_values = {
            'pass': 1,
            'play_member_to_stage': 5,
            'use_ability': 4,
            'set_live_card': 3,
            'rock_choice': 1,
            'paper_choice': 1,
            'scissors_choice': 1,
            'choose_first_attacker': 2,
            'skip_mulligan': 2
        }
        
        base_value = base_values.get(action_type, 2)
        
        # Contextual modifiers
        if action_type == 'play_member_to_stage':
            # More valuable when behind on stage
            p1_stage = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            p2_stage = len([c for c in [state.get('player2', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            
            if p1_stage < p2_stage:
                base_value += 2  # More valuable when behind
            elif p1_stage == 0:
                base_value += 1  # Valuable to get first stage card
        
        elif action_type == 'pass':
            # Less valuable in critical phases
            phase = state.get('phase', '')
            if phase == 'Main':
                base_value -= 1  # Passing Main is often bad
        
        return max(base_value, 1)
    
    def suggest_alternatives(self, action, state):
        """Suggest alternative actions"""
        action_type = action.get('action_type', '')
        alternatives = []
        
        if action_type == 'pass':
            alternatives.append('Consider playing a member if possible')
            alternatives.append('Look for ability activation opportunities')
        
        elif action_type == 'play_member_to_stage':
            alternatives.append('Consider passing if cost is too high')
            alternatives.append('Look for cheaper members to play')
        
        return alternatives
    
    def execute_and_verify_action(self, action_index, action_type):
        """Execute action and verify prediction accuracy"""
        # Get state before action
        before_state, before_actions = get_state_and_actions()
        if not before_state:
            return None, None, False
        
        # Make prediction
        action = before_actions[action_index]
        prediction = self.predict_action_outcome_detailed(action, before_state)
        
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
        
        # Verify prediction
        verification = self.verify_prediction(prediction, before_state, after_state, success)
        
        return {
            'action': action,
            'prediction': prediction,
            'result': result,
            'success': success,
            'verification': verification,
            'before_state': before_state,
            'after_state': after_state
        }
    
    def verify_prediction(self, prediction, before_state, after_state, success):
        """Verify if prediction was accurate"""
        if not success:
            return {
                'accurate': False,
                'reason': 'Action failed to execute',
                'predicted_success': True,
                'actual_success': False
            }
        
        # Check phase changes
        before_phase = before_state.get('phase', '')
        after_phase = after_state.get('phase', '')
        
        # Check hand changes
        before_hand = len(before_state.get('player1', {}).get('hand', {}).get('cards', []))
        after_hand = len(after_state.get('player1', {}).get('hand', {}).get('cards', []))
        
        # Check stage changes
        before_stage = len([c for c in [before_state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        after_stage = len([c for c in [after_state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
        
        # Check energy changes
        before_energy = len([e for e in before_state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
        after_energy = len([e for e in after_state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
        
        # Verify prediction against actual changes
        accurate = True
        discrepancies = []
        
        if 'phase advances' in prediction and before_phase == after_phase:
            accurate = False
            discrepancies.append('Phase did not advance as predicted')
        
        if 'stage:' in prediction and before_stage == after_stage:
            accurate = False
            discrepancies.append('Stage did not change as predicted')
        
        if 'energy spent' in prediction and before_energy == after_energy:
            accurate = False
            discrepancies.append('Energy was not spent as predicted')
        
        return {
            'accurate': accurate,
            'discrepancies': discrepancies,
            'changes': {
                'phase': f'{before_phase} -> {after_phase}',
                'hand': f'{before_hand} -> {after_hand}',
                'stage': f'{before_stage} -> {after_stage}',
                'energy': f'{before_energy} -> {after_energy}'
            }
        }
    
    def find_and_verify_abilities(self):
        """Find and verify abilities with actual behavior"""
        state, actions = get_state_and_actions()
        if not state:
            return []
        
        ability_verifications = []
        
        # Look for ability actions
        for i, action in enumerate(actions):
            action_type = action.get('action_type', '').lower()
            description = action.get('description', '')
            
            if 'ability' in action_type or 'use_ability' in action_type or '{{kidou' in description or '{{jidou' in description:
                verification = self.verify_ability(action, i, state)
                ability_verifications.append(verification)
        
        return ability_verifications
    
    def verify_ability(self, action, action_index, state):
        """Verify ability against actual behavior"""
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
            # Execute and verify ability
            execution_result = self.execute_and_verify_action(action_index, action_type)
            ability_info['execution_result'] = execution_result
            ability_info['verification_status'] = 'tested'
        else:
            ability_info['execution_result'] = 'Requirements not met'
            ability_info['verification_status'] = 'blocked'
        
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
        elif 'discard' in desc_lower:
            return 'Discard cards'
        elif 'search' in desc_lower:
            return 'Search deck'
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
    
    def extract_cost(self, description):
        """Extract cost from description"""
        import re
        cost_patterns = [
            r'Cost: (\d+)',
            r'cost: (\d+)',
            r'(\d+) energy'
        ]
        
        for pattern in cost_patterns:
            match = re.search(pattern, description)
            if match:
                return int(match.group(1))
        return 0
    
    def check_ability_requirements(self, requirements, state):
        """Check if ability requirements are met"""
        # Check energy requirements
        if requirements['needs_energy'] > 0:
            active_energy = len([e for e in state.get('player1', {}).get('energy', {}).get('cards', []) if isinstance(e, dict) and e.get('orientation') == 'Active'])
            if active_energy < requirements['needs_energy']:
                return False
        
        # Check stage requirements
        if requirements['needs_stage']:
            stage_cards = len([c for c in [state.get('player1', {}).get('stage', {}).get(pos) for pos in ['left_side', 'center', 'right_side']] if c and isinstance(c, dict) and c.get('name')])
            if requirements['exclude_self']:
                # Need other cards besides self
                if stage_cards <= requirements['target_count']:
                    return False
            elif stage_cards == 0:
                return False
        
        return True
    
    def check_rules_compliance(self):
        """Check game behavior against rules.txt and qa_data.json"""
        compliance_issues = []
        
        # Check rules.txt
        try:
            with open('rules.txt', 'r', encoding='utf-8') as f:
                rules_content = f.read()
            
            # Analyze rules compliance
            rules_analysis = self.analyze_rules_compliance(rules_content)
            compliance_issues.extend(rules_analysis)
        except FileNotFoundError:
            compliance_issues.append({'type': 'missing_file', 'file': 'rules.txt', 'issue': 'Rules file not found'})
        
        # Check qa_data.json
        try:
            with open('qa_data.json', 'r', encoding='utf-8') as f:
                qa_data = json.load(f)
            
            # Analyze QA data compliance
            qa_analysis = self.analyze_qa_compliance(qa_data)
            compliance_issues.extend(qa_analysis)
        except FileNotFoundError:
            compliance_issues.append({'type': 'missing_file', 'file': 'qa_data.json', 'issue': 'QA data file not found'})
        
        return compliance_issues
    
    def analyze_rules_compliance(self, rules_content):
        """Analyze compliance with rules.txt"""
        issues = []
        
        # Check for common rule patterns
        if 'cost' in rules_content.lower():
            # Verify cost calculation is working correctly
            issues.append({'type': 'rule_check', 'rule': 'cost_calculation', 'status': 'needs_verification'})
        
        if 'phase' in rules_content.lower():
            # Verify phase transitions are correct
            issues.append({'type': 'rule_check', 'rule': 'phase_transitions', 'status': 'needs_verification'})
        
        if 'ability' in rules_content.lower():
            # Verify ability implementation
            issues.append({'type': 'rule_check', 'rule': 'ability_implementation', 'status': 'needs_verification'})
        
        return issues
    
    def analyze_qa_compliance(self, qa_data):
        """Analyze compliance with qa_data.json"""
        issues = []
        
        # Check QA test cases
        if isinstance(qa_data, dict):
            for test_name, test_case in qa_data.items():
                if isinstance(test_case, dict):
                    issues.append({'type': 'qa_test', 'test': test_name, 'status': 'needs_verification'})
        
        return issues
    
    def generate_comprehensive_documentation(self):
        """Generate comprehensive game documentation"""
        analysis, state, actions = self.analyze_game_state_deep()
        if not analysis:
            return "No game state available"
        
        doc = []
        doc.append("# COMPREHENSIVE GAME ANALYSIS DOCUMENTATION")
        doc.append(f"Generated: Turn {analysis['turn']}, Phase {analysis['phase']}")
        doc.append("")
        
        # Executive Summary
        doc.append("## EXECUTIVE SUMMARY")
        doc.append(f"**Current Position**: {analysis['strategic_position']['game_state']}")
        doc.append(f"**Overall Leader**: {analysis['strategic_position']['overall_leader']}")
        doc.append(f"**Recommended Focus**: {analysis['strategic_position']['recommended_focus']}")
        doc.append(f"**Winning Probability**: P1 {analysis['winning_analysis']['p1_win_probability']:.1%}, P2 {analysis['winning_analysis']['p2_win_probability']:.1%}")
        doc.append("")
        
        # Strategic Analysis
        doc.append("## STRATEGIC ANALYSIS")
        doc.append(f"### Tempo Analysis")
        doc.append(f"- **Tempo Leader**: {analysis['tempo_analysis']['tempo_leader']}")
        doc.append(f"- **Tempo Advantage**: P1 {analysis['tempo_analysis']['p1_stage_count']} vs P2 {analysis['tempo_analysis']['p2_stage_count']}")
        doc.append(f"- **Tempo Pressure**: {analysis['tempo_analysis']['tempo_pressure']}")
        doc.append(f"- **Tempo Control**: {analysis['tempo_analysis']['tempo_control']}")
        doc.append("")
        
        doc.append(f"### Resource Analysis")
        doc.append(f"- **Resource Balance**: {analysis['resource_analysis']['resource_balance']}")
        doc.append(f"- **Energy Advantage**: {analysis['resource_analysis']['energy_advantage']}")
        doc.append(f"- **Hand Advantage**: {analysis['resource_analysis']['hand_advantage']}")
        doc.append("")
        
        # Player Analysis
        doc.append("## PLAYER ANALYSIS")
        for player_key, player_data in analysis['players'].items():
            doc.append(f"### {player_data['name']}")
            doc.append(f"- **Hand**: {player_data['hand_count']} cards ({player_data['hand_quality']['analysis']})")
            doc.append(f"- **Stage**: {player_data['stage_count']} cards, {player_data['stage_power']} power")
            doc.append(f"- **Energy**: {player_data['energy_active']}/{player_data['energy_total']} ({player_data['energy_efficiency']:.1%} efficiency)")
            doc.append(f"- **Resource Position**: {player_data['resource_position']['position']} ({player_data['resource_position']['score']:.2f})")
            doc.append(f"- **Life**: {player_data['life_count']} cards")
            doc.append("")
        
        # Phase Analysis
        doc.append("## PHASE ANALYSIS")
        phase = analysis['phase_analysis']
        doc.append(f"- **Current Phase**: {phase['current_phase']} (Importance: {phase['importance']})")
        doc.append(f"- **Strategic Focus**: {phase['strategic_focus']}")
        doc.append(f"- **Optimal Play**: {phase['optimal_play']}")
        doc.append("")
        
        # Winning Analysis
        doc.append("## WINNING ANALYSIS")
        winning = analysis['winning_analysis']
        doc.append(f"- **Life Count**: P1 {winning['p1_life']}, P2 {winning['p2_life']}")
        doc.append(f"- **Critical Turn**: {winning['critical_turn']}")
        doc.append("")
        
        doc.append("### P1 Winning Paths")
        for path in winning['p1_winning_paths']:
            doc.append(f"- **{path['name']}**: {path['description']} (Probability: {path['probability']})")
        doc.append("")
        
        doc.append("### P2 Winning Paths")
        for path in winning['p2_winning_paths']:
            doc.append(f"- **{path['name']}**: {path['description']} (Probability: {path['probability']})")
        doc.append("")
        
        # Action Analysis
        doc.append("## ACTION ANALYSIS")
        predictions = self.predict_action_outcomes_deep(actions, state)
        
        # Sort by strategic value
        predictions.sort(key=lambda x: x['strategic_value'], reverse=True)
        
        for i, pred in enumerate(predictions[:10]):  # Top 10 actions
            action = pred['action']
            doc.append(f"### {i+1}. {action.get('action_type', 'Unknown')}")
            doc.append(f"**Description**: {action.get('description', 'No description')}")
            doc.append(f"**Strategic Value**: {pred['strategic_value']}/10")
            doc.append(f"**Predicted Outcome**: {pred['predicted_outcome']}")
            doc.append(f"**Confidence**: {pred['confidence']:.0%}")
            doc.append(f"**Reasoning**: {', '.join(pred['reasoning'])}")
            if pred['risks']:
                doc.append(f"**Risks**: {', '.join(pred['risks'])}")
            if pred['opportunities']:
                doc.append(f"**Opportunities**: {', '.join(pred['opportunities'])}")
            if pred['alternatives']:
                doc.append(f"**Alternatives**: {', '.join(pred['alternatives'])}")
            doc.append("")
        
        # Ability Analysis
        doc.append("## ABILITY ANALYSIS")
        ability_verifications = self.find_and_verify_abilities()
        
        if ability_verifications:
            doc.append(f"Found {len(ability_verifications)} abilities")
            for ability in ability_verifications:
                doc.append(f"### {ability['trigger_type']} Ability")
                doc.append(f"**Description**: {ability['description']}")
                doc.append(f"**Requirements Met**: {ability['requirements_met']}")
                doc.append(f"**Predicted Effect**: {ability['predicted_effect']}")
                doc.append(f"**Verification Status**: {ability['verification_status']}")
                if ability.get('execution_result'):
                    result = ability['execution_result']
                    if isinstance(result, dict):
                        doc.append(f"**Prediction Accurate**: {result['verification']['accurate']}")
                        if result['verification']['discrepancies']:
                            doc.append(f"**Discrepancies**: {', '.join(result['verification']['discrepancies'])}")
                    else:
                        doc.append(f"**Result**: {result}")
                doc.append("")
        else:
            doc.append("No abilities found or requirements not met")
            doc.append("")
        
        # Rules Compliance
        doc.append("## RULES COMPLIANCE")
        compliance_issues = self.check_rules_compliance()
        
        if compliance_issues:
            doc.append(f"Found {len(compliance_issues)} compliance issues:")
            for issue in compliance_issues:
                doc.append(f"- **{issue['type']}**: {issue.get('issue', 'Unknown issue')}")
        else:
            doc.append("No compliance issues found")
        doc.append("")
        
        # Problems and Fixes
        doc.append("## PROBLEMS IDENTIFIED AND FIXED")
        if self.problems_fixed:
            for problem in self.problems_fixed:
                doc.append(f"### {problem['type']}")
                doc.append(f"**Issue**: {problem['description']}")
                doc.append(f"**Fix**: {problem['fix']}")
                doc.append(f"**Status**: {problem['status']}")
                doc.append("")
        else:
            doc.append("No problems fixed yet")
            doc.append("")
        
        return "\n".join(doc)

def run_advanced_analysis():
    """Run advanced game analysis and documentation"""
    analyzer = AdvancedGameAnalyzer()
    
    print("=== ADVANCED GAME ANALYSIS SYSTEM ===")
    
    # Analyze current state
    analysis, state, actions = analyzer.analyze_game_state_deep()
    if not analysis:
        print("No game state available")
        return
    
    print(f"\n=== STRATEGIC OVERVIEW ===")
    print(f"Game State: {analysis['strategic_position']['game_state']}")
    print(f"Overall Leader: {analysis['strategic_position']['overall_leader']}")
    print(f"Recommended Focus: {analysis['strategic_position']['recommended_focus']}")
    print(f"P1 Win Probability: {analysis['winning_analysis']['p1_win_probability']:.1%}")
    print(f"P2 Win Probability: {analysis['winning_analysis']['p2_win_probability']:.1%}")
    
    print(f"\n=== TEMPO ANALYSIS ===")
    print(f"Tempo Leader: {analysis['tempo_analysis']['tempo_leader']}")
    print(f"Stage Count: P1 {analysis['tempo_analysis']['p1_stage_count']} vs P2 {analysis['tempo_analysis']['p2_stage_count']}")
    print(f"Tempo Pressure: {analysis['tempo_analysis']['tempo_pressure']}")
    
    print(f"\n=== RESOURCE ANALYSIS ===")
    print(f"Resource Balance: {analysis['resource_analysis']['resource_balance']}")
    print(f"Energy Advantage: {analysis['resource_analysis']['energy_advantage']}")
    print(f"Hand Advantage: {analysis['resource_analysis']['hand_advantage']}")
    
    # Test action predictions
    print(f"\n=== ACTION PREDICTION TESTING ===")
    predictions = analyzer.predict_action_outcomes_deep(actions, state)
    
    # Test top strategic actions
    top_actions = sorted(predictions, key=lambda x: x['strategic_value'], reverse=True)[:3]
    
    for i, pred in enumerate(top_actions):
        print(f"\n--- Top Action {i+1} ---")
        print(f"Action: {pred['action'].get('action_type', 'Unknown')}")
        print(f"Strategic Value: {pred['strategic_value']}/10")
        print(f"Predicted Outcome: {pred['predicted_outcome']}")
        print(f"Confidence: {pred['confidence']:.0%}")
        print(f"Risks: {', '.join(pred['risks']) if pred['risks'] else 'None'}")
        print(f"Opportunities: {', '.join(pred['opportunities']) if pred['opportunities'] else 'None'}")
    
    # Test abilities
    print(f"\n=== ABILITY VERIFICATION ===")
    ability_verifications = analyzer.find_and_verify_abilities()
    print(f"Found {len(ability_verifications)} abilities")
    
    for ability in ability_verifications:
        print(f"\n--- {ability['trigger_type']} Ability ---")
        print(f"Description: {ability['description']}")
        print(f"Requirements Met: {ability['requirements_met']}")
        print(f"Predicted Effect: {ability['predicted_effect']}")
        print(f"Verification Status: {ability['verification_status']}")
        
        if ability.get('execution_result') and isinstance(ability['execution_result'], dict):
            result = ability['execution_result']
            print(f"Prediction Accurate: {result['verification']['accurate']}")
            if result['verification']['discrepancies']:
                print(f"Discrepancies: {', '.join(result['verification']['discrepancies'])}")
    
    # Check rules compliance
    print(f"\n=== RULES COMPLIANCE CHECK ===")
    compliance_issues = analyzer.check_rules_compliance()
    print(f"Found {len(compliance_issues)} compliance issues")
    
    for issue in compliance_issues:
        print(f"- {issue['type']}: {issue.get('issue', 'Unknown issue')}")
    
    # Generate documentation
    print(f"\n=== GENERATING COMPREHENSIVE DOCUMENTATION ===")
    documentation = analyzer.generate_comprehensive_documentation()
    
    # Save documentation
    with open('advanced_game_analysis_documentation.md', 'w', encoding='utf-8') as f:
        f.write(documentation)
    
    print("Documentation saved to advanced_game_analysis_documentation.md")
    
    # Summary
    print(f"\n=== ANALYSIS SUMMARY ===")
    print(f"Game State: {analysis['strategic_position']['game_state']}")
    print(f"Total Actions: {len(actions)}")
    print(f"Strategic Actions Analyzed: {len(predictions)}")
    print(f"Abilities Found: {len(ability_verifications)}")
    print(f"Compliance Issues: {len(compliance_issues)}")
    print(f"Problems Fixed: {len(analyzer.problems_fixed)}")
    
    return analyzer, analysis

if __name__ == "__main__":
    run_advanced_analysis()

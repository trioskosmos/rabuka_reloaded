"""General ability parser based on MANUAL_GRAMMAR_ANALYSIS.md patterns."""
import re
import json
from typing import Dict, List, Optional, Any, Tuple
from pathlib import Path


class GeneralAbilityParser:
    """Parse card abilities into structured components based on grammatical patterns."""
    
    # Key patterns from MANUAL_GRAMMAR_ANALYSIS.md
    CONDITION_MARKERS = ['場合', 'とき', 'なら', 'かぎり']
    COST_MARKERS = ['支払う', 'コスト', '必要ハート']
    EFFECT_MARKERS = ['得る', 'する', 'なる', '置く', '加える', '引く', '登場させる', '移動させる']
    DURATION_MARKERS = ['ライブ終了時まで', 'かぎり', 'そのターンの間']
    TIMING_MARKERS = {
        '登場': 'on_entry',
        'ライブ開始時': 'live_start',
        'エールしたとき': 'on_cheer',
        '成功したとき': 'on_success',
        'ライブ終了時': 'live_end'
    }
    
    # Resource patterns
    RESOURCE_PATTERNS = {
        'heart': r'\{\{heart_\d+\.png\|[^}]+\}\}',
        'blade': r'\{\{icon_blade\.png\|[^}]+\}\}',
        'energy': r'\{\{icon_energy\.png\|[^}]+\}\}',
        'draw': r'\{\{icon_draw\.png\|[^}]+\}\}',
        'score': r'\{\{icon_score\.png\|[^}]+\}\}',
        'all': r'\{\{icon_all\.png\|[^}]+\}\}',
        'center': r'\{\{center\.png\|[^}]+\}\}',
        'live_start': r'\{\{live_start\.png\|[^}]+\}\}',
        'constant': r'\{\{jyouji\.png\|[^}]+\}\}'
    }
    
    # Location patterns
    LOCATIONS = {
        'ステージ': 'stage',
        '控え室': 'discard',
        '手札': 'hand',
        'デッキ': 'deck',
        'エネルギー置き場': 'energy_zone',
        '成功ライブカード置き場': 'success_live_zone',
        'ライブ中': 'during_live'
    }
    
    # Group patterns
    GROUP_PATTERN = r'『([^』]+)』'
    
    # Count patterns
    COUNT_PATTERN = r'(\d+)(?:枚|人|色|種類|つ)'
    
    # Comparison operators
    OPERATORS = {
        '以上': '>=',
        '以下': '<=',
        'より多い': '>',
        'より少ない': '<',
        '未満': '<',
        '超': '>'
    }
    
    def __init__(self):
        self.stats = {
            'total': 0,
            'parsed': 0,
            'unparsed': 0,
            'patterns_found': {}
        }
    
    def parse_ability(self, ability_text: str, card_info: Dict = None) -> Dict[str, Any]:
        """Parse a single ability text into structured components."""
        if not ability_text or ability_text.strip() == '':
            return {'raw': ability_text, 'parsed': False, 'error': 'Empty ability'}
        
        self.stats['total'] += 1
        
        result = {
            'raw': ability_text,
            'parsed': True,
            'timing': self._extract_timing(ability_text),
            'conditions': self._extract_conditions(ability_text),
            'costs': self._extract_costs(ability_text),
            'effects': self._extract_effects(ability_text),
            'targets': self._extract_targets(ability_text),
            'resources': self._extract_resources(ability_text),
            'locations': self._extract_locations(ability_text),
            'groups': self._extract_groups(ability_text),
            'patterns': self._detect_patterns(ability_text)
        }
        
        # Check if we actually found meaningful content
        if not any([result['timing'], result['conditions'], result['costs'], 
                   result['effects'], result['patterns']]):
            result['parsed'] = False
            result['error'] = 'No components extracted'
            self.stats['unparsed'] += 1
        else:
            self.stats['parsed'] += 1
            for pattern in result['patterns']:
                self.stats['patterns_found'][pattern] = self.stats['patterns_found'].get(pattern, 0) + 1
        
        return result
    
    def _extract_timing(self, text: str) -> List[str]:
        """Extract when the ability triggers."""
        timings = []
        for marker, timing in self.TIMING_MARKERS.items():
            if marker in text:
                timings.append(timing)
        return timings
    
    def _extract_conditions(self, text: str) -> List[Dict[str, Any]]:
        """Extract conditions that trigger the ability."""
        conditions = []
        
        # Split by condition markers
        parts = re.split(r'(場合|とき|なら|かぎり)(?:、|。)', text)
        
        for i in range(len(parts) - 1):
            if i + 1 < len(parts) and parts[i + 1] in ['場合', 'とき', 'なら', 'かぎり']:
                condition_text = parts[i]
                if condition_text.strip():
                    condition = {
                        'text': condition_text.strip(),
                        'marker': parts[i + 1],
                        'has_count': self._has_count(condition_text),
                        'has_location': self._has_location(condition_text),
                        'has_group': self._has_group(condition_text),
                        'comparison': self._extract_comparison(condition_text)
                    }
                    conditions.append(condition)
        
        return conditions
    
    def _extract_costs(self, text: str) -> List[Dict[str, Any]]:
        """Extract costs (energy, cards, etc.)."""
        costs = []
        
        # Handle activation abilities (colon-separated cost:effect)
        if '：' in text or ':' in text:
            # Split by colon to get cost part
            parts = re.split(r'[：:]', text)
            if len(parts) > 1:
                cost_text = parts[0]
                
                # Energy costs in cost part
                energy_matches = re.findall(r'(\{\{icon_energy\.png\|[^}]+\}\}+)', cost_text)
                for match in energy_matches:
                    count = match.count('{')
                    costs.append({
                        'type': 'energy',
                        'amount': count,
                        'optional': 'もよい' in cost_text or 'してもよい' in cost_text
                    })
                
                # Card costs in cost part
                if '控え室に置く' in cost_text:
                    costs.append({
                        'type': 'card_to_discard',
                        'text': cost_text
                    })
                
                # Member costs
                if 'ステージから控え室に置く' in cost_text:
                    costs.append({
                        'type': 'member_to_discard',
                        'text': cost_text
                    })
        else:
            # Energy costs (non-activation)
            energy_matches = re.findall(r'(\{\{icon_energy\.png\|[^}]+\}\}+)', text)
            for match in energy_matches:
                count = match.count('{')
                costs.append({
                    'type': 'energy',
                    'amount': count,
                    'optional': 'もよい' in text or 'してもよい' in text
                })
        
        # Card costs (e.g., "手札を1枚控え室に置く")
        card_cost_pattern = r'([^\s]+)を(\d+)枚控え室に置く'
        card_matches = re.findall(card_cost_pattern, text)
        for source, count in card_matches:
            costs.append({
                'type': 'card',
                'source': source,
                'amount': int(count),
                'destination': 'discard'
            })
        
        # Cost modifications
        if 'コストは' in text and 'になる' in text:
            costs.append({
                'type': 'cost_modification',
                'text': self._extract_between(text, 'コストは', 'になる')
            })
        
        return costs
    
    def _extract_effects(self, text: str) -> List[Dict[str, Any]]:
        """Extract effects (what the ability does)."""
        effects = []
        
        # Gain effects
        if 'を得る' in text:
            gain_parts = text.split('を得る')
            for part in gain_parts[:-1]:
                # Get the object being gained
                obj_match = re.search(r'([^\s]+)を得る', part + 'を得る')
                if obj_match:
                    effects.append({
                        'type': 'gain',
                        'object': obj_match.group(1),
                        'duration': self._extract_duration(text)
                    })
        
        # Score modification
        if 'スコア' in text and ('＋' in text or '−' in text or '＋１' in text or '＋２' in text):
            score_match = re.search(r'スコアを([＋−]\d+)', text)
            if score_match:
                effects.append({
                    'type': 'score_modification',
                    'value': score_match.group(1)
                })
        
        # Move cards
        if '登場させる' in text:
            effects.append({
                'type': 'summon',
                'duration': self._extract_duration(text)
            })
        
        if '移動させる' in text or '移動している' in text:
            effects.append({
                'type': 'move'
            })
        
        # Draw cards
        if 'カードを(\d+)枚引く' in text or 'カードを1枚引き' in text:
            draw_match = re.search(r'カードを(\d+)枚引く', text)
            if draw_match:
                effects.append({
                    'type': 'draw',
                    'amount': int(draw_match.group(1))
                })
            elif 'カードを1枚引き' in text:
                effects.append({'type': 'draw', 'amount': 1})
        
        # State changes
        if 'ウェイトにする' in text or 'ウェイトにし' in text:
            effects.append({'type': 'state_change', 'state': 'wait'})
        if 'アクティブにする' in text:
            effects.append({'type': 'state_change', 'state': 'active'})
        
        # Look at cards
        if '見る' in text:
            effects.append({'type': 'look'})
        
        return effects
    
    def _extract_targets(self, text: str) -> List[str]:
        """Extract targets (who/what is affected)."""
        targets = []
        
        if '自分の' in text:
            targets.append('self')
        if '相手の' in text:
            targets.append('opponent')
        if 'このカード' in text:
            targets.append('this_card')
        if 'このメンバー' in text:
            targets.append('this_member')
        if 'ステージにいる' in text:
            targets.append('stage_members')
        
        # Except modifier
        if '以外' in text:
            targets.append('except_modifier')
        
        return list(set(targets))
    
    def _extract_resources(self, text: str) -> Dict[str, int]:
        """Extract resource icons and their counts."""
        resources = {}
        
        for resource_name, pattern in self.RESOURCE_PATTERNS.items():
            matches = re.findall(pattern, text)
            if matches:
                resources[resource_name] = len(matches)
        
        return resources
    
    def _extract_locations(self, text: str) -> List[str]:
        """Extract location references."""
        locations = []
        
        for location_jp, location_en in self.LOCATIONS.items():
            if location_jp in text:
                locations.append(location_en)
        
        # Compound locations
        if 'と' in text and any(loc in text for loc in self.LOCATIONS.keys()):
            locations.append('compound')
        
        return list(set(locations))
    
    def _extract_groups(self, text: str) -> List[str]:
        """Extract group names (e.g., 『μ's』)."""
        return re.findall(self.GROUP_PATTERN, text)
    
    def _detect_patterns(self, text: str) -> List[str]:
        """Detect which grammatical patterns from MANUAL_GRAMMAR_ANALYSIS.md are present."""
        patterns = []
        
        # 1. Nested Conditionals
        if text.count('場合') > 1 or ('場合' in text and 'さらに' in text):
            patterns.append('nested_conditionals')
        
        # 2. Cost Modification
        if 'コストは' in text and ('になる' in text or 'にしてもよい' in text):
            patterns.append('cost_modification')
        
        # 3. Compound Locations
        if 'と' in text and any(loc in text for loc in ['ステージ', '控え室', '手札']):
            patterns.append('compound_locations')
        
        # 4. OR Conditionals
        if text.count('か') >= 2 and '場合' in text:
            patterns.append('or_conditionals')
        
        # 5. Per-Unit Modifiers
        if 'につき' in text:
            patterns.append('per_unit_modifiers')
        
        # 6. Complex Card Specifications
        if '必要ハートに含まれる' in text:
            patterns.append('complex_card_specifications')
        
        # 7. Sequential Actions
        if 'その後' in text or 'し、' in text:
            patterns.append('sequential_actions')
        
        # 8. Relative Locations
        if 'このメンバーがいたエリア' in text or 'そのメンバーの下' in text:
            patterns.append('relative_locations')
        
        # 9. Optional Action with Conditional Follow-up
        if 'してもよい' in text and 'そうした場合' in text:
            patterns.append('optional_conditional_followup')
        
        # 10. Alternative Effects
        if '代わりに' in text:
            patterns.append('alternative_effects')
        
        # 11. Choice Effects
        if '以下から1つを選ぶ' in text or ('か' in text and 'のうち' in text and '選ぶ' in text):
            patterns.append('choice_effects')
        
        # 12. "その中から" Pattern
        if 'その中から' in text:
            patterns.append('from_among_pattern')
        
        # 13. Universal Quantifiers
        if 'すべてある' in text or 'それぞれ' in text:
            patterns.append('universal_quantifiers')
        
        # 14. Duration Markers
        if 'かぎり' in text:
            patterns.append('duration_markers')
        
        # 15. Except Modifiers
        if '以外' in text:
            patterns.append('except_modifiers')
        
        # 16. Cost Calculation
        if 'コストに' in text and '足した' in text:
            patterns.append('cost_calculation')
        
        # 17. Ability Gain
        if '「' in text and '」を得る' in text:
            patterns.append('ability_gain')
        
        # 18. State Changes
        if 'になる' in text or '減らす' in text:
            patterns.append('state_changes')
        
        return patterns
    
    def _extract_duration(self, text: str) -> Optional[str]:
        """Extract duration of effect."""
        for marker in self.DURATION_MARKERS:
            if marker in text:
                return marker
        return None
    
    def _extract_between(self, text: str, start: str, end: str) -> str:
        """Extract text between two markers."""
        pattern = f'{re.escape(start)}(.*?){re.escape(end)}'
        match = re.search(pattern, text)
        return match.group(1) if match else ''
    
    def _has_count(self, text: str) -> bool:
        """Check if text contains a count."""
        return bool(re.search(self.COUNT_PATTERN, text))
    
    def _has_location(self, text: str) -> bool:
        """Check if text contains a location."""
        return any(loc in text for loc in self.LOCATIONS.keys())
    
    def _has_group(self, text: str) -> bool:
        """Check if text contains a group."""
        return bool(re.search(self.GROUP_PATTERN, text))
    
    def _extract_comparison(self, text: str) -> Optional[str]:
        """Extract comparison operator."""
        for op_jp, op_en in self.OPERATORS.items():
            if op_jp in text:
                return op_en
        return None
    
    def parse_cards_file(self, cards_file: str, output_file: str = None):
        """Parse all cards from a JSON file."""
        cards_path = Path(cards_file)
        with open(cards_path, 'r', encoding='utf-8') as f:
            cards = json.load(f)
        
        results = {}
        for card_id, card_data in cards.items():
            if 'ability' in card_data and card_data['ability']:
                parsed = self.parse_ability(card_data['ability'], card_data)
                results[card_id] = {
                    'card_name': card_data.get('name', ''),
                    'parsed_ability': parsed
                }
        
        if output_file:
            output_path = Path(output_file)
            with open(output_path, 'w', encoding='utf-8') as f:
                json.dump({
                    'results': results,
                    'stats': self.stats
                }, f, ensure_ascii=False, indent=2)
        
        return results, self.stats
    
    def print_stats(self):
        """Print parsing statistics."""
        print(f"\n=== Parsing Statistics ===")
        print(f"Total abilities: {self.stats['total']}")
        print(f"Parsed: {self.stats['parsed']}")
        print(f"Unparsed: {self.stats['unparsed']}")
        print(f"Success rate: {self.stats['parsed']/self.stats['total']*100:.1f}%")
        print(f"\nPatterns found:")
        for pattern, count in sorted(self.stats['patterns_found'].items(), key=lambda x: x[1], reverse=True):
            print(f"  {pattern}: {count}")


if __name__ == '__main__':
    parser = GeneralAbilityParser()
    
    # Parse all cards
    cards_file = '../cards.json'
    output_file = 'parsed_abilities_general.json'
    
    print(f"Parsing abilities from {cards_file}...")
    results, stats = parser.parse_cards_file(cards_file, output_file)
    
    parser.print_stats()
    
    print(f"\nResults saved to {output_file}")

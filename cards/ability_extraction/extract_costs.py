"""Process abilities: parse costs/effects and standardize JSON structure."""

import json
import re
from dataclasses import dataclass
from typing import Any, Callable
from condition_parser import parse_condition
from effect_parser import (
    _extract_optional_payment,
    parse_generic_effect,
)
from parser_utils import (
    extract_count,
    extract_group_name,
    extract_int,
    extract_quoted_name,
    has_any,
    normalize_text,
    parse_optional_flag,
)

def walk_nodes(value, visit):
    """Depth-first walk over dict/list nodes and call visit for each dict."""
    if isinstance(value, dict):
        visit(value)
        for item in value.values():
            walk_nodes(item, visit)
    elif isinstance(value, list):
        for item in value:
            walk_nodes(item, visit)


# Cost parsing
@dataclass(frozen=True)
class CostRule:
    name: str
    output_key: str
    priority: int
    matches: Callable[[str], bool]
    extract: Callable[[str], Any]
    notes: str = ""

ENERGY_ICON = '{{icon_energy.png|E}}'
MEMBER_COUNT_PATTERN = re.compile(r'メンバー(\d+)人')
MAX_PEOPLE_PATTERN = re.compile(r'(\d+)人まで')


def _extract_cost_text(text):
    if '：' not in text and ':' not in text:
        return None
    delimiter = '：' if '：' in text else ':'
    cost_text = text.split(delimiter, 1)[0]
    # Don't strip period here - normalize_text will do it later
    # But we need to handle the case where cost text might have a period at the end
    cost_text = cost_text.strip()
    return cost_text or None


def _extract_card_type(text, *, default=None):
    if 'メンバーカード' in text:
        return 'member_card'
    if 'ライブカード' in text:
        return 'live_card'
    return default


def _extract_position(text):
    if 'センター' in text:
        return 'center'
    if '左サイド' in text:
        return 'left_side'
    if '右サイド' in text:
        return 'right_side'
    return None


def _extract_member_count(text, default=1):
    return extract_int(MEMBER_COUNT_PATTERN, text, default=default)


def _extract_max_people_count(text):
    return extract_int(MAX_PEOPLE_PATTERN, text)


def _clean_extracted_cost(value):
    if not isinstance(value, dict):
        return value
    cleaned = {key: item for key, item in value.items() if item is not None}
    if cleaned.get('max') is False:
        del cleaned['max']
    return cleaned


def _annotate_text(value, text):
    if not text or value is None:
        return value
    if isinstance(value, dict):
        value.setdefault('text', text)
        for item in value.values():
            _annotate_text(item, text)
    elif isinstance(value, list):
        for item in value:
            _annotate_text(item, text)
    return value


def _build_move_cost(text, *, source, destination, cost_type='move_cards', **kwargs):
    """Generic cost builder for card movement costs."""
    result = {
        'type': cost_type,
        'source': source,
        'destination': destination,
        'count': extract_count(text) or kwargs.get('default_count', 1),
    }
    # Add optional fields based on context
    if 'このメンバー' in text:
        result['target'] = "this_member"
        if 'このメンバー以外' in text:
            result['exclude_member'] = True
        elif '以外' in text:
            result['exclude_member'] = extract_quoted_name(text)
        if 'ウェイト' in text:
            result['destination'] = 'wait'
        max_count = _extract_max_people_count(text) if '人まで' in text else None
        if max_count:
            result['count'] = _extract_member_count(text, default=max_count)
            result['max'] = True
        else:
            result['count'] = _extract_member_count(text) if 'メンバー' in text else result['count']
    if 'hand' in source or '公開' in text:
        result['optional'] = parse_optional_flag(text, ['置いてもよい', 'でもよい', '公開してもよい', '支払ってもよい'])
    if 'カード' in text or 'card_type' in kwargs:
        result['card_type'] = _extract_card_type(text) or kwargs.get('card_type') or kwargs.get('default_card_type', 'card')
    if 'エネルギー' in text or 'energy' in source or 'energy' in destination:
        result['card_type'] = 'energy_card'
    if '『' in text:
        result['group'] = extract_group_name(text)
    if '好きな順番' in text:
        result['order'] = 'any'
    if 'すべて' in text:
        result['count'] = 'all'
    # Override with any explicit kwargs, but don't override target if already set
    for k, v in kwargs.items():
        if v is not None and k not in ('default_count', 'default_card_type', 'card_type'):
            if k == 'target' and 'target' in result:
                continue  # Don't override target if already set by logic
            result[k] = v
    return result


COST_RULES = (
    CostRule(
        name='energy',
        output_key='energy',
        priority=10,
        matches=lambda text: ENERGY_ICON in text,
        extract=lambda text: {'type': 'pay_energy', 'energy': text.count(ENERGY_ICON)},
        notes='Counts explicit energy icons before any text-based cost parsing.',
    ),
    CostRule(
        name='this_member_to_discard',
        output_key='member_to_discard',
        priority=25,
        matches=lambda text: has_any(text, ['このメンバーをステージから控え室に置く']),
        extract=lambda text: _build_move_cost(text, source='stage', destination='discard', target='this_member'),
        notes='Self-discard cost with this_member target.',
    ),
    CostRule(
        name='member_to_discard',
        output_key='member_to_discard',
        priority=20,
        matches=lambda text: has_any(text, ['ステージから控え室に置']),
        extract=lambda text: _build_move_cost(text, source='stage', destination='discard'),
        notes='Specific stage-to-discard movement should win over generic discard rules.',
    ),
    CostRule(
        name='member_to_wait',
        output_key='member_to_wait',
        priority=30,
        matches=lambda text: has_any(text, ['ウェイトにする', 'ウェイトにしてもよい']),
        extract=lambda text: _build_move_cost(text, source='stage', destination='wait'),
    ),
    CostRule(
        name='reveal',
        output_key='reveal',
        priority=40,
        matches=lambda text: '手札' in text and has_any(text, ['公開する', '公開してもよい', '公開し']),
        extract=lambda text: _build_move_cost(text, source='hand', destination=None, cost_type='reveal_cards', default_count='any'),
    ),
    CostRule(
        name='energy_to_member',
        output_key='energy_to_member',
        priority=50,
        matches=lambda text: 'エネルギー置き場' in text and 'このメンバーの下に置く' in text,
        extract=lambda text: _build_move_cost(text, source='energy_zone', destination='member_under', target='this_member', card_type='energy_card'),
    ),
    CostRule(
        name='energy_to_energy_deck',
        output_key='energy_to_energy_deck',
        priority=60,
        matches=lambda text: 'エネルギーデッキ' in text and '置く' in text,
        extract=lambda text: _build_move_cost(text, source='energy_zone', destination='energy_deck', card_type='energy_card'),
    ),
    CostRule(
        name='discard_to_deck_bottom',
        output_key='discard_to_deck_bottom',
        priority=70,
        matches=lambda text: '控え室' in text and has_any(text, ['デッキの一番下に置く', 'デッキの一番下に置いてもよい']),
        extract=lambda text: _build_move_cost(text, source='discard', destination='deck_bottom', default_card_type='card'),
    ),
    CostRule(
        name='hand_to_deck_bottom',
        output_key='hand_to_deck_bottom',
        priority=80,
        matches=lambda text: '手札' in text and has_any(text, ['デッキの一番下に置く', 'デッキの一番下に置いてもよい']),
        extract=lambda text: _build_move_cost(text, source='hand', destination='deck_bottom', default_card_type='card'),
    ),
    CostRule(
        name='discard_from_hand',
        output_key='discard_from_hand',
        priority=90,
        matches=lambda text: '手札' in text and has_any(text, ['控え室に置く', '控え室に置いてもよい']),
        extract=lambda text: _build_move_cost(text, source='hand', destination='discard'),
    ),
    CostRule(
        name='discard_from_deck',
        output_key='discard_from_deck',
        priority=100,
        matches=lambda text: 'デッキ' in text and '控え室に置く' in text,
        extract=lambda text: _build_move_cost(text, source='deck', destination='discard'),
    ),
)


def parse_cost(text):
    """Parse cost from triggerless text using ordered cost rules."""
    cost_text = _extract_cost_text(text)
    if not cost_text:
        # If no delimiter found, assume text is already split cost text
        cost_text = text
        if not cost_text:
            return None

    # Normalize the cost text for matching (don't strip trigger icons here)
    normalized_cost_text = normalize_text(cost_text)
    
    cost = {}
    for rule in sorted(COST_RULES, key=lambda item: item.priority):
        if rule.matches(normalized_cost_text):
            extracted = _clean_extracted_cost(rule.extract(normalized_cost_text))
            cost.update(extracted)

    position = _extract_position(cost_text)
    if position:
        cost['position'] = position

    if not cost:
        return _annotate_text(cost_text, cost_text)
    cost['text'] = cost_text
    return _annotate_text(cost, cost_text)


def extract_location_from_text(text):
    """Extract location from full ability text."""
    if not text:
        return None
    if 'ステージ' in text:
        return 'stage'
    if '控え室' in text:
        return 'discard'
    if '手札' in text:
        return 'hand'
    if 'エネルギー置き場' in text:
        return 'energy_zone'
    if 'エネルギーデッキ' in text:
        return 'energy_deck'
    if 'ライブカード置き場' in text:
        return 'live_card_zone'
    if '成功ライブカード置き場' in text:
        return 'success_live_card_zone'
    if 'デッキ' in text:
        return 'deck'
    return None


def extract_energy_from_text(text):
    """Extract energy count from full ability text."""
    if not text:
        return None
    if '{{icon_energy.png|E}}' in text or 'エネルギー' in text:
        count = text.count('{{icon_energy.png|E}}')
        if count == 0:
            # Try to extract from text patterns like "エネルギーを2枚"
            match = re.search(r'エネルギー.*?(\d+)枚', text)
            if match:
                return int(match.group(1))
            return 1  # Default to 1 if energy is mentioned but no count
        return count
    return None


def extract_group_from_text(text):
    """Extract group name from full ability text."""
    if not text:
        return None
    match = re.search(r'『(.+?)』', text)
    if match:
        return match.group(1)
    return None


def extract_heart_from_text(text):
    """Extract heart types/count from full ability text."""
    if not text:
        return None
    if 'ハート' in text or 'heart' in text.lower():
        heart_types = re.findall(r'heart_(\d+)', text, re.IGNORECASE)
        if heart_types:
            return heart_types
        # If heart is mentioned but no specific types, return a generic indicator
        return True
    return None


def extract_card_type_from_text(text):
    """Extract card type from full ability text."""
    if not text:
        return None
    if 'メンバーカード' in text:
        return 'member_card'
    if 'ライブカード' in text:
        return 'live_card'
    if 'エネルギーカード' in text:
        return 'energy_card'
    if 'ブレードを持つカード' in text:
        return 'blade_card'
    if 'メンバー' in text:
        return 'member_card'
    if 'カード' in text:
        return 'card'
    return None


def extract_blade_from_text(text):
    """Extract blade count from full ability text."""
    if not text:
        return None
    if 'ブレード' in text or 'blade' in text.lower():
        count = text.count('{{icon_blade.png|ブレード}}')
        if count > 0:
            return count
        # Try to extract from text patterns
        match = re.search(r'ブレード.*?(\d+)つ', text)
        if match:
            return int(match.group(1))
        # If blade is mentioned but no count, return a generic indicator
        return True
    return None

def apply_tree_field(data, field_name, value_factory, *, tree_key='effect'):
    """Copy a computed field onto every dict node in each parsed tree."""
    for item in data.get('unique_abilities', []):
        tree = item.get(tree_key)
        if not tree:
            continue
        value = value_factory(item)
        if value is None:
            continue
        def visit(node):
            if field_name not in node:
                node[field_name] = value
        walk_nodes(tree, visit)
    return data

def prune_empty_raw_nodes(value):
    """Remove nodes that only carry an empty raw_text placeholder."""
    if isinstance(value, dict):
        keys_to_delete = []
        for key, item in value.items():
            if isinstance(item, dict) and item.get('raw_text') == '':
                keys_to_delete.append(key)
            else:
                prune_empty_raw_nodes(item)
        for key in keys_to_delete:
            del value[key]
    elif isinstance(value, list):
        items = []
        for item in value:
            if isinstance(item, dict) and item.get('raw_text') == '':
                continue
            prune_empty_raw_nodes(item)
            items.append(item)
        value[:] = items
    return value

def process_abilities(data):
    """Populate structured cost/effect fields for each unique ability.
    
    Processing flow:
    1. Parse cost from triggerless text
    2. Strip trigger icons to get effect text
    3. Extract optional payment from effect text
    4. Split on cost delimiter (if present) to get costless effect text
    5. Strip trailing parenthetical notes from costless text
    6. Parse effect from cleaned costless text
    """
    for ability in data['unique_abilities']:
        # Step 1: Parse cost from triggerless text (triggers already removed)
        triggerless_text = ability['triggerless_text']
        ability['cost'] = parse_cost(triggerless_text)
        
        # Determine if this ability has no cost
        ability['costless'] = ability['cost'] is None or (
            isinstance(ability['cost'], str) and ability['cost'] == triggerless_text
        )
        
        # Step 2: Strip trigger icons to get raw effect text
        effect_text_raw = triggerless_text
        effect_text_raw = re.sub(r'^(?:\{\{[^}]+\}\}\s*)+', '', effect_text_raw)
        ability['use_limitless_text'] = effect_text_raw
        
        # Step 3: Extract optional payment from effect text
        payment, text_after_payment = _extract_optional_payment(effect_text_raw)
        if payment:
            ability['payment'] = payment
            effect_text = text_after_payment
        else:
            effect_text = effect_text_raw
        
        # Step 4: Split on cost delimiter to get costless effect text
        if '??' in effect_text:
            effect_text = effect_text.split('??', 1)[1].strip()
        elif ':' in effect_text or '：' in effect_text:
            delimiter = '：' if '：' in effect_text else ':'
            effect_text = effect_text.split(delimiter, 1)[1].strip()
        
        ability['costless_text'] = effect_text
        
        # Step 5: Strip trailing parenthetical notes before effect parsing
        effect_text_clean = _strip_trailing_parenthetical_note(effect_text)
        
        # Step 6: Parse effect from cleaned text
        ability['effect'] = parse_generic_effect(effect_text_clean) if effect_text_clean else None
    
    return data


def _strip_trailing_parenthetical_note(text):
    """Strip trailing parenthetical notes like （ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を増やさない。）"""
    if not text:
        return text
    text = text.strip()
    # Check for trailing parenthetical note
    if text.endswith(('。）', '.)', ')')):
        # Find the last opening parenthesis
        last_open_paren = max(text.rfind('（'), text.rfind('('))
        if last_open_paren > 0:
            # Check if it's a trailing parenthetical note (space before it, or starts with common note patterns)
            paren_content = text[last_open_paren:]
            if paren_content.startswith(('（この能力は', '（この効果は', '（ウェイト状態のメンバーが持つ')) or text[last_open_paren - 1] in '。 ':
                return text[:last_open_paren].strip()
    return text

def post_process(data):
    """Standardize JSON architecture and fix common issues."""
    def visit(node):
        # Fix condition types
        condition = node.get('condition')
        if isinstance(condition, dict) and condition.get('exclude_this_member') and 'type' not in condition:
            condition['type'] = 'member_exclusion'
        # Fix missing condition types
        if isinstance(condition, dict) and 'type' not in condition:
            text = condition.get('text', '')
            if 'ちょうど' in text or 'exactly' in text:
                condition['type'] = 'member_count_exact'
            elif '以上' in text or '以上の' in text:
                condition['type'] = 'member_count_at_least'
            elif '以下' in text or '以下の' in text:
                condition['type'] = 'member_count_at_most'
            elif 'いる' in text or 'present' in text:
                condition['type'] = 'card_presence'
            elif text:
                condition['type'] = 'custom'
        # Flatten nested actions
        nested_action = node.get('action')
        if isinstance(nested_action, dict):
            if 'actions' in nested_action:
                del node['action']
                for key, item in nested_action.items():
                    if key == 'actions' or key not in node:
                        node[key] = item
            elif 'action' in nested_action:
                del node['action']
                for key, item in nested_action.items():
                    if key not in node:
                        node[key] = item
    walk_nodes(data, visit)
    
    # Ability-level fixes
    for ability in data.get('unique_abilities', []):
        full_text = ability.get('full_text', '') or ability.get('triggerless_text', '')
        effect = ability.get('effect')
        # Fix cost int type mismatch by moving to cost_limit (ability level)
        if isinstance(ability.get('cost'), int):
            ability['cost_limit'] = ability['cost']
            ability['cost'] = None
        # Extract location from full text if not already present
        if full_text and not ability.get('location'):
            location_text = ability.get('costless_text') or full_text
            location = extract_location_from_text(location_text)
            if location:
                ability['location'] = location
        # Extract energy from full text if not already present
        if full_text and not ability.get('energy_count') and not ability.get('resource'):
            energy = extract_energy_from_text(full_text)
            if energy:
                ability['energy_count'] = energy
                ability['resource'] = 'energy'
        # Extract group from full text if not already present
        if full_text and not ability.get('group') and not ability.get('value'):
            group = extract_group_from_text(full_text)
            if group:
                ability['group'] = group
                ability['value'] = group
        # Extract heart from full text if not already present
        if full_text and not ability.get('heart_types') and not ability.get('heart_count'):
            heart = extract_heart_from_text(full_text)
            if heart:
                if isinstance(heart, list):
                    ability['heart_types'] = heart
                elif isinstance(heart, bool):
                    ability['heart_present'] = heart
        # Extract card_type from full text if not already present
        if full_text and not ability.get('card_type'):
            card_type = extract_card_type_from_text(full_text)
            if card_type:
                ability['card_type'] = card_type
        # Extract blade from full text if not already present
        if full_text and not ability.get('blade_count'):
            blade = extract_blade_from_text(full_text)
            if blade:
                if isinstance(blade, int):
                    ability['blade_count'] = blade
                elif isinstance(blade, bool):
                    ability['blade_present'] = blade
        if isinstance(effect, dict) and ability.get('location') == 'discard':
            effect_source = effect.get('source')
            if effect_source == 'energy_deck':
                ability['location'] = 'energy_deck'
            elif isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and action.get('source') == 'energy_deck':
                        ability['location'] = 'energy_deck'
                        break

        # Recover energy-deck source information that can be lost when the
        # optional payment prefix is stripped before effect parsing.
        if full_text and 'エネルギーデッキから' in full_text:
            effect = ability.get('effect')
            if isinstance(effect, dict):
                def _mark_energy_deck_source(node):
                    if not isinstance(node, dict):
                        return
                    if (
                        node.get('action') == 'place_card'
                        and node.get('card_type') == 'energy_card'
                        and node.get('state') == 'wait'
                        and 'source' not in node
                    ):
                        node['source'] = 'energy_deck'

                walk_nodes(effect, _mark_energy_deck_source)
    
    # Fix missing source/destination fields in actions (run after other fixes)
    def fix_action_fields(node):
        """Infer missing source/destination fields based on action type."""
        if not isinstance(node, dict):
            return
        
        # Fix cost int type mismatch in conditions
        if isinstance(node.get('cost'), int):
            if node.get('cost_limit'):
                del node['cost']  # Delete duplicate cost field if cost_limit exists
            else:
                node['cost_limit'] = node['cost']
                node['cost'] = None
        
        action = node.get('action')
        if not action or not isinstance(action, str):
            return
        
        # Infer missing source/destination for move_cards
        if action == 'move_cards':
            text = node.get('text', '')
            if 'destination' not in node:
                # Infer destination from context
                if '手札に加える' in text or '引く' in text:
                    node['destination'] = 'hand'
                elif 'ウェイトにする' in text or '控え室に置く' in text:
                    node['destination'] = 'discard'
                elif '登場させる' in text or 'ステージに置く' in text:
                    node['destination'] = 'stage'
                elif 'デッキの一番上に置く' in text or 'デッキの上に置く' in text:
                    node['destination'] = 'deck_top'
                elif 'デッキの一番下に置く' in text or 'デッキの下に置く' in text:
                    node['destination'] = 'deck_bottom'
            if 'source' not in node:
                # Infer source from context
                if '引く' in text:
                    node['source'] = 'deck'
                elif '自分の控え室から' in text or '控え室から' in text:
                    node['source'] = 'discard'
                elif '自分のエネルギーデッキから' in text:
                    node['source'] = 'energy_deck'
                elif '手札を' in text and '控え室に置く' in text:
                    node['source'] = 'hand'
                elif 'ステージから' in text or 'ウェイトにする' in text:
                    node['source'] = 'stage'
        elif action == 'select_member' and 'target' not in node:
            node['target'] = 'self'
        
        # Recursively fix nested actions
        if 'actions' in node:
            for child in node['actions']:
                fix_action_fields(child)
        if 'action' in node and isinstance(node['action'], dict):
            fix_action_fields(node['action'])
        # Also fix condition nodes
        if 'condition' in node and isinstance(node['condition'], dict):
            fix_action_fields(node['condition'])
    
    for ability in data.get('unique_abilities', []):
        # Convert single action to array
        if isinstance(ability.get('effect'), dict):
            effect = ability['effect']
            if ability.get('location') == 'waitroom':
                effect_source = effect.get('source')
                if effect_source == 'energy_deck':
                    ability['location'] = 'energy_deck'
                elif isinstance(effect.get('actions'), list):
                    for action in effect['actions']:
                        if isinstance(action, dict) and action.get('source') == 'energy_deck':
                            ability['location'] = 'energy_deck'
                            break
            if 'action' in effect:
                if isinstance(effect['action'], dict):
                    effect['actions'] = [effect['action']]
                elif isinstance(effect['action'], str):
                    # Use the effect's text field if available, otherwise use action name
                    action_text = effect.get('text', effect['action'])
                    effect['actions'] = [{'action': effect['action'], 'text': action_text}]
                    # Copy other relevant fields from effect to action
                    for field in ['source', 'destination', 'count', 'card_type', 'trigger']:
                        if field in effect:
                            effect['actions'][0][field] = effect[field]
                del effect['action']
            # Remove redundant payment from effect
            if isinstance(effect.get('payment'), dict) and ability.get('cost'):
                del effect['payment']
            # Remove redundant payment from ability level
            if ability.get('payment') and ability.get('cost'):
                del ability['payment']
            
            # Fix truncated text in member_to_wait actions
            # If effect has cost_limit but action text is missing "コストX以下", add it back
            # Also fix action type from discard_to_waitroom to member_to_wait when appropriate
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    # Check if action should be member_to_wait but is mislabeled as discard_to_waitroom
                    if action.get('action') == 'discard_to_waitroom' and 'ウェイトにする' in action.get('text', ''):
                        action['action'] = 'member_to_wait'
                        action['source'] = action.get('source', 'stage')
                    
                    # Check if action should be deploy_to_stage but is mislabeled as discard_to_waitroom
                    if action.get('action') == 'discard_to_waitroom' and '登場させる' in action.get('text', ''):
                        action['action'] = 'deploy_to_stage'
                        action['destination'] = 'stage'
                        if '控え室から' in action.get('text', ''):
                            action['source'] = 'waitroom'
                    
                    # Fix truncated text for both member_to_wait and discard_to_waitroom actions
                    if action.get('action') in ['member_to_wait', 'discard_to_waitroom']:
                        if effect.get('cost_limit') and 'コスト' not in action.get('text', ''):
                            # Reconstruct the full text with cost_limit
                            original_text = action.get('text', '')
                            # Find where "のメンバー" appears and insert cost_limit before it
                            if 'のメンバー' in original_text:
                                action['text'] = original_text.replace('のメンバー', f'コスト{effect["cost_limit"]}以下のメンバー', 1)
            
            # Fix unknown actions that match "choose heart color then gain heart" pattern
            if isinstance(effect.get('actions'), list):
                for i, action in enumerate(effect['actions']):
                    if action.get('action') == 'unknown' and 'parsing_error' in action:
                        text = action.get('text', '')
                        # Strip condition prefix if present (text after "場合、" or similar)
                        for marker in ['場合、', '場合', 'とき、', 'とき']:
                            if marker in text:
                                text = text.split(marker, 1)[1].strip()
                        # Check if this is the "choose heart color then gain heart" pattern
                        if '好きなハートの色を1つ指定する' in text and 'そのハートを1つ得る' in text:
                            # Replace the unknown action with two actions
                            effect['actions'][i] = {
                                'action': 'choose_heart_color',
                                'choice': True,
                                'count': 1,
                                'text': '好きなハートの色を1つ指定する'
                            }
                            effect['actions'].insert(i + 1, {
                                'action': 'gain_resource',
                                'resource': 'heart',
                                'resource_count': 1,
                                'count': 1,
                                'source': 'chosen_heart',
                                'text': 'そのハートを1つ得る'
                            })
                            # Copy duration from effect to the gain_resource action
                            if effect.get('duration'):
                                effect['actions'][i + 1]['duration'] = effect['duration']
            # Fix cost text appearing in action text
            if ability.get('cost') and isinstance(ability.get('cost'), dict) and ability.get('cost', {}).get('text'):
                cost_text = ability['cost']['text']
                # Remove cost text from action texts
                if isinstance(effect.get('actions'), list):
                    for action in effect['actions']:
                        if isinstance(action, dict) and 'text' in action:
                            action_text = action['text']
                            if action_text.startswith(cost_text + '：'):
                                action['text'] = action_text[len(cost_text) + 1:].strip()
                            elif action_text.startswith(cost_text):
                                action['text'] = action_text[len(cost_text):].strip()
                # Also fix effect text
                if 'text' in effect and effect['text'].startswith(cost_text + '：'):
                    effect['text'] = effect['text'][len(cost_text) + 1:].strip()
                elif 'text' in effect and effect['text'].startswith(cost_text):
                    effect['text'] = effect['text'][len(cost_text):].strip()
                # Also fix with energy icons stripped (in case they were removed from action text)
                cost_text_no_icons = cost_text
                for icon in ['{{icon_energy.png|E}}', '{{icon_energy.png|e}}']:
                    cost_text_no_icons = cost_text_no_icons.replace(icon, '')
                if cost_text_no_icons != cost_text:
                    if isinstance(effect.get('actions'), list):
                        for action in effect['actions']:
                            if isinstance(action, dict) and 'text' in action:
                                action_text = action['text']
                                if action_text.startswith(cost_text_no_icons + '：'):
                                    action['text'] = action_text[len(cost_text_no_icons) + 1:].strip()
                                elif action_text.startswith(cost_text_no_icons):
                                    action['text'] = action_text[len(cost_text_no_icons):].strip()
                    if 'text' in effect and effect['text'].startswith(cost_text_no_icons + '：'):
                        effect['text'] = effect['text'][len(cost_text_no_icons) + 1:].strip()
                    elif 'text' in effect and effect['text'].startswith(cost_text_no_icons):
                        effect['text'] = effect['text'][len(cost_text_no_icons):].strip()
            # Fix cost text appearing in action text (when cost is in effect.actions instead of ability.cost)
            if effect.get('actions') and ability.get('cost') and isinstance(ability.get('cost'), dict):
                for action in effect['actions']:
                    if action.get('text') and ability.get('cost', {}).get('text'):
                        cost_text = ability['cost']['text']
                        action_text = action['text']
                        if action_text.startswith(cost_text + '：'):
                            action['text'] = action_text[len(cost_text) + 1:].strip()
                        elif action_text.startswith(cost_text):
                            action['text'] = action_text[len(cost_text):].strip()
                        # Also check for partial cost prefix (any substring from cost text followed by colon)
                        if '：' in action_text:
                            # Try to find the longest substring from cost text that appears at the start of action text
                            for i in range(len(cost_text), 0, -1):
                                substring = cost_text[-i:]
                                if action_text.startswith(substring + '：'):
                                    action['text'] = action_text[len(substring) + 1:].strip()
                                    break
            
            # Fix corrupted action text starting with "こ" instead of "この"
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        if action_text.startswith('こコスト'):
                            action['text'] = 'この' + action_text[1:]
            
            # Fix action type mismatch (member_to_wait with draw text)
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'action' in action and 'text' in action:
                        action_type = action['action']
                        action_text = action['text']
                        # If action is member_to_wait but text is about drawing cards, fix it
                        if action_type == 'member_to_wait' and 'カードを1枚引く' in action_text and 'ウェイト' not in action_text:
                            action['action'] = 'draw_cards'
                        # If action is draw_cards but text is about waiting member, fix it
                        elif action_type == 'draw_cards' and 'ウェイト' in action_text and 'カードを1枚引く' not in action_text:
                            action['action'] = 'member_to_wait'
            
            # Fix leading punctuation in action texts
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        # Remove leading punctuation (、。)
                        if action_text and action_text[0] in '、。':
                            action['text'] = action_text[1:].strip()
                        # Remove leading position markers like 【右サイド】
                        if action_text and action_text.startswith('【'):
                            # Remove the position marker
                            idx = action_text.find('】')
                            if idx != -1:
                                action['text'] = action_text[idx + 1:].strip()
                        # Remove trailing punctuation (：)
                        if action_text and action_text.endswith('：'):
                            action['text'] = action_text[:-1].strip()
                        # Fix double periods
                        if action_text and action_text.endswith('。。'):
                            action['text'] = action_text[:-1].strip()
                        # Fix truncated action text ending with opening parenthesis
                        if action_text and '（' in action_text and action_text.endswith('（'):
                            action['text'] = action_text[:-1].strip()
                        # Also check if action text contains opening parenthesis with text after it
                        if action_text and '（' in action_text:
                            # If the part after the parenthesis is a note, remove it
                            idx = action_text.find('（')
                            if idx != -1 and len(action_text) > idx + 1:
                                # Check if it's a note (starts with parenthesized text)
                                after_paren = action_text[idx:]
                                if len(after_paren) < 20 and not '。' in after_paren:
                                    action['text'] = action_text[:idx].strip()
                                # Also check if the note is already in the ability's notes field
                                if ability.get('notes') and after_paren in str(ability['notes']):
                                    action['text'] = action_text[:idx].strip()
            
            # Fix truncated condition text ending with 'の' instead of 'の場合' (additional check)
            if isinstance(effect.get('condition'), dict):
                if 'text' in effect['condition']:
                    condition_text = effect['condition']['text']
                    # If condition text ends with 'の' and full text has 'の場合', append '場合'
                    if condition_text.endswith('の') and not condition_text.endswith('の場合'):
                        full_effect_text = effect.get('text', '')
                        if full_effect_text:
                            # Normalize digits for comparison
                            from parser_utils import normalize_fullwidth_digits
                            normalized_condition = normalize_fullwidth_digits(condition_text)
                            normalized_full = normalize_fullwidth_digits(full_effect_text)
                            if normalized_condition + '場合' in normalized_full:
                                effect['condition']['text'] = condition_text + '場合'
                            elif normalized_condition + 'の' in normalized_full:
                                # Check if the full text has 'の場合' after condition_text
                                idx = normalized_full.find(normalized_condition)
                                if idx != -1 and normalized_full[idx + len(normalized_condition):].startswith('場合'):
                                    effect['condition']['text'] = condition_text + '場合'
            
            # Fix truncated condition text ending with "の" to "の場合"
            if isinstance(effect.get('condition'), dict):
                if 'text' in effect['condition'] and effect['condition']['text'].endswith('の'):
                    # Check if the full text has "の場合" after this
                    condition_text = effect['condition']['text']
                    full_effect_text = effect.get('text', '')
                    if full_effect_text.startswith(condition_text + '場合'):
                        effect['condition']['text'] = condition_text + '場合'
            
            # Fix truncated condition text starting with "の" (missing source location)
            if isinstance(effect.get('condition'), dict):
                if 'text' in effect['condition']:
                    condition_text = effect['condition']['text']
                    # If condition text starts with "の", try to reconstruct
                    if condition_text.startswith('の'):
                        full_effect_text = effect.get('text', '')
                        if full_effect_text:
                            # Check if the full text contains the condition text without the leading "の"
                            if condition_text[1:] in full_effect_text:
                                # Find where it appears and extract the prefix
                                idx = full_effect_text.find(condition_text[1:])
                                if idx > 0:
                                    prefix = full_effect_text[:idx]
                                    # Check if prefix ends with "の" to avoid duplicate
                                    if prefix.endswith('の'):
                                        effect['condition']['text'] = prefix + condition_text[1:]
                                    else:
                                        effect['condition']['text'] = prefix + condition_text
                    # Remove truncated note text from condition text (ends with opening parenthesis)
                    if '\n（' in effect['condition']['text']:
                        parts = effect['condition']['text'].split('\n（', 1)
                        if len(parts) > 1:
                            effect['condition']['text'] = parts[0].strip()
            
            # Fix condition text including action text
            if isinstance(effect.get('condition'), dict):
                if 'text' in effect['condition'] and isinstance(effect.get('actions'), list):
                    condition_text = effect['condition']['text']
                    for action in effect['actions']:
                        if isinstance(action, dict) and 'text' in action:
                            action_text = action['text']
                            # If condition text starts with action text followed by "。", remove it
                            if action_text.endswith('。') and condition_text.startswith(action_text):
                                effect['condition']['text'] = condition_text[len(action_text):].strip()
                            # If condition text starts with action text followed by "自分", remove it
                            elif condition_text.startswith(action_text + '自分'):
                                effect['condition']['text'] = condition_text[len(action_text):].strip()
            
            # Fix condition text including cost text
            if isinstance(effect.get('condition'), dict):
                if 'text' in effect['condition'] and ability.get('cost') and isinstance(ability.get('cost'), dict) and ability.get('cost', {}).get('text'):
                    cost_text = ability['cost']['text']
                    condition_text = effect['condition']['text']
                    # Remove cost text prefix (with or without colon)
                    if condition_text.startswith(cost_text + '：'):
                        effect['condition']['text'] = condition_text[len(cost_text) + 1:].strip()
                    elif condition_text.startswith(cost_text):
                        effect['condition']['text'] = condition_text[len(cost_text):].strip()
            
            # Fix condition text including action text (different patterns)
            if isinstance(effect.get('condition'), dict):
                if 'text' in effect['condition'] and isinstance(effect.get('actions'), list):
                    condition_text = effect['condition']['text']
                    for action in effect['actions']:
                        if isinstance(action, dict) and 'text' in action:
                            action_text = action['text']
                            # If condition text starts with "カードを1枚引く。" followed by condition, remove it
                            if condition_text.startswith('カードを1枚引く。') and '自分のステージに' in condition_text:
                                effect['condition']['text'] = condition_text.replace('カードを1枚引く。', '', 1).strip()
                            # If condition text starts with action text followed by "。", remove it
                            elif action_text.endswith('。') and condition_text.startswith(action_text):
                                effect['condition']['text'] = condition_text[len(action_text):].strip()
                            # If condition text starts with action text followed by "自分", remove it
                            elif condition_text.startswith(action_text + '自分'):
                                effect['condition']['text'] = condition_text[len(action_text):].strip()
                            # If condition text starts with card discard action followed by condition, remove it
                            if '控え室に置く。それらの中に' in condition_text:
                                parts = condition_text.split('控え室に置く。それらの中に', 1)
                                if len(parts) > 1:
                                    effect['condition']['text'] = 'それらの中に' + parts[1].strip()
                            # If condition text starts with card discard action followed by "それらがすべて", remove it
                            if '控え室に置く。それらがすべて' in condition_text:
                                parts = condition_text.split('控え室に置く。それらがすべて', 1)
                                if len(parts) > 1:
                                    effect['condition']['text'] = 'それらがすべて' + parts[1].strip()
            
            # Fix leading '、' punctuation in condition text (including nested conditions)
            def fix_condition_punctuation(node):
                if isinstance(node, dict):
                    if 'condition' in node and isinstance(node['condition'], dict):
                        if 'text' in node['condition']:
                            condition_text = node['condition']['text']
                            if condition_text and condition_text[0] in '、。':
                                node['condition']['text'] = condition_text[1:].strip()
                            # Fix use limit marker prefix in condition text
                            if condition_text and condition_text.startswith('［ターン1回］'):
                                node['condition']['text'] = condition_text.replace('［ターン1回］', '', 1).strip()
                            elif condition_text and condition_text.startswith('[ターン1回]'):
                                node['condition']['text'] = condition_text.replace('[ターン1回]', '', 1).strip()
                        # Recursively fix nested conditions
                        fix_condition_punctuation(node['condition'])
                    if 'actions' in node:
                        for action in node['actions']:
                            fix_condition_punctuation(action)
            fix_condition_punctuation(effect)
            
            # Fix action text including condition text
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        # Remove condition markers like "場合、" from start of action text
                        for marker in ['場合、', '場合', 'とき、', 'とき']:
                            if action_text.startswith(marker):
                                action['text'] = action_text[len(marker):].strip()
                                break
                        # Also check if action text starts with condition text
                        if isinstance(effect.get('condition'), dict):
                            condition_text = effect['condition'].get('text', '')
                            if condition_text and action_text.startswith(condition_text) and len(action_text) > len(condition_text):
                                remaining = action_text[len(condition_text):].strip()
                                if remaining and (remaining[0] in '、。' or 'を' in remaining or '加える' in remaining or '引く' in remaining):
                                    action['text'] = remaining
            
            # Fix truncated action text by reconstructing from full text
            if isinstance(effect.get('actions'), list):
                full_effect_text = effect.get('text', '')
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        # If action text ends with "この能力を起動するためのコストは", it's truncated
                        if action_text.endswith('この能力を起動するためのコストは'):
                            # Find the complete text in the full effect text
                            if full_effect_text and action_text in full_effect_text:
                                # Extract the complete action text
                                idx = full_effect_text.find(action_text)
                                if idx != -1:
                                    # Get everything from the start of the action text to the end of the full text
                                    complete_text = full_effect_text[idx:].strip()
                                    # But we need to split it correctly - the note should be in a separate action
                                    if '。' in complete_text:
                                        parts = complete_text.split('。', 1)
                                        if len(parts) > 1:
                                            action['text'] = parts[0] + '。'
                        # If action text starts with "場合、" and there's a condition, reconstruct from full text
                        if action_text.startswith('場合、') and isinstance(effect.get('condition'), dict):
                            condition_text = effect['condition'].get('text', '')
                            if condition_text and full_effect_text:
                                # The action text should be the part after "場合、" in the full effect text
                                if condition_text + '場合、' in full_effect_text:
                                    idx = full_effect_text.find(condition_text + '場合、')
                                    if idx != -1:
                                        reconstructed = full_effect_text[idx + len(condition_text + '場合、'):].strip()
                                        # Remove the note in parentheses if present
                                        if '\n(' in reconstructed:
                                            reconstructed = reconstructed.split('\n(')[0].strip()
                                        if reconstructed:
                                            action['text'] = reconstructed
            
            # Fix leading punctuation in use_limitless_text and costless_text
            if ability.get('use_limitless_text') and ability['use_limitless_text'].startswith('：'):
                ability['use_limitless_text'] = ability['use_limitless_text'][1:].strip()
            if ability.get('costless_text') and ability['costless_text'].startswith('：'):
                ability['costless_text'] = ability['costless_text'][1:].strip()
            
            # Fix use_limitless_text with cost prefix like "支払ってもよい："
            if ability.get('use_limitless_text') and ability.get('cost') and isinstance(ability.get('cost'), dict) and ability['cost'].get('text'):
                cost_text = ability['cost']['text']
                use_limitless_text = ability['use_limitless_text']
                # Remove cost text prefix (with or without colon)
                if use_limitless_text.startswith(cost_text + '：'):
                    ability['use_limitless_text'] = use_limitless_text[len(cost_text) + 1:].strip()
                elif use_limitless_text.startswith(cost_text):
                    ability['use_limitless_text'] = use_limitless_text[len(cost_text):].strip()
                # Also check with energy icons stripped from cost text
                cost_text_no_icons = cost_text
                for icon in ['{{icon_energy.png|E}}', '{{icon_energy.png|e}}']:
                    cost_text_no_icons = cost_text_no_icons.replace(icon, '')
                if cost_text_no_icons != cost_text:
                    if use_limitless_text.startswith(cost_text_no_icons + '：'):
                        ability['use_limitless_text'] = use_limitless_text[len(cost_text_no_icons) + 1:].strip()
                    elif use_limitless_text.startswith(cost_text_no_icons):
                        ability['use_limitless_text'] = use_limitless_text[len(cost_text_no_icons):].strip()
            
            # Fix escaped quotes in effect text (backslash-quote pattern)
            if isinstance(effect.get('text'), str):
                if effect['text'].endswith('\\\"'):
                    effect['text'] = effect['text'][:-2].strip()
                elif effect['text'].endswith('"'):
                    effect['text'] = effect['text'][:-1].strip()
            
            # Fix incomplete action text ending with "のうち" (should include "、1つを選ぶ")
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        if action_text.endswith('のうち') and not action_text.endswith('、1つを選ぶ'):
                            full_effect_text = effect.get('text', '')
                            if full_effect_text and action_text + '、1つを選ぶ' in full_effect_text:
                                action['text'] = action_text + '、1つを選ぶ'
            
            # Fix action text including activation restriction
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        # If action text includes "この能力は、...場合のみ起動できる", remove it
                        if 'この能力は' in action_text and '起動できる' in action_text:
                            idx = action_text.find('この能力は')
                            if idx != -1:
                                action['text'] = action_text[:idx].strip()
            
            # Fix incomplete action text ending with "から" (should include the rest of the action)
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        if action_text.endswith('から') and len(action_text) < 20:
                            full_effect_text = effect.get('text', '')
                            if full_effect_text and action_text in full_effect_text:
                                # Extract the complete action text
                                idx = full_effect_text.find(action_text)
                                if idx != -1 and '。' in full_effect_text[idx:]:
                                    complete = full_effect_text[idx:].split('。')[0] + '。'
                                    if len(complete) > len(action_text):
                                        action['text'] = complete
            
            # Fix subject field including action text or choice text
            if effect.get('subject'):
                subject_text = effect['subject']
                # If subject includes choice text like "のうち、1つを選ぶ", split it
                if 'のうち、1つを選ぶ' in subject_text:
                    parts = subject_text.split('のうち、1つを選ぶ', 1)
                    if len(parts) > 1:
                        effect['subject'] = parts[1].strip()
                # If subject includes action text like "スコアを+１し、", split it
                if 'スコアを+１し、' in subject_text:
                    parts = subject_text.split('スコアを+１し、', 1)
                    if len(parts) > 1:
                        effect['subject'] = parts[1].strip()
                # Remove leading punctuation
                if effect['subject'] and effect['subject'][0] in '。、':
                    effect['subject'] = effect['subject'][1:].strip()
            
            # Fix empty condition text for position conditions
            if isinstance(effect.get('condition'), dict):
                if 'text' in effect['condition'] and effect['condition']['text'] == '':
                    if effect['condition'].get('type') == 'position':
                        position = effect['condition'].get('value', '')
                        if position == 'right_side':
                            effect['condition']['text'] = '右サイドエリアにいる'
                        elif position == 'left_side':
                            effect['condition']['text'] = '左サイドエリアにいる'
                        elif position == 'center':
                            effect['condition']['text'] = 'センターにいる'
            
            # Fix action texts that have full effect text instead of specific action text
            if isinstance(effect.get('actions'), list) and len(effect['actions']) > 1:
                full_effect_text = effect.get('text', '')
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        # If action text is the same as full effect text, try to extract specific text
                        if action_text == full_effect_text and len(action_text) > 20:
                            # Extract based on action type
                            action_type = action.get('action', '')
                            if action_type == 'look_at_cards' and '見る' in action_text:
                                # Extract just the look_at_cards part
                                if '。' in action_text:
                                    action['text'] = action_text.split('。')[0] + '。'
                            elif action_type == 'add_to_hand' and '手札に加えてもよい' in action_text:
                                # Extract the add_to_hand part
                                if 'その中から' in action_text:
                                    idx = action_text.find('その中から')
                                    if idx != -1 and '。' in action_text[idx:]:
                                        part = action_text[idx:].split('。')[0]
                                        action['text'] = part + '。'
                            elif action_type == 'discard_to_waitroom' and '控え室に置く' in action_text:
                                # Extract the discard part
                                if '残りを' in action_text:
                                    idx = action_text.find('残りを')
                                    if idx != -1:
                                        extracted = action_text[idx:]
                                        # Only add period if it doesn't already end with one
                                        if not extracted.endswith('。'):
                                            action['text'] = extracted + '。'
                                        else:
                                            action['text'] = extracted
            
            # Fix action text including note text (notes starting with "次の")
            if isinstance(effect.get('actions'), list):
                for action in effect['actions']:
                    if isinstance(action, dict) and 'text' in action:
                        action_text = action['text']
                        # If action text includes note starting with "次の", split it
                        if '。次の' in action_text:
                            parts = action_text.split('。次の', 1)
                            if len(parts) > 1:
                                action['text'] = parts[0] + '。'
                                # The note could be added as a separate field if needed
            
            # Fix null triggers
            if ability.get('triggers') is None:
                match = re.search(r'\{\{[^}]+\|([^\}]+)\}\}', ability.get('full_text', ''))
                if match:
                    ability['triggers'] = match.group(1)
            # Remove excessive trigger duplication
            if isinstance(effect.get('actions'), list):
                trigger_count = 1 if 'trigger' in effect else 0
                for action in effect['actions']:
                    if isinstance(action, dict) and 'trigger' in action:
                        trigger_count += 1
                if trigger_count > 3:
                    for action in effect['actions']:
                        if isinstance(action, dict) and 'trigger' in action:
                            del action['trigger']
            # Remove non-action metadata
            if isinstance(effect.get('actions'), list):
                filtered = [a for a in effect['actions'] if isinstance(a, dict) and 'action' in a]
                if filtered:
                    effect['actions'] = filtered
                elif effect.get('actions'):  # Would be empty, add placeholder
                    effect['actions'] = [{'action': 'unknown', 'text': effect.get('text', ''), 'parsing_error': 'Action field missing'}]
    
    # Fix missing source/destination fields in actions (run after all other fixes)
    for ability in data.get('unique_abilities', []):
        if ability.get('effect'):
            fix_action_fields(ability['effect'])
        if ability.get('cost'):
            fix_action_fields(ability['cost'])
        if ability.get('payment'):
            fix_action_fields(ability['payment'])
    return data


def extract_metadata(ability):
    """Extract metadata fields from ability text."""
    full_text = ability.get('full_text', '')
    # Use limit
    use_limit = ability.get('use_limit')
    if not use_limit:
        if 'ターン1回' in full_text:
            use_limit = 'turn1'
    # Position requirement
    position = None
    if 'センター' in full_text or 'センター' in ability.get('costless_text', ''):
        position = 'center'
    elif '左サイド' in full_text or '左サイド' in ability.get('costless_text', ''):
        position = 'left_side'
    elif '右サイド' in full_text or '右サイド' in ability.get('costless_text', ''):
        position = 'right_side'
    # Notes
    notes = None
    if '（' in full_text or '(' in full_text:
        notes = re.findall(r'（[^）]+）|\([^)]*\)', full_text)
    return use_limit, position, notes


def main():
    """Load input data, process abilities, and write the updated JSON."""
    with open('data/abilities_extracted_from_cards.json', 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    data = process_abilities(data)
    
    # Apply metadata fields
    for ability in data['unique_abilities']:
        use_limit, position, notes = extract_metadata(ability)
        if use_limit:
            ability['use_limit'] = use_limit
        if position:
            ability['position_requirement'] = position
        if notes:
            ability['notes'] = notes
        
        # Apply trigger to effect tree
        trigger = ability.get('triggers')
        if trigger and ability.get('effect'):
            def set_trigger(node):
                if 'trigger' not in node:
                    node['trigger'] = trigger
            walk_nodes(ability['effect'], set_trigger)
    
    data = post_process(data)
    prune_empty_raw_nodes(data)
    
    with open('data/abilities_extracted_from_cards.json', 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

if __name__ == '__main__':
    main()

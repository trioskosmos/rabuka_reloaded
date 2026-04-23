"""Parser for ability extraction - structural approach based on actual data analysis."""
import re
from typing import Dict, Any, Optional, Tuple, List, Union
from parser_utils import (
    extract_count,
    extract_group_name,
    normalize_fullwidth_digits,
    strip_suffix_period,
)

# ============== CONFIGURATION CONSTANTS ==============
MAX_CHARACTER_NAME_LENGTH = 10
SPLIT_LIMIT = 1

# ============== SOURCE PATTERNS (FROM) ==============
SOURCE_PATTERNS = [
    ('控え室から', 'discard'),
    ('控え室か ら', 'discard'),  # Q226: Handle unusual spacing
    ('手札から', 'hand'),
    ('デッキから', 'deck'),
    ('デッキの上から', 'deck_top'),
    ('山札から', 'deck'),
    ('ステージから', 'stage'),
    ('エネルギー置き場から', 'energy_zone'),
    ('ライブカード置き場から', 'live_card_zone'),
    ('成功ライブカード置き場から', 'success_live_zone'),
    ('からライブカード', 'discard'),  # Q226: Handle "～からライブカード" pattern
    ('デッキの一番下から', 'deck_bottom'),
    ('相手の控え室から', 'discard'),
    ('相手の控え室にある', 'discard'),
]

# ============== DESTINATION PATTERNS (TO) ==============
DESTINATION_PATTERNS = [
    ('控え室に置く', 'discard'),
    ('控え室に置いて', 'discard'),  # Handle te-form
    # Removed overly broad ('控え室', 'discard') - it was matching source locations
    ('手札に加える', 'hand'),
    ('手札に加えて', 'hand'),  # Handle te-form
    ('手札に置く', 'hand'),
    ('ステージに置く', 'stage'),
    ('ステージに登場させる', 'stage'),
    ('エネルギー置き場に置く', 'energy_zone'),
    ('ライブカード置き場に置く', 'live_card_zone'),
    ('成功ライブカード置き場に置く', 'success_live_zone'),
    ('デッキの上に置く', 'deck_top'),
    ('デッキの一番上に置く', 'deck_top'),
    ('デッキの下に置く', 'deck_bottom'),
    ('デッキの一番下に置く', 'deck_bottom'),
    ('デッキの一番下に置いて', 'deck_bottom'),  # Handle te-form
    ('デッキの一番上から4枚目に置く', 'deck_position_4'),  # Q226: 4th from top
    ('デッキの一番上から(\d+)枚目に置く', 'deck_position'),  # Q226: General deck position pattern
    ('デッキに置く', 'deck'),  # Q226: General deck placement
    ('成功ライブカード置き場に置く', 'success_live_zone'),
    ('メンバーのいないエリア', 'empty_area'),
    ('そのメンバーがいたエリア', 'same_area'),
    ('このメンバーの下に置く', 'under_member'),
    ('このメンバーの下に置いて', 'under_member'),  # Handle te-form
    # Handle "手札を1枚控え室に置く" - destination is discard
    ('枚控え室に置く', 'discard'),
    ('枚控え室に置いて', 'discard'),  # Handle te-form
]

# ============== ACTION PATTERNS ==============
ACTION_PATTERNS = [
    ('シャッフルする', 'shuffle'),
    ('シャッフルして', 'shuffle'),  # Handle te-form
    ('入れ替える', 'swap'),
    ('入れ替えて', 'swap'),  # Handle te-form
]

# ============== STATE CHANGE PATTERNS ==============
STATE_CHANGE_PATTERNS = [
    ('ウェイトにする', 'wait'),
    ('ウェイトにしてもよい', 'wait'),
    ('ウェイトにし', 'wait'),
    ('ウェイト状態で置く', 'wait'),
    ('ウェイト状態で登場させる', 'wait'),
    ('アクティブにする', 'active'),
]

# ============== LOCATION PATTERNS ==============
LOCATION_PATTERNS = [
    ('成功ライブカード置き場', 'success_live_card_zone'),
    ('ライブカード置き場', 'live_card_zone'),
    ('控え室', 'discard'),
    ('手札', 'hand'),
    ('ステージ', 'stage'),
    ('デッキ', 'deck'),
    ('エネルギーデッキ', 'energy_deck'),
    ('エネルギー置き場', 'energy_zone'),
]

# ============== CARD TYPE PATTERNS ==============
CARD_TYPE_PATTERNS = [
    ('メンバーカード', 'member_card'),
    ('メンバー', 'member_card'),
    ('ライブカード', 'live_card'),
    ('エネルギーカード', 'energy_card'),
    
]

# ============== OPERATOR PATTERNS ==============
OPERATOR_PATTERNS = [
    ('以上', '>='),
    ('以下', '<='),
    ('より少ない', '<'),
    ('より多い', '>'),
    ('未満', '<'),
    ('超', '>'),
]

# ============== CONDITION MARKERS ==============
CONDITION_MARKERS = ['場合、', 'とき、', 'なら、']

# ============== STRUCTURAL MARKERS ==============
SEQUENTIAL_MARKER = 'その後、'
CONDITIONAL_SEQUENTIAL_MARKER = 'そうした場合'
CHOICE_MARKER = '以下から1つを選ぶ'
DURATION_MARKER = 'かぎり'
COMPOUND_OPERATOR = 'かつ'
COMPOUND_OPERATOR_ALT = 'あり、'  # Alternative compound operator
PER_UNIT_MARKER = 'につき'
EACH_TIME_MARKER = 'たび'
EITHER_CASE_MARKER = 'いずれかの場合'

# ============== UTILITY FUNCTIONS ==============

def extract_by_pattern(text: str, patterns: List[Tuple[str, str]]) -> Optional[str]:
    """Generic function to extract value by matching patterns."""
    for pattern, code in patterns:
        if pattern in text:
            return code
    return None

def extract_source(text: str) -> Optional[str]:
    """Extract source location (FROM)."""
    # Special case for Q226: "自分の控え室からライブカード" pattern
    if '控え室からライブカード' in text:
        return 'discard'
    return extract_by_pattern(text, SOURCE_PATTERNS)

def extract_destination(text: str) -> Optional[str]:
    """Extract destination location (TO)."""
    # First check for specific deck position patterns (Q226)
    deck_pos_match = re.search(r'デッキの一番上から(\d+)枚目に置く', text)
    if deck_pos_match:
        return 'deck'  # Return 'deck' as the destination, position will be extracted separately
    # Also check for "置いてもよい" pattern (Q226)
    deck_pos_match2 = re.search(r'デッキの一番上から(\d+)枚目に置いてもよい', text)
    if deck_pos_match2:
        return 'deck'
    return extract_by_pattern(text, DESTINATION_PATTERNS)

def extract_location(text: str) -> Optional[str]:
    """Extract location (general)."""
    return extract_by_pattern(text, LOCATION_PATTERNS)

def extract_state_change(text: str) -> Optional[str]:
    """Extract state change (wait/active)."""
    return extract_by_pattern(text, STATE_CHANGE_PATTERNS)

def extract_card_type(text: str) -> Optional[str]:
    """Extract card type."""
    return extract_by_pattern(text, CARD_TYPE_PATTERNS)

def extract_target(text: str) -> Optional[str]:
    """Extract target (self/opponent/both/either)."""
    if ('自分の' in text and '相手の' in text) or '自分と相手の' in text:
        return 'both'
    if '自分か相手の' in text:
        return 'either'
    if '相手の' in text:
        return 'opponent'
    if '自分の' in text:
        return 'self'
    return None

def extract_operator(text: str) -> Optional[str]:
    """Extract comparison operator."""
    return extract_by_pattern(text, OPERATOR_PATTERNS)

def extract_group(text: str) -> Optional[Dict[str, Any]]:
    """Extract group information from text."""
    if '『' in text:
        group = extract_group_name(text)
        if group:
            return {
                'name': group
            }
    return None

def extract_cost_limit(text: str) -> Optional[Union[int, List[int]]]:
    """Extract cost limit."""
    # Match patterns like "4コスト以下" or "コスト4以下"
    match = re.search(r'(\d+)コスト(?:以上|以下|未満|超)', text)
    if match:
        return int(match.group(1))
    match = re.search(r'コスト(\d+)(?:以上|以下|未満|超)', text)
    if match:
        return int(match.group(1))
    return None

def extract_deck_position(text: str) -> Optional[int]:
    """Extract deck position from text like '一番上から4枚目' (4th from top)."""
    # Match patterns like "一番上から4枚目" or "上から4枚目"
    match = re.search(r'一番上から(\d+)枚目', text)
    if match:
        return int(match.group(1))
    match = re.search(r'上から(\d+)枚目', text)
    if match:
        return int(match.group(1))
    return None

def extract_deck_position_for_action(text: str) -> Optional[Dict[str, Any]]:
    """Extract deck position for action, returns PositionInfo format."""
    pos = extract_deck_position(text)
    if pos:
        return {
            'position': {
                'position': str(pos)
            }
        }
    return None

def extract_position(text: str) -> Dict[str, Any]:
    """Extract position requirement with target."""
    result = {}
    
    # Extract target
    target = extract_target(text)
    if target:
        result['target'] = target
    
    # Check for both players effect (自分と相手はそれぞれ) - override target
    if '自分と相手はそれぞれ' in text:
        result['target'] = 'both'
    
    # Extract deck position (Q226: 一番上から4枚目)
    deck_pos = extract_deck_position(text)
    if deck_pos:
        result['position'] = {
            'position': str(deck_pos)
        }
    
    # Note: Position field removed to avoid deserialization errors
    # Rust expects PositionInfo struct, not string
    
    return result if result else None

def extract_optional(text: str) -> bool:
    """Check if action is optional."""
    return 'もよい' in text or 'てもよい' in text

def extract_group_names(text: str) -> List[str]:
    """Extract all group names within 『』 brackets."""
    return re.findall(r'『([^』]+)』', text)

def extract_quoted_text(text: str) -> List[str]:
    """Extract all text within 「」 quotes."""
    return re.findall(r'「([^」]+)」', text)

def extract_parenthetical(text: str) -> List[str]:
    """Extract all text within （） parentheses."""
    return re.findall(r'（([^）]+)）', text)

def strip_parenthetical(text: str) -> str:
    """Remove parenthetical notes from text."""
    return re.sub(r'（([^）]+)）', '', text).strip()

def extract_max(text: str) -> bool:
    """Check if count has 'max' modifier (まで)."""
    return '人まで' in text or '枚まで' in text

def filter_character_names(quoted_text: List[str]) -> List[str]:
    """Filter character names from quoted text, excluding ability names."""
    # Filter out ability names (which typically contain {{ or are longer than 10 chars)
    return [c for c in quoted_text if '{{' not in c and len(c) <= 10]

def categorize_quoted_text(quoted_text: List[str]) -> Dict[str, List[str]]:
    """Categorize quoted text into character names and ability texts."""
    result = {
        'characters': [],
        'abilities': []
    }
    for q in quoted_text:
        if '{{' in q and '}}' in q:
            result['abilities'].append(q)
        else:
            result['characters'].append(q)
    return result

# ============== STRUCTURAL PARSING ==============

def identify_structure(text: str) -> Dict[str, Any]:
    """Identify the high-level structure of the text."""
    structure = {
        'has_cost': False,
        'has_condition': False,
        'has_sequential': False,
        'has_choice': False,
        'has_compound': False,
        'has_duration': False,
        'has_per_unit': False,
    }
    
    if '：' in text:
        structure['has_cost'] = True
    
    for marker in CONDITION_MARKERS:
        if marker in text:
            structure['has_condition'] = True
            break
    
    if SEQUENTIAL_MARKER in text:
        structure['has_sequential'] = True
    
    if CHOICE_MARKER in text:
        structure['has_choice'] = True
    
    if COMPOUND_OPERATOR in text:
        structure['has_compound'] = True
    
    if DURATION_MARKER in text:
        structure['has_duration'] = True
    
    if PER_UNIT_MARKER in text:
        structure['has_per_unit'] = True
    
    return structure

def split_cost_effect(text: str) -> Tuple[str, str]:
    """Split text into cost and effect parts, skipping colons inside parentheses and quotes."""
    if '：' not in text:
        return '', text
    
    # Find the first colon that's not inside parentheses or quotes
    paren_depth = 0
    quote_depth = 0
    split_index = -1
    
    for i, char in enumerate(text):
        if char == '（' or char == '(':
            paren_depth += 1
        elif char == '）' or char == ')':
            paren_depth -= 1
        elif char == '"' or char == '"':
            quote_depth += 1 if quote_depth == 0 else -1
        elif char == '：':
            # Only split if not inside parentheses or quotes
            if paren_depth == 0 and quote_depth == 0:
                split_index = i
                break
    
    if split_index >= 0:
        cost = text[:split_index].strip()
        effect = text[split_index + 1:].strip()
        return cost, effect
    else:
        # No valid split point found
        return '', text

def split_condition_action(text: str) -> Tuple[str, str]:
    """Split text into condition and action parts."""
    for marker in CONDITION_MARKERS:
        if marker in text:
            parts = text.split(marker, 1)
            return parts[0].strip(), parts[1].strip()
    return '', text

# ============== COMPONENT PARSING ==============

def parse_condition(text: str) -> Dict[str, Any]:
    """Parse a condition text."""
    condition = {
        'text': text
    }
    
    # Check for count conditions like "2枚以上ある" or "6枚以上ある"
    count_match = re.search(r'(\d+)枚以上ある', text)
    if count_match:
        condition['condition_type'] = 'card_count_condition'
        condition['count'] = int(count_match.group(1))
        condition['operator'] = '>='
        return condition
    
    # Check for count conditions with unit like "2人以上いる"
    unit_count_match = re.search(r'(\d+)(人|枚|つ)以上いる', text)
    if unit_count_match:
        condition['condition_type'] = 'card_count_condition'
        condition['count'] = int(unit_count_match.group(1))
        condition['operator'] = '>='
        condition['unit'] = unit_count_match.group(2)
        return condition
    
    # Check for "それらが両方ある" (both present) condition
    if 'それらが両方ある' in text:
        condition['condition_type'] = 'both_condition'
        return condition
    
    # Check for temporal conditions with "移動していない" (not moved)
    if 'このターン' in text and '移動していない' in text:
        condition['condition_type'] = 'temporal_condition'
        condition['temporal'] = 'this_turn'
        condition['condition'] = {
            'type': 'not_moved'
        }
        # Check for card type
        card_type = extract_card_type(text)
        if card_type:
            condition['card_type'] = card_type
        return condition
    
    # Check for temporal conditions with "移動している" (has moved)
    if 'このターン' in text and '移動している' in text:
        condition['condition_type'] = 'temporal_condition'
        condition['temporal'] = 'this_turn'
        condition['condition'] = {
            'type': 'has_moved'
        }
        # Check for card type
        card_type = extract_card_type(text)
        if card_type:
            condition['card_type'] = card_type
        return condition
    
    # Check for temporal conditions with "ライブを成功させていた" (live success)
    if 'このターン' in text and 'ライブを成功させていた' in text:
        condition['condition_type'] = 'temporal_condition'
        condition['temporal'] = 'this_turn'
        condition['condition'] = {
            'type': 'opponent_live_success'
        }
        # Check for "余剰のハートを持たずに" (no excess heart)
        if '余剰のハートを持たずに' in text:
            condition['condition']['no_excess_heart'] = True
        return condition
    
    # Check for temporal conditions with specific turn phase
    if 'このゲームの' in text and 'ターン目' in text and 'ライブフェイズ' in text:
        condition['condition_type'] = 'temporal_condition'
        # Extract turn number
        turn_match = re.search(r'(\d+)ターン目', text)
        if turn_match:
            condition['turn'] = int(turn_match.group(1))
        condition['phase'] = 'live_phase'
        return condition
    
    # Check for baton touch conditions
    if 'バトンタッチして登場した' in text:
        # Use location_condition with baton_touch_trigger field instead of baton_touch_condition
        # since engine doesn't have a handler for baton_touch_condition
        condition['condition_type'] = 'location_condition'
        condition['location'] = 'stage'
        condition['target'] = 'self'
        condition['baton_touch_trigger'] = True
        # Extract specific member if quoted (e.g., 「中須かすみ」からバトンタッチ)
        quoted = extract_quoted_text(text)
        if quoted:
            categorized = categorize_quoted_text(quoted)
            if categorized['abilities'] and len(categorized['abilities']) > 0:
                condition['source_ability'] = categorized['abilities'][0]
            if categorized['characters'] and len(categorized['characters']) > 0:
                condition['source_member'] = categorized['characters'][0]
        # Check for negation (能力を持たないメンバーから)
        if '能力を持たない' in text:
            condition['ability_negation'] = True
        # Check for cost comparison (このメンバーよりコストが低い)
        if 'コストが低い' in text or 'コストが小さい' in text:
            condition['cost_comparison'] = 'lower'
        elif 'コストが高い' in text or 'コストが大きい' in text:
            condition['cost_comparison'] = 'higher'
        return condition
    
    # Check for "このターン、自分のステージにメンバーが3回登場したとき" type temporal count conditions
    # Check this BEFORE appearance condition to prevent override
    if ('このターン' in text or 'ターン目' in text) and ('回' in text or '登場' in text):
        condition['condition_type'] = 'temporal_condition'
        condition['temporal'] = 'this_turn'
        # Extract count if present (e.g., "3回登場した" or "2回以上登場している")
        count_match = re.search(r'(\d+)回', text)
        if count_match:
            condition['count'] = int(count_match.group(1))
        condition['event'] = 'appearance' if '登場' in text else 'custom'
        # Check for specific phase
        if 'ライブフェイズ' in text:
            condition['phase'] = 'live_phase'
        elif 'メインフェイズ' in text:
            condition['phase'] = 'main_phase'
        # Check for location
        location = extract_location(text)
        if location:
            condition['location'] = location
        # Check for card type
        card_type = extract_card_type(text)
        if card_type:
            condition['card_type'] = card_type
        # Check for target
        target = extract_target(text)
        if target:
            condition['target'] = target
        # Check for all_areas flag (e.g., "エリアすべて")
        if 'エリアすべて' in text:
            condition['all_areas'] = True
        # Return early to prevent being overridden by appearance check
        return condition
    
    # Check for distinct conditions (名前が異なる) - MUST CHECK BEFORE compound conditions
    if '名前が異なる' in text or 'ユニット名がそれぞれ異なる' in text:
        # Use location_condition with distinct field instead of distinct_condition
        # since engine doesn't have a handler for distinct_condition
        condition['condition_type'] = 'location_condition'
        condition['location'] = 'stage'
        condition['target'] = 'self'
        condition['distinct'] = True
        # Set all_areas if present
        if 'エリアすべて' in text:
            condition['all_areas'] = True
        return condition
    
    # Check for OR conditions (か、 = or) - MUST CHECK BEFORE movement condition
    if 'か、' in text:
        parts = [p.strip() for p in text.split('か、') if p.strip()]
        if len(parts) >= 2:
            parsed_conditions = [parse_condition(p) for p in parts]
            # Don't filter out custom conditions - keep structure even if unparsed
            if len(parsed_conditions) >= 2:
                compound = {
                    'type': 'or_condition',
                    'conditions': parsed_conditions,
                    'text': text
                }
                # Don't set target on compound - let sub-conditions have their own targets
                return compound
    
    # Check for movement conditions
    if '移動した' in text:
        condition['condition_type'] = 'movement_condition'
        # movement is already set to 'moved' string by extract_movement
        # Don't override with boolean
        if 'movement' not in condition:
            condition['movement'] = 'moved'
        # Check for negation (移動していない)
        if '移動していない' in text:
            condition['negated'] = True
        return condition
    
    # Check for appearance conditions (登場 = appearance/appear)
    if '登場' in text:
        condition['condition_type'] = 'appearance_condition'
        condition['appearance'] = True
        # Check for all_areas flag (e.g., "エリアすべて")
        if 'エリアすべて' in text:
            condition['all_areas'] = True
        return condition
    
    # Check for energy state conditions (アクティブ状態のエネルギーがある)
    if 'エネルギーがある' in text:
        condition['condition_type'] = 'energy_state_condition'
        if 'アクティブ状態' in text:
            condition['state'] = 'active'
        return condition
    
    # Check for state conditions
    if 'ウェイト状態である' in text or 'ウェイト状態にある' in text:
        condition['condition_type'] = 'state_condition'
        condition['state'] = 'wait'
        return condition
    if 'アクティブ状態である' in text or 'アクティブ状態にある' in text or 'アクティブ状態の' in text:
        condition['condition_type'] = 'state_condition'
        condition['state'] = 'active'
        # Check if it's about energy
    
    # Check for appearance conditions (登場 = appearance/appear)
    if '登場' in text:
        condition['condition_type'] = 'appearance_condition'
        condition['appearance'] = True
        # Check for all_areas flag (e.g., "エリアすべて")
        if 'エリアすべて' in text:
            condition['all_areas'] = True
        return condition
    
    # Check for energy state conditions (アクティブ状態のエネルギーがある)
    if 'エネルギーがある' in text:
        condition['condition_type'] = 'energy_state_condition'
        if 'アクティブ状態' in text:
            condition['state'] = 'active'
        return condition
    
    # Check for state conditions
    if 'ウェイト状態である' in text or 'ウェイト状態にある' in text:
        condition['condition_type'] = 'state_condition'
        condition['state'] = 'wait'
        return condition
    if 'アクティブ状態である' in text or 'アクティブ状態にある' in text or 'アクティブ状態の' in text:
        condition['condition_type'] = 'state_condition'
        condition['state'] = 'active'
        # Check if it's about energy
        if 'エネルギー' in text:
            condition['resource_type'] = 'energy'
        return condition
    
    # Check for appearance conditions (登場 = appearance/appear)
    if '登場' in text:
        condition['condition_type'] = 'appearance_condition'
        condition['appearance'] = True
        # Check for all_areas flag (e.g., "エリアすべて")
        if 'エリアすべて' in text:
            condition['all_areas'] = True
        return condition
    
    # Check for position conditions
    position_keywords = {
        'センターエリア': 'center',
        '左サイドエリア': 'left_side',
        '右サイドエリア': 'right_side',
        'センター': 'center',
        '左サイド': 'left_side',
        '右サイド': 'right_side'
    }
    for keyword, position in position_keywords.items():
        if keyword in text:
            condition['condition_type'] = 'position_condition'
            # Don't set position as string - Rust expects PositionInfo struct
            # condition['position'] = position
            return condition
    
    # Check for state conditions
    state_keywords = {
        'ウェイト状態である': 'wait',
        'ウェイト状態にある': 'wait',
        'アクティブ状態である': 'active',
        'アクティブ状態にある': 'active'
    }
    for keyword, state in state_keywords.items():
        if keyword in text:
            condition['condition_type'] = 'state_condition'
            condition['state'] = state
            return condition
    
    # Check for active energy condition (アクティブ状態の自分のエネルギーがある)
    if 'アクティブ状態の自分のエネルギー' in text or 'アクティブ状態のエネルギー' in text:
        condition['condition_type'] = 'active_energy_condition'
        condition['card_type'] = 'energy_card'
        return condition
    
    # Check for state transition conditions (アクティブ状態からウェイト状態になった)
    if 'から' in text and '状態' in text and 'なった' in text:
        condition['condition_type'] = 'state_transition_condition'
        return condition
    
    # Check for ability negation
    if '能力も持たない' in text or '能力を持たない' in text:
        condition['condition_type'] = 'ability_negation_condition'
        condition['ability_negation'] = True
        return condition
    
    # Check for heart negation (ブレードハートを持たない)
    if 'ブレードハートを持たない' in text or 'ハートを持たない' in text:
        condition['condition_type'] = 'heart_negation_condition'
        condition['heart_negation'] = True
        return condition
    
    # Check for same group name condition
    if '同じグループ名を持つ' in text:
        condition['condition_type'] = 'same_group_condition'
        condition['same_group'] = True
        return condition
    
    # Check for heart variety condition (6種類以上ある)
    if '種類以上ある' in text or '種類以上含まれる' in text:
        condition['condition_type'] = 'heart_variety_condition'
        variety_count = extract_count(text)
        if variety_count:
            condition['variety_count'] = variety_count
        return condition
    
    # Check for energy payment negation (E支払わないかぎり)
    if '支払わないかぎり' in text:
        condition['condition_type'] = 'payment_negation_condition'
        condition['negated'] = True
        # Extract payment amount
        payment_count = extract_count(text)
        if payment_count:
            condition['payment_count'] = payment_count
        return condition
    
    # Check for negative choice (そうしなかった)
    if 'そうしなかった' in text:
        condition['condition_type'] = 'negative_choice_condition'
        return condition
    
    # Check for any_of conditions (いずれか)
    if 'いずれか' in text:
        # Extract values
        values_match = re.search(r'(\d+)[、\s]*(\d+)[、\s]*(\d+)[、\s]*(\d+)[、\s]*(\d+)', text)
        if values_match:
            values = [int(g) for g in values_match.groups()]
            condition['condition_type'] = 'any_of_condition'
            condition['values'] = values
            # Don't set any_of as boolean - type already indicates it's any_of condition
            # condition['any_of'] = True
            return condition
    
    # Check for yell-revealed card conditions (エールにより公開された自分のカードの中に〜)
    if 'エールにより公開された自分のカードの中に' in text or 'エールによって公開される自分のカードの中に' in text:
        condition['condition_type'] = 'yell_revealed_condition'
        condition['source'] = 'yell_revealed'
        return condition
    # Check for yell action conditions (自分がエールした)
    if 'エールした' in text:
        condition['condition_type'] = 'yell_action_condition'
        return condition
    
    # Check for live card count conditions (自分のライブ中のライブカードが〜)
    if '自分のライブ中のライブカード' in text:
        condition['condition_type'] = 'location_count_condition'
        condition['location'] = 'live'
        return condition
    
    # Check for character presence conditions (自分のステージに「X」がいる)
    if re.search(r'自分のステージに「[^」]+」がいる', text) or re.search(r'自分のステージに「[^」]+」か「[^」]+」がいる', text) or re.search(r'自分のステージに「[^」]+」と「[^」]+」がいる', text):
        condition['condition_type'] = 'character_presence_condition'
        # Extract character names - exclude ability names (which contain icons or are longer)
        char_match = re.findall(r'「([^」]+)」', text)
        if char_match:
            # Filter out ability names (which typically contain {{ or are longer than 10 chars)
            characters = filter_character_names(char_match)
            if characters:
                condition['characters'] = characters
        # Check for OR pattern (か)
        if 'か' in text:
            condition['or_condition'] = True
        return condition
    
    # Check for comparison conditions (after extraction, if comparison fields exist)
    # This will be set after the extraction phase below
    
    # Check for compound conditions (かつ or あり、) - MUST CHECK AFTER distinct conditions
    if COMPOUND_OPERATOR in text or COMPOUND_OPERATOR_ALT in text:
        # Use whichever operator is present
        operator = COMPOUND_OPERATOR if COMPOUND_OPERATOR in text else COMPOUND_OPERATOR_ALT
        parts = [p.strip() for p in text.split(operator) if p.strip()]
        if len(parts) >= 2:
            parsed_conditions = [parse_condition(p) for p in parts]
            # Don't filter out custom conditions - keep structure even if unparsed
            if len(parsed_conditions) >= 2:
                compound = {
                    'condition_type': 'compound',
                    'operator': 'and',
                    'conditions': parsed_conditions,
                    'text': text
                }
                # Don't set target on compound - let sub-conditions have their own targets
                return compound
    
    # Check for name-based matching conditions (～と同じ名前を持つ)
    if 'と同じ名前を持つ' in text or 'と同じ名前の' in text:
        condition['condition_type'] = 'name_match_condition'
        # Extract the reference name
        name_match = re.search(r'([^と]+)と同じ名前', text)
        if name_match:
            condition['reference_name'] = name_match.group(1).strip()
        return condition
    
    # Check for except conditions (以外)
    if '以外' in text:
        condition['except'] = True
        # Extract the thing being excluded
        except_match = re.search(r'([^以外]+)以外', text)
        if except_match:
            except_target = except_match.group(1).strip()
            # Strip parenthetical notes from except_target
            if '(' in except_target:
                parts = except_target.split('(')
                except_target = parts[0].strip() if len(parts) > 0 else except_target
            elif '（' in except_target:
                parts = except_target.split('（')
                except_target = parts[0].strip() if len(parts) > 0 else except_target
            # Strip newlines and incomplete text
            parts = except_target.split('\n')
            except_target = parts[0].strip() if len(parts) > 0 else except_target
            condition['except_target'] = except_target
        # Check if the exclusion is quoted (e.g., 「name」以外)
        quoted_exclusions = extract_quoted_text(text)
        if quoted_exclusions:
            categorized = categorize_quoted_text(quoted_exclusions)
            if categorized['abilities']:
                condition['except_abilities'] = categorized['abilities']
            if categorized['characters']:
                condition['except_quoted'] = categorized['characters']
    
    # Extract target
    target = extract_target(text)
    if target:
        condition['target'] = target
    
    # Extract location
    location = extract_location(text)
    if location:
        condition['location'] = location
    
    # Extract heart count condition (e.g., "heart02を3つ以上持つ") - extract BEFORE other extractions
    if 'heart' in text and ('つ以上持つ' in text or '枚持つ' in text or 'つ持つ' in text):
        heart_count = extract_count(text)
        if heart_count:
            condition['count'] = heart_count
            # Extract specific heart type from icon pattern
            heart_types = {
                'heart_01': 'heart_01',
                'heart_02': 'heart_02',
                'heart_06': 'heart_06'
            }
            for pattern, resource_type in heart_types.items():
                if pattern in text:
                    condition['resource_type'] = resource_type
                    break
            else:
                condition['resource_type'] = 'heart'
    
    # Extract energy count condition
    if 'エネルギー' in text and '枚' in text:
        condition['resource_type'] = 'energy'
        energy_count = extract_count(text)
        if energy_count:
            condition['count'] = energy_count
    
    # Extract surplus heart condition
    if '余剰ハート' in text:
        condition['resource_type'] = 'surplus_heart'
        surplus_count = extract_count(text)
        if surplus_count:
            condition['count'] = surplus_count
    
    # Extract source and destination for action-like conditions
    # This handles cases like "控え室からステージに登場させる" in conditions
    source = extract_source(text)
    if source:
        condition['source'] = source
    destination = extract_destination(text)
    if destination:
        condition['destination'] = destination
    
    # Extract card type
    card_type = extract_card_type(text)
    if card_type:
        condition['card_type'] = card_type
    
    # Extract count
    count = extract_count(text)
    if count:
        condition['count'] = count
    
    # Extract operator
    operator = extract_operator(text)
    if operator:
        condition['operator'] = operator
    
    # Extract comparison information (e.g., "相手より高い")
    comparison_targets = {
        '相手より': 'opponent',
        '自分より': 'self',
        'このメンバーより': 'self'
    }
    comparison_operators = {
        '高い': '>',
        '低い': '<',
        '少ない': '<',
        '多い': '>',
        '大きい': '>',
        '小さい': '<'
    }
    for keyword, target in comparison_targets.items():
        if keyword in text:
            condition['comparison_target'] = target
            # Extract operator
            for op_keyword, operator in comparison_operators.items():
                if op_keyword in text:
                    condition['comparison_operator'] = operator
                    break
            break
    
    # Extract comparison type (score, cost, etc.)
    comparison_types = {
        'スコア': 'score',
        'コスト': 'cost'
    }
    for keyword, comp_type in comparison_types.items():
        if keyword in text:
            condition['comparison_type'] = comp_type
            break
    
    # Extract aggregate (total/sum)
    if '合計' in text:
        condition['aggregate'] = 'total'
    
    # Extract exact match (ちょうど or 同じ)
    if 'ちょうど' in text or '同じ' in text:
        condition['operator'] = '='
        if '同じ' in text:
            condition['comparison_type'] = 'equality'
            condition['condition_type'] = 'comparison_condition'
    
    # Extract negation (いない = not exist)
    if 'いない' in text and 'メンバーがいない' in text:
        condition['negated'] = True
    
    # Extract includes pattern (含む = includes/contains)
    if '含む' in text:
        # Check if it's a nested condition like "その中に～を含む"
        if 'その中に' in text and '含む' in text:
            condition['includes'] = True
    
    # Extract movement condition (移動した / 移動する = moved / moves)
    if '移動した' in text or '移動する' in text:
        # Set movement as string, not boolean
        if '移動した' in text:
            condition['movement'] = 'moved'
        elif '移動する' in text:
            condition['movement'] = 'moves'
    
    # Extract baton touch trigger condition
    if 'バタンタッチ' in text:
        condition['trigger_type'] = 'baton_touch'
    
    # Extract movement state (～ている = ongoing state vs ～た = completed)
    if '移動している' in text:
        condition['movement_state'] = 'has_moved'
    
    # Extract temporal scope
    temporal_keywords = {
        'このターン': 'this_turn',
        'このライブ': 'this_live'
    }
    for keyword, temporal in temporal_keywords.items():
        if keyword in text:
            condition['temporal'] = temporal
            break
    
    # Extract distinct/unique flags
    distinct_keywords = {
        '名前が異なる': 'name',
        'カード名が異なる': 'card_name',
        'グループ名が異なる': 'group_name',
        'コストがそれぞれ異なる': 'cost'
    }
    for keyword, distinct_type in distinct_keywords.items():
        if keyword in text:
            # Don't set distinct as string - Rust expects boolean
            # condition['distinct'] = distinct_type
            condition['distinct'] = True
            break
    
    # Extract all areas flag - check this earlier before other location checks
    if 'エリアすべて' in text:
        condition['all_areas'] = True
    
    # Extract exclude_self flag (other members)
    exclude_self_keywords = ['ほかのメンバー', 'このメンバー以外', 'このメンバー以外の']
    for keyword in exclude_self_keywords:
        if keyword in text:
            condition['exclude_self'] = True
            break
    
    # Extract any_of pattern with multiple values (e.g., "10、20、30のいずれか")
    if 'いずれか' in text:
        # Try to extract the values
        values_match = re.search(r'(\d+)(?:、(\d+))+(?:のいずれか)', text)
        if values_match:
            values = re.findall(r'\d+', values_match.group(0))
            condition['values'] = [int(v) for v in values]
            # Don't set any_of as boolean - type already indicates it's any_of condition
            # condition['any_of'] = True
    
    # Extract group
    group = extract_group(text)
    if group:
        condition['group'] = group
    
    # Extract group names from 『』 brackets - only add if group object not already present
    # Check if group actually has content (name field)
    if not group or not group.get('name'):
        group_names = extract_group_names(text)
        if group_names:
            condition['group_names'] = group_names
    
    # Extract cost limit
    cost_limit = extract_cost_limit(text)
    if cost_limit:
        condition['cost_limit'] = cost_limit
    
    # Extract position
    position = extract_position(text)
    if position:
        if isinstance(position, dict):
            condition.update(position)
        else:
            # Output as PositionInfo struct format
            condition['position'] = {
                'position': position
            }
    
    # Determine condition type
    if location and count and operator:
        condition['condition_type'] = 'location_count_condition'
        # If group is present, it's a group-specific location count condition
        if group:
            condition['condition_type'] = 'group_location_count_condition'
    elif cost_limit:
        condition['condition_type'] = 'cost_limit_condition'
    elif condition.get('resource_type') and count and operator:
        # Heart count or energy count conditions with group/location context
        condition['condition_type'] = 'resource_count_condition'
        if group:
            condition['condition_type'] = 'group_resource_count_condition'
    elif group or group_names:
        condition['condition_type'] = 'group_condition'
    elif location and card_type:
        condition['condition_type'] = 'location_condition'
    elif location and position:
        condition['condition_type'] = 'position_condition'
    elif condition.get('resource_type') == 'energy' and count:
        condition['condition_type'] = 'energy_condition'
    elif condition.get('resource_type') == 'surplus_heart':
        condition['condition_type'] = 'surplus_heart_condition'
    elif source and destination:
        condition['condition_type'] = 'move_action_condition'
    elif (source or destination) and (location or condition.get('destination')):
        condition['condition_type'] = 'location_condition'
    elif condition.get('comparison_target') or condition.get('comparison_type'):
        condition['condition_type'] = 'comparison_condition'
    elif condition.get('operator') and condition.get('target'):
        condition['condition_type'] = 'comparison_condition'
    elif condition.get('aggregate') == 'total':
        condition['condition_type'] = 'score_threshold_condition'
    elif condition.get('movement') and count:
        condition['condition_type'] = 'movement_count_condition'
    elif condition.get('except') and condition.get('card_type'):
        condition['condition_type'] = 'action_restriction_condition'
    elif condition.get('except') and count:
        condition['condition_type'] = 'except_count_condition'
    elif condition.get('card_type') and count:
        condition['condition_type'] = 'card_count_condition'
    elif (location or condition.get('target')) and count and operator:
        condition['condition_type'] = 'location_count_condition'
    elif location and condition.get('target'):
        condition['condition_type'] = 'location_condition'
    else:
        condition['condition_type'] = 'custom'
    
    return condition

def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text."""
    # Check for optional draw action "カードを1枚引いてもよい" - CHECK THIS FIRST
    if 'カードを1枚引いてもよい' in text:
        return {
            'text': text,
            'action': 'draw_card',
            'count': 1,
            'optional': True
        }
    
    # Check for per-unit scaling (e.g., "メンバー1人につき") - CHECK THIS FIRST before any text splitting
    if PER_UNIT_MARKER in text:
        # Extract the per-unit pattern
        per_unit_match = re.search(r'(.*?)につき', text)
        if per_unit_match:
            per_unit_text = per_unit_match.group(1).strip()
            # Extract the count if present (e.g., "メンバー1人")
            count_match = re.search(r'(\d+)(?:人|枚|つ)', per_unit_text)
            if count_match:
                per_unit_count = int(count_match.group(1))
            else:
                per_unit_count = 1
            # Extract the unit type (e.g., "メンバー")
            per_unit_type = None
            if 'メンバー' in per_unit_text:
                per_unit_type = 'member'
            elif 'カード' in per_unit_text:
                per_unit_type = 'card'
            # Store per_unit info to be set later
            per_unit_info = {
                'per_unit': True,
                'per_unit_count': per_unit_count,
            }
            if per_unit_type:
                per_unit_info['per_unit_type'] = per_unit_type
            # Infer action from text
            if 'ブレードを得る' in text or '選んだブレード' in text:
                per_unit_info['action'] = 'gain_resource'
                per_unit_info['resource'] = 'blade'
                # Extract resource icon count
                icon_count = text.count('{{icon_blade.png|ブレード}}')
                if icon_count > 0:
                    per_unit_info['count'] = icon_count
                # Set duration if present
                if 'ライブ終了時まで' in text:
                    per_unit_info['duration'] = 'live_end'
            elif 'ハートを得る' in text or '選んだハート' in text:
                per_unit_info['action'] = 'gain_resource'
                per_unit_info['resource'] = 'heart'
                # Set duration if present
                if 'ライブ終了時まで' in text:
                    per_unit_info['duration'] = 'live_end'
            elif '引く' in text:
                per_unit_info['action'] = 'draw_card'
                # Set duration if present
                if 'ライブ終了時まで' in text:
                    per_unit_info['duration'] = 'live_end'
            # Strip the per-unit pattern from the text
            text = text.replace(per_unit_match.group(0), '').strip()
    
    # Strip parenthetical notes first (before any other processing)
    text = strip_parenthetical(text)
    
    # Strip duration prefixes
    duration_prefixes = ['ライブ終了時まで、', 'このターンの間、', 'このライブの間、']
    for prefix in duration_prefixes:
        if text.startswith(prefix):
            action = {
                'text': text,
                'duration': 'live_end',
            }
            text = text[len(prefix):].strip()
            break
    else:
        action = {
            'text': text,
        }
    
    # Apply per_unit_info if it was set
    if 'per_unit_info' in locals():
        action.update(per_unit_info)
    
    # Extract per-unit scaling - check for "～につき" pattern
    per_unit_match = re.search(r'(.+?)につき', text)
    if per_unit_match:
        # Don't set per_unit as string - Rust expects boolean
        # action['per_unit'] = per_unit_match.group(1).strip()
        action['per_unit'] = True
        # Also extract location if present in per_unit context
        if '成功ライブカード置き場にある' in text:
            action['per_unit_location'] = 'success_live_card_zone'
        # Infer action from text
        if 'ブレードを得る' in text or '選んだブレード' in text:
            action['action'] = 'gain_resource'
            action['resource'] = 'blade'
            # Extract resource icon count
            icon_count = text.count('{{icon_blade.png|ブレード}}')
            if icon_count > 0:
                action['count'] = icon_count
        elif 'ハートを得る' in text or '選んだハート' in text:
            action['action'] = 'gain_resource'
            action['resource'] = 'heart'
        elif '引く' in text:
            action['action'] = 'draw_card'
        # Set duration if present
        if 'ライブ終了時まで' in text:
            action['duration'] = 'live_end'
    # Extract per-unit for specific actions (cost, required_hearts, etc.)
    if '枚につき' in text and ('コスト' in text or '必要ハート' in text):
        # Don't set per_unit as string - Rust expects boolean
        # action['per_unit'] = re.search(r'([^枚]+)枚につき', text).group(1).strip() if re.search(r'([^枚]+)枚につき', text) else 'card'
        action['per_unit'] = True
    
    # Extract source - handle "手札を" pattern for discard
    if '手札を' in text and '控え室に置く' in text:
        action['source'] = 'hand'
    # Handle source description patterns: "この[X]で控え室に置かれた[Y]"
    # Extract the description of which cards from the source are being targeted
    source_desc_match = re.search(r'この([^で]+)で控え室に置かれた', text)
    if source_desc_match:
        action['source'] = 'discard'
    # Check for under_member source (e.g., "下に置かれているエネルギーカード")
    if '下に置かれているエネルギーカード' in text:
        action['source'] = 'under_member'
        action['card_type'] = 'energy_card'
    
    # Extract source
    source = extract_source(text)
    if source:
        action['source'] = source
    
    # Extract destination
    destination = extract_destination(text)
    if destination:
        action['destination'] = destination
    # Check for destination choice (e.g., "好きなエリアに移動させる")
    if '好きなエリア' in text:
        action['destination_choice'] = True
    # Check for "好きな順番で" (in any order) placement
    if '好きな順番で' in text:
        action['placement_order'] = 'any_order'
    
    # Extract deck position (Q226: 一番上から4枚目)
    deck_position = extract_deck_position_for_action(text)
    if deck_position:
        action.update(deck_position)
    
    # Extract cost limit specifically for move_cards actions
    cost_limit = extract_cost_limit(text)
    if cost_limit:
        action['cost_limit'] = cost_limit
    
    # Extract state change
    state_change = extract_state_change(text)
    if state_change:
        action['state_change'] = state_change
    
    # Extract count
    count = extract_count(text)
    if count:
        action['count'] = count
    # Always count resource icons for blade/heart/energy (don't rely only on numeric count)
    icon_counts = {
        '{{icon_blade.png|ブレード}}': text.count('{{icon_blade.png|ブレード}}'),
        '{{icon_all.png|ハート}}': text.count('{{icon_all.png|ハート}}') + text.count('{{heart_'),
        '{{icon_energy.png|E}}': text.count('{{icon_energy.png|E}}')
    }
    for icon, icon_count in icon_counts.items():
        if icon_count > 0:
            # Always set count from icon counts for gain_resource effects
            if action.get('action') == 'gain_resource' or 'count' not in action:
                action['count'] = icon_count
            break
    
    # Extract card type
    card_type = extract_card_type(text)
    if card_type:
        action['card_type'] = card_type
    
    # Extract target
    target = extract_target(text)
    if target:
        action['target'] = target
    
    # Extract exclude_self for actions (e.g., "このメンバー以外の" or "「character name」以外")
    if 'このメンバー以外' in text or 'ほかのメンバー' in text:
        action['exclude_self'] = True
    # Also check for specific character name exclusions like "「鬼塚冬毬」以外"
    if re.search(r'「.+」以外', text):
        action['exclude_self'] = True
        # Extract the quoted character names being excluded
        quoted_exclusions = extract_quoted_text(text)
        if quoted_exclusions:
            categorized = categorize_quoted_text(quoted_exclusions)
            if categorized['abilities']:
                action['except_abilities'] = categorized['abilities']
            if categorized['characters']:
                action['except_characters'] = categorized['characters']
    
    # Extract group
    group = extract_group(text)
    if group:
        action['group'] = group
    
    # Extract group names from 『』 brackets
    group_names = extract_group_names(text)
    if group_names:
        action['group_names'] = group_names
    
    # Extract cost limit
    cost_limit = extract_cost_limit(text)
    if cost_limit:
        action['cost_limit'] = cost_limit
    
    # Check for ability gain pattern - MUST BE CHECKED BEFORE general quoted text extraction
    # Pattern 1: Explicit "能力を得る" (gain ability)
    # Pattern 2: "～を得る" where quoted text contains icon syntax (indicates ability text)
    quoted_text = extract_quoted_text(text)
    is_ability_gain = False
    if 'を得る' in text and '能力' in text:
        is_ability_gain = True
    elif 'を得る' in text and quoted_text:
        categorized = categorize_quoted_text(quoted_text)
        if categorized['abilities']:
            is_ability_gain = True
    
    if is_ability_gain:
        action['action'] = 'gain_ability'
        if quoted_text:
            categorized = categorize_quoted_text(quoted_text)
            if categorized['characters']:
                # These are likely character names or card names
                # Convert to QuotedText struct format (text, quoted_type)
                # Only set if single character - Rust expects single QuotedText, not array
                if len(categorized['characters']) == 1:
                    action['quoted_text'] = {
                        'text': categorized['characters'][0],
                        'quoted_type': 'character'
                    }
                # For multiple characters, don't set quoted_text to avoid deserialization errors
                # action['gained_ability'] = {'text': ability_text}
            elif categorized['characters'] and len(categorized['characters']) > 0:
                # This is a character name
                action['ability_source'] = categorized['characters'][0]
    else:
        # Extract quoted text from 「」 for other contexts
        quoted_text = extract_quoted_text(text)
        if quoted_text:
            categorized = categorize_quoted_text(quoted_text)
            if categorized['abilities']:
                # These contain icon syntax, likely ability references
                action['ability_references'] = categorized['abilities']
            if categorized['characters']:
                # These are likely character names or card names
                # Only set quoted_text for single character - Rust expects QuotedText struct, not array
                if len(categorized['characters']) == 1:
                    action['quoted_text'] = {
                        'text': categorized['characters'][0],
                        'quoted_type': 'character'
                    }
                # For multiple characters, don't set quoted_text to avoid deserialization errors
    
    # Extract cost limit
    cost_limit = extract_cost_limit(text)
    if cost_limit:
        action['cost_limit'] = cost_limit
    
    # Extract position
    position = extract_position(text)
    if position:
        if isinstance(position, dict):
            action.update(position)
        else:
            # Output as PositionInfo struct format
            action['position'] = {
                'position': position
            }
    
    # Extract optional flag
    if extract_optional(text):
        action['optional'] = True
    
    # Extract max flag
    if extract_max(text):
        action['max'] = True
    
    # Extract effect constraints
    constraint_patterns = {
        '未満にはならない': ('minimum_value', r'(\d+)未満にはならない'),
        '以上にはならない': ('maximum_value', r'(\d+)以上にはならない')
    }
    for keyword, (constraint_type, pattern) in constraint_patterns.items():
        if keyword in text:
            constraint_match = re.search(pattern, text)
            if constraint_match:
                action['constraint'] = {'type': constraint_type, 'value': int(constraint_match.group(1))}
            break
    
    # Check for shuffle action (5.5. シャッフルする)
    if 'シャッフルする' in text or 'シャッフルして' in text:
        action['action'] = 'shuffle'
        # Extract target location to shuffle
        shuffle_targets = {
            'デッキ': 'deck',
            'エネルギーデッキ': 'energy_deck'
        }
        for keyword, target in shuffle_targets.items():
            if keyword in text:
                action['target'] = target
                break
        # Extract count if specified (though shuffle typically applies to entire location)
        if count:
            action['count'] = count
    # Check for swap/exchange action (5.8. 入れ替える)
    elif '入れ替える' in text or '入れ替えて' in text:
        action['action'] = 'swap'
        # Extract the two items being swapped
        swap_match = re.search(r'(.+?)と(.+?)を入れ替える', text)
        if swap_match:
            action['item1'] = swap_match.group(1).strip()
            action['item2'] = swap_match.group(2).strip()
        # Extract locations if specified
        if source:
            action['source'] = source
        if destination:
            action['destination'] = destination
    # Check for pay energy action (5.9. を支払う)
    elif '{{icon_energy.png|E}}' in text and ('支払う' in text or 'コスト' in text):
        action['energy'] = text.count('{{icon_energy.png|E}}')
        # Check for per-trigger payment pattern
        if 'たび' in text and '支払ってもよい' in text:
            action['action'] = 'pay_energy_per_trigger'
            # Extract trigger event
            trigger_match = re.search(r'([^たび]+)たび', text)
            if trigger_match:
                action['trigger_event'] = trigger_match.group(1).strip()
        else:
            action['action'] = 'pay_energy'
        # Extract target (self/opponent)
        action['target'] = target if target else 'self'
    # Check for place energy under member action (5.10. エネルギーをメンバーの下に置く)
    elif destination == 'under_member' and ('エネルギー' in text or 'energy_card' in text):
        action['action'] = 'place_energy_under_member'
        # Extract energy count if specified
        if count:
            action['energy_count'] = count
        else:
            action['energy_count'] = 1  # Default to 1
        # Extract target member if specified
        if 'このメンバー' in text:
            action['target_member'] = 'this_member'
        else:
            # Extract quoted text for target member
            quoted = extract_quoted_text(text)
            if quoted:
                # Only use quoted text that doesn't contain icon syntax (character name, not ability)
                for q in quoted:
                    if '{{' not in q or '}}' not in q:
                        action['target_member'] = q
                        break
    # Check for draw action
    elif '引く' in text:
        # Check for draw until count pattern FIRST
        if '枚になるまで' in text:
            action['action'] = 'draw_until_count'
            action['source'] = 'deck'
            action['destination'] = 'hand'
            # Extract target count
            count_match = re.search(r'(\d+)枚になるまで', text)
            if count_match:
                action['target_count'] = int(count_match.group(1))
        else:
            action['action'] = 'draw_card'
            action['source'] = 'deck'
            action['destination'] = 'hand'
        # Check for source specification
        if 'デッキの上から' in text:
            action['source'] = 'deck_top'
    # Determine action type
    elif state_change:
        action['action'] = 'change_state'
    elif 'アクティブにしてもよい' in text or 'アクティブにする' in text:
        action['action'] = 'change_state'
        action['state_change'] = 'active'
        if 'してもよい' in text:
            action['optional'] = True
    # Check for activation restriction patterns
    elif 'のみ起動できる' in text or 'のみ発動する' in text:
        action['action'] = 'activation_restriction'
        action['restriction_type'] = 'only'
    elif '支払って発動させる' in text:
        action['action'] = 'activation_cost'
    # Check for restriction patterns
    elif 'ライブできない' in text:
        action['action'] = 'restriction'
        action['restriction_type'] = 'cannot_live'
    elif 'アクティブにしない' in text:
        action['action'] = 'restriction'
        action['restriction_type'] = 'cannot_activate'
    elif 'バトンタッチで控え室に置けない' in text:
        action['action'] = 'restriction'
        action['restriction_type'] = 'cannot_baton_touch'
    # Check for general "できない" (cannot) restriction patterns
    # Must check BEFORE destination-based move_cards to avoid false positives
    elif '置くことができない' in text:
        action['action'] = 'restriction'
        action['restriction_type'] = 'cannot_place'
        # Extract destination if present for context
        if 'destination' in action:
            action['restricted_destination'] = action['destination']
    elif '置けない' in text:
        action['action'] = 'restriction'
        action['restriction_type'] = 'cannot_place'
        # Extract destination if present for context
        if 'destination' in action:
            action['restricted_destination'] = action['destination']
    elif '登場できない' in text:
        action['action'] = 'restriction'
        action['restriction_type'] = 'cannot_appear'
    elif '移動できない' in text:
        action['action'] = 'restriction'
        action['restriction_type'] = 'cannot_move'
    elif 'source' in action and 'destination' in action and action['source'] and action['destination']:
        action['action'] = 'move_cards'
    # Check for "加える" (add to) pattern - common for adding cards to hand
    elif '加える' in text or '加え' in text:
        action['action'] = 'move_cards'
        if 'destination' not in action:
            action['destination'] = 'hand'
    elif 'destination' in action:
        action['action'] = 'move_cards'
    elif 'ポジションチェンジ' in text:
        action['action'] = 'position_change'
    elif '移動させる' in text:
        # "移動させる" is an action, but context determines if it's position_change or move_cards
        # If it's about moving between areas (エリアに移動させる), it's position_change
        if 'エリア' in text:
            action['action'] = 'position_change'
        else:
            action['action'] = 'move_cards'
    # Check for "置く" (place) pattern
    elif '置く' in text or '置いて' in text:
        # Check for draw until count pattern (check this BEFORE discard pattern)
        if '枚になるまで' in text and '引く' in text:
            action['action'] = 'draw_until_count'
            # Extract target count
            count_match = re.search(r'(\d+)枚になるまで', text)
            if count_match:
                action['target_count'] = int(count_match.group(1))
        # Check for discard until count pattern
        elif '枚になるまで' in text and '控え室に置く' in text:
            action['action'] = 'discard_until_count'
            # Extract target count
            count_match = re.search(r'(\d+)枚になるまで', text)
            if count_match:
                action['target_count'] = int(count_match.group(1))
            # Extract card type if present
            if '手札' in text:
                action['source'] = 'hand'
            elif 'ステージ' in text:
                action['source'] = 'stage'
        else:
            action['action'] = 'move_cards'
            # If destination not already set, infer it from context
            if 'destination' not in action:
                destination = None
                if '手札に加える' in text:
                    destination = 'hand'
                elif '控え室に置く' in text or '控え室に送る' in text:
                    destination = 'discard'
                elif 'ステージに置く' in text or '登場させる' in text:
                    destination = 'stage'
                elif 'エネルギー置き場に置く' in text:
                    destination = 'energy_zone'
                elif 'ライブカード置き場に置く' in text:
                    destination = 'live_card_zone'
                elif '成功ライブカード置き場に置く' in text:
                    destination = 'success_live_card_zone'
                elif 'デッキの上に置く' in text:
                    destination = 'deck_top'
                elif 'デッキの下に置く' in text:
                    destination = 'deck_bottom'
                elif 'デッキに戻す' in text or 'デッキに置く' in text or 'デッキの一番上から' in text:
                    destination = 'deck'
                    # Check for deck position (Q226: 一番上から4枚目)
                    deck_position = extract_deck_position_for_action(text)
                    if deck_position:
                        action.update(deck_position)
                elif '山札の下に置く' in text:
                    destination = 'deck_bottom'
                elif '山札の上に置く' in text:
                    destination = 'deck_top'
                if destination:
                    action['destination'] = destination
    # Check for destination-only moves (inferred source)
    elif destination:
        action['action'] = 'move_cards'
    elif '引く' in text or '引き' in text:
        action['action'] = 'draw'
        action['source'] = 'deck'
        action['destination'] = 'hand'
    elif 'デッキの上に置き' in text or 'デッキの上に置く' in text:
        action['action'] = 'move_cards'
        action['destination'] = 'deck_top'
        if '好きな順番で' in text:
            action['placement_order'] = 'any_order'
    elif '見る' in text:
        action['action'] = 'look_at'
        # Enhanced look_at parsing (5.7. 上から見る)
        if 'デッキの上から' in text:
            action['source'] = 'deck_top'
            # Extract count for "上から（数値）枚見る"
            look_count_match = re.search(r'上から(\d+)枚', text)
            if look_count_match:
                action['count'] = int(look_count_match.group(1))
            # Check for "まで" (up to) modifier
            if 'まで見る' in text:
                action['max'] = True
        elif 'デッキから' in text:
            action['source'] = 'deck'
    elif '選び' in text or '選ぶ' in text:
        action['action'] = 'select'
        # Check for distinct condition
        if 'カード名の異なる' in text or '名前の異なる' in text:
            action['distinct'] = 'card_name'
        # Extract source
        if '控え室にある' in text:
            action['source'] = 'discard'
        elif '手札から' in text:
            action['source'] = 'hand'
        elif 'デッキから' in text:
            action['source'] = 'deck'
        elif 'ステージから' in text:
            action['source'] = 'stage'
    # Check for appearance action pattern (ステージに登場させてもよい / 登場させる)
    # Check this early to catch it before other patterns
    elif 'ステージに登場させてもよい' in text or 'ステージに登場させる' in text or '登場させる' in text:
        action['action'] = 'appear'
        action['destination'] = 'stage'
        # Extract source
        if '手札から' in text:
            action['source'] = 'hand'
        elif '控え室から' in text:
            action['source'] = 'discard'
        # Extract count
        count_match = re.search(r'(\d+)枚', text)
        if count_match:
            action['count'] = int(count_match.group(1))
        # Extract card type
        card_type = extract_card_type(text)
        if card_type:
            action['card_type'] = card_type
        # Check for optional
        if 'もよい' in text or 'てもよい' in text:
            action['optional'] = True
    # Check for move_cards with destination_choice (好きなエリアに移動させる)
    elif '好きなエリアに移動させ' in text:
        action['action'] = 'move_cards'
        action['source'] = 'stage'
        action['destination'] = 'stage'
        action['destination_choice'] = True
        # Extract card type
        card_type = extract_card_type(text)
        if card_type:
            action['card_type'] = card_type
        # Check for optional
        if 'もよい' in text or 'てもよい' in text:
            action['optional'] = True
    # Check for discard_until_count pattern (手札の枚数がX枚になるまで手札を控え室に置き)
    # Check this early before other patterns that might match parts of the text
    elif '手札の枚数が' in text and '枚になるまで手札を控え室に置' in text:
        action['action'] = 'discard_until_count'
        target_count_match = re.search(r'手札の枚数が(\d+)枚になるまで', text)
        if target_count_match:
            action['target_count'] = int(target_count_match.group(1))
        action['source'] = 'hand'
        action['destination'] = 'discard'
    # Check for energy payment pattern (E支払ってもよい)
    elif '{{icon_energy.png|E}}支払ってもよい' in text or '{{icon_energy.png|E}}支払ってもよい' in text:
        action['action'] = 'pay_energy'
        action['type'] = 'pay_energy'
        # Count energy icons
        energy_count = text.count('{{icon_energy.png|E}}')
        action['energy'] = energy_count
        if 'もよい' in text or 'てもよい' in text:
            action['optional'] = True
    elif 'デッキの上に置き' in text or 'デッキの上に置く' in text:
        action['action'] = 'move_cards'
        action['destination'] = 'deck_top'
        if '好きな順番で' in text:
            action['placement_order'] = 'any_order'
        if '好きな枚数' in text:
            action['count'] = 'variable'
    elif '公開する' in text or '公開し' in text:
        # Check for reveal per group pattern
        if '各グループ名につき1枚ずつ公開し' in text:
            action['action'] = 'reveal_per_group'
            # Don't set per_unit as string - Rust expects boolean
            # action['per_unit'] = '各グループ名'
            action['per_unit'] = True
            action['count'] = 1
        else:
            action['action'] = 'reveal'
            # Extract count if present (e.g., "1枚ずつ")
            count_match = re.search(r'(\d+)枚ずつ', text)
            if count_match:
                action['count'] = int(count_match.group(1))
            # Check for variable count (好きな枚数)
            if '好きな枚数' in text:
                action['count'] = 'variable'
    # Check for discard_until_count pattern (手札の枚数がX枚になるまで手札を控え室に置き)
    elif '手札の枚数が' in text and '枚になるまで手札を控え室に置' in text:
        action['action'] = 'discard_until_count'
        target_count_match = re.search(r'手札の枚数が(\d+)枚になるまで', text)
        if target_count_match:
            action['target_count'] = int(target_count_match.group(1))
        action['source'] = 'hand'
        action['destination'] = 'discard'
    # Check for cost modification pattern (コストを＋Xする)
    elif 'コストを' in text and ('＋' in text or '＋' in text or '+' in text) and 'する' in text:
        action['action'] = 'modify_cost'
        # Extract operation
        if '＋' in text or '+' in text:
            action['operation'] = 'add'
        elif '－' in text or '-' in text:
            action['operation'] = 'subtract'
        # Extract value - try both full-width and half-width plus
        value_match = re.search(r'([＋+])(\d+)', text)
        if value_match:
            action['value'] = int(value_match.group(2))
    elif 'エールによって公開される自分のカードの枚数が' in text:
        action['action'] = 'modify_yell_count'
        # Extract the count change (e.g., "8枚減る" or "2枚増える")
        if '減る' in text or '減らす' in text:
            action['operation'] = 'subtract'
        elif '増える' in text or '増やす' in text:
            action['operation'] = 'add'
        # Extract the count
        count_match = re.search(r'(\d+)枚', text)
        if count_match:
            action['count'] = int(count_match.group(1))
    elif '得る' in text:
        # Check for "好きな順番で" (in any order) placement
        if '好きな順番で' in text:
            action['placement_order'] = 'any_order'
        # Check for character-specific resource mapping: "「X」はYを得る"
        character_mapping_match = re.search(r'「([^」]+)」は(.+)を得る', text)
        if character_mapping_match:
            # Distinguish character names from ability names
            # Character names are short and don't contain icons
            potential_character = character_mapping_match.group(1)
            if '{{' not in potential_character and len(potential_character) <= MAX_CHARACTER_NAME_LENGTH:
                action['action'] = 'character_resource_mapping'
                action['character'] = potential_character
                action['resource_text'] = character_mapping_match.group(2)
            else:
                # It's an ability gain pattern, not a character resource mapping
                action['action'] = 'gain_ability'
                action['ability'] = potential_character
        else:
            action['action'] = 'gain_resource'
            if 'ハート' in text:
                action['resource'] = 'heart'
            elif 'ブレード' in text:
                action['resource'] = 'blade'
            # Remove count field since it's derived from resource icon count
            if 'count' in action:
                del action['count']
    elif '選ぶ' in text and ('heart_' in text or 'ハート' in text):
        action['action'] = 'choose_heart'
        # Extract the heart options
        heart_options = re.findall(r'{{heart_(\d+)\.png\|heart\d+}}', text)
        if heart_options:
            action['options'] = [f'heart_{h}' for h in heart_options]
    # Check for choose_required_hearts pattern (e.g., "必要ハートは、...のうち、選んだ1つにしてもよい")
    elif '必要ハートは' in text and '選んだ1つにしてもよい' in text:
        action['action'] = 'choose_required_hearts'
        # Extract the heart option groups
        heart_groups = re.findall(r'{{heart_\d+\.png\|heart\d+}}{{heart_\d+\.png\|heart\d+}}{{heart_\d+\.png\|heart\d+}}', text)
        if heart_groups:
            action['options'] = heart_groups
        if 'してもよい' in text:
            action['optional'] = True
    # Check for set_card_identity pattern (e.g., "すべての領域にあるこのカードは『...』として扱う")
    elif 'として扱う' in text and 'すべての領域にあるこのカードは' in text:
        action['action'] = 'set_card_identity'
        # Extract group names
        groups = re.findall(r'『([^』]+)』', text)
        if groups:
            action['identities'] = groups
    elif 'スコアを' in text:
        action['action'] = 'modify_score'
        if '＋' in text:
            action['operation'] = 'add'
            # Extract the value after +
            value_match = re.search(r'＋(\d+)', text)
            if value_match:
                action['value'] = int(value_match.group(1))
        elif '－' in text:
            action['operation'] = 'subtract'
            # Extract the value after -
            value_match = re.search(r'－(\d+)', text)
            if value_match:
                action['value'] = int(value_match.group(1))
    elif '必要ハート' in text and '少なくなる' in text:
        action['action'] = 'modify_required_hearts'
        action['operation'] = 'decrease'
    elif '必要ハート' in text and '増える' in text:
        action['action'] = 'modify_required_hearts'
        action['operation'] = 'increase'
    elif '必要ハート' in text and '減らす' in text:
        action['action'] = 'modify_required_hearts'
        action['operation'] = 'decrease'
    # Check for set_score pattern (e.g., "スコアは4になる")
    elif 'スコアは' in text and 'なる' in text:
        action['action'] = 'set_score'
        score_match = re.search(r'スコアは(\d+)になる', text)
        if score_match:
            action['value'] = int(score_match.group(1))
    # Check for set_cost pattern (e.g., "コストは...になる")
    elif 'コストは' in text and 'なる' in text:
        action['action'] = 'set_cost'
        # Extract heart icons to calculate cost
        heart_icons = re.findall(r'{{heart_\d+\.png\|heart\d+}}', text)
        if heart_icons:
            action['value'] = len(heart_icons)
    # Check for set_required_hearts pattern (e.g., "必要ハートは...になる")
    elif '必要ハートは' in text and 'なる' in text:
        action['action'] = 'set_required_hearts'
        heart_icons = re.findall(r'{{heart_\d+\.png\|heart\d+}}', text)
        if heart_icons:
            action['value'] = len(heart_icons)
    # Check for set_blade_type pattern (e.g., "ブレードはすべて[青ブレード]になる")
    elif 'ブレード' in text and 'なる' in text and 'すべて' in text:
        action['action'] = 'set_blade_type'
        # Extract the blade type from brackets
        blade_match = re.search(r'\[([^\]]+)ブレード\]になる', text)
        if blade_match:
            action['blade_type'] = blade_match.group(1)
    # Check for set_heart_type pattern (e.g., "ハートをすべて{{heart_01.png|heart01}}にする")
    elif 'ハート' in text and 'にする' in text and 'すべて' in text:
        action['action'] = 'set_heart_type'
        heart_match = re.search(r'{{heart_(\d+)\.png\|heart\d+}}にする', text)
        if heart_match:
            action['heart_type'] = f'heart_{heart_match.group(1)}'
    # Check for set_blade_count pattern (e.g., "ブレードの数は3つになる")
    elif 'ブレードの数は' in text and 'つになる' in text:
        action['action'] = 'set_blade_count'
        count_match = re.search(r'ブレードの数は(\d+)つになる', text)
        if count_match:
            action['value'] = int(count_match.group(1))
    elif 'コスト' in text and ('減る' in text or '少なくなる' in text):
        action['action'] = 'modify_cost'
        action['operation'] = 'decrease'
    elif 'コスト' in text and '増える' in text:
        action['action'] = 'modify_cost'
        action['operation'] = 'increase'
    elif 'ポジションチェンジ' in text:
        action['action'] = 'position_change'
        # Extract swap logic
        if '入れ替える' in text or '入れ替えて' in text:
            action['swap'] = True
        # Extract optionality
        if 'してもよい' in text:
            action['optional'] = True
        # Extract group-based destination criteria
        if '同じグループの' in text:
            action['destination_criteria'] = 'same_group'
        elif '別のグループの' in text:
            action['destination_criteria'] = 'different_group'
        # Extract destination area if specified
        if 'センターエリア' in text:
            action['destination_area'] = 'center'
        elif '左サイドエリア' in text or '左サイド' in text:
            action['destination_area'] = 'left_side'
        elif '右サイドエリア' in text or '右サイド' in text:
            action['destination_area'] = 'right_side'
    # Check for gain_ability via quoted text (even without explicit "能力" keyword) - check this BEFORE generic 'を得る' check
    elif quoted_text and any('ライブ' in q or 'スコア' in q or 'ブレード' in q or 'ハート' in q for q in quoted_text):
        action['action'] = 'gain_ability'
        action['ability'] = quoted_text
    elif 'を得る' in text and '能力' in text:
        action['action'] = 'gain_ability'
        # Extract ability name from quoted_text if available
        if quoted_text:
            action['ability'] = quoted_text
        else:
            ability_match = re.search(r'「([^」]+)」を得る', text)
            if ability_match:
                action['ability'] = [ability_match.group(1)]
    elif '枚数が' in text and ('減る' in text or '増える' in text):
        action['action'] = 'modify_reveal_count'
        if '減る' in text:
            action['operation'] = 'decrease'
        elif '増える' in text:
            action['operation'] = 'increase'
    elif '枚数の上限' in text and ('減る' in text or '増える' in text):
        action['action'] = 'modify_limit'
        if '減る' in text:
            action['operation'] = 'decrease'
        elif '増える' in text:
            action['operation'] = 'increase'
    elif '能力を無効にする' in text or '能力を無効にしてもよい' in text:
        action['action'] = 'invalidate_ability'
        if 'してもよい' in text:
            action['optional'] = True
    else:
        action['action'] = 'custom'
    
    return action

# ============== MAIN PARSING FUNCTIONS ==============

def parse_cost(text: str) -> Dict[str, Any]:
    """Parse a cost text."""
    cost = {
        'text': text,
    }
    
    # Check for sequential cost (～し、～ or ～し、～置いてもよい)
    if '、' in text:
        parts = text.split('、')
        if len(parts) >= 2:
            # Check if first part ends with 'し' (te-form)
            if parts[0].strip().endswith('し'):
                # Parse each sequential cost part
                cost_parts = []
                for i, part in enumerate(parts):
                    # Add 'し' back to first part for context if it was stripped
                    if i == 0 and not part.strip().endswith('し'):
                        part = part.strip() + 'し'
                    cost_part = parse_cost(part.strip())
                    cost_parts.append(cost_part)
                return {
                    'text': text,
                    'cost_type': 'sequential_cost',
                    'costs': cost_parts
                }
    
    # Check for reveal action (公開する/公開し)
    if '公開する' in text or '公開し' in text:
        cost['cost_type'] = 'reveal'
        cost['action'] = 'reveal'
        # Extract source if present
        if '手札' in text:
            cost['source'] = 'hand'
        # Extract count if present
        count_match = re.search(r'(\d+)枚', text)
        if count_match:
            cost['count'] = int(count_match.group(1))
        # Extract card type if present
        card_type = extract_card_type(text)
        if card_type:
            cost['card_type'] = card_type
        return cost
    
    # Check for choice cost (～か、～)
    if 'か、' in text:
        parts = text.split('か、', SPLIT_LIMIT)
        if len(parts) == 2:
            # Parse each option as a separate cost
            option1 = parse_cost(parts[0].strip())
            option2 = parse_cost(parts[1].strip())
            return {
                'text': text,
                'type': 'choice_condition',
                'options': [option1, option2]
            }
    
    # Check for activation condition (～場合のみ起動できる)
    if '起動できる' in text and '場合' in text:
        activation_match = re.search(r'([^場合]+)場合', text)
        if activation_match:
            cost['activation_condition'] = activation_match.group(1).strip()
    
    # Extract source - handle "手札を" and "手札の" patterns
    if '手札を' in text:
        cost['source'] = 'hand'
    elif '手札の' in text:
        cost['source'] = 'hand'
    
    source = extract_source(text)
    if source and 'source' not in cost:
        cost['source'] = source
    
    # Special case: deck_bottom destination (check early to avoid custom fallback)
    if 'デッキの一番下に置く' in text or 'デッキの一番下に置いて' in text or 'デッキの下に置く' in text or 'デッキの下に置いて' in text or '山札の下に置く' in text or '山札の下に置いて' in text:
        cost['destination'] = 'deck_bottom'
        cost['cost_type'] = 'move_cards'
        cost['action'] = 'move_cards'
        if 'もよい' in text or 'てもよい' in text:
            cost['optional'] = True
        return cost
    
    # Extract destination
    destination = extract_destination(text)
    if destination:
        cost['destination'] = destination
    # Special case: energy_deck destination
    if 'エネルギーデッキに置く' in text:
        cost['destination'] = 'energy_deck'
    # If source is present but destination is not, infer destination
    elif 'source' in cost:
        if cost['source'] == 'hand' and '控え室に置く' in text:
            cost['destination'] = 'discard'
        elif cost['source'] == 'discard' and '手札に加える' in text:
            cost['destination'] = 'hand'
    
    # Extract state change
    state_change = extract_state_change(text)
    if state_change:
        cost['state_change'] = state_change
        cost['cost_type'] = 'state_change'  # Set type for wait/active costs
    
    # Check for reveal card pattern (公開してもよい)
    if '公開してもよい' in text:
        cost['cost_type'] = 'reveal_condition'
    
    # Check for energy cost (エネルギーをエネルギーデッキに置く)
    if 'エネルギー' in text and 'エネルギーデッキに置く' in text:
        cost['cost_type'] = 'energy_condition'
    
    # Extract count
    count = extract_count(text)
    if count:
        cost['count'] = count
    
    # Extract card type
    card_type = extract_card_type(text)
    if card_type:
        cost['card_type'] = card_type
    
    # Extract target
    target = extract_target(text)
    if target:
        cost['target'] = target
    
    # Extract exclude_self for costs (e.g., "このメンバー以外の" or "「character name」以外")
    if 'このメンバー以外' in text or 'ほかのメンバー' in text:
        cost['exclude_self'] = True
    # Also check for specific character name exclusions like "「鬼塚冬毬」以外"
    if re.search(r'「.+」以外', text):
        cost['exclude_self'] = True
        # Extract the quoted character names being excluded
        quoted_exclusions = extract_quoted_text(text)
        if quoted_exclusions:
            categorized = categorize_quoted_text(quoted_exclusions)
            if categorized['abilities']:
                cost['except_abilities'] = categorized['abilities']
            if categorized['characters']:
                cost['except_characters'] = categorized['characters']
    
    # Extract self_cost for costs (e.g., "このメンバーを" - the card itself is the cost)
    # This is distinct from exclude_self - here the card itself is being acted upon
    if 'このメンバー' in text and 'このメンバー以外' not in text and 'ほかのメンバー' not in text:
        # Check if it's the subject/object of the action (marked by を or が)
        # Common patterns: "このメンバーをステージから控え室に置く", "このメンバーをウェイトにする"
        if re.search(r'このメンバー[をが]', text):
            cost['self_cost'] = True
    
    # Extract group
    group = extract_group(text)
    if group:
        cost['group'] = group
    
    # Extract optional flag
    if extract_optional(text):
        cost['optional'] = True
    
    # Extract max flag
    if extract_max(text):
        cost['max'] = True
    
    # Extract dynamic cost (cost depends on card score or other value)
    if 'に等しい数の' in text and '支払う' in text:
        cost['dynamic'] = True
        # Try to extract the source of the dynamic cost
        if 'スコアに等しい' in text:
            cost['dynamic_source'] = 'card_score'
        elif 'コストに等しい' in text:
            cost['dynamic_source'] = 'card_cost'
    
    # Extract cost reduction (e.g., "グループ名1種類につき、E減る")
    if '減る' in text and 'コスト' in text:
        reduction_match = re.search(r'(\d+)種類につき.*?(\d+)減る', text)
        if reduction_match:
            cost['cost_reduction'] = {
                'per_unit_type': 'group_variety',
                'per_unit_count': int(reduction_match.group(1)),
                'reduction_amount': int(reduction_match.group(2))
            }
        else:
            # Try simpler pattern: "X減る"
            simple_match = re.search(r'(\d+)減る', text)
            if simple_match:
                cost['cost_reduction'] = {
                    'reduction_amount': int(simple_match.group(1))
                }
    
    # Determine cost type - check energy first
    if 'エネルギー' in text and 'エネルギーデッキに置く' in text:
        cost['cost_type'] = 'energy_condition'
    elif '公開してもよい' in text:
        cost['cost_type'] = 'reveal_condition'
    elif cost.get('source') and cost.get('destination'):
        cost['cost_type'] = 'move_cards'
    elif 'ウェイトにする' in text or 'ウェイト状態で置く' in text or 'ウェイト状態で登場させる' in text or 'アクティブにする' in text:
        cost['cost_type'] = 'change_state'
    elif state_change:
        cost['cost_type'] = 'change_state'
    elif '公開する' in text:
        cost['cost_type'] = 'reveal_condition'
    elif '{{icon_energy.png|E}}' in text:
        cost['cost_type'] = 'pay_energy'
        cost['energy'] = text.count('{{icon_energy.png|E}}')
    elif cost.get('source'):
        # If source but no destination, infer based on common patterns
        if cost['source'] == 'hand' and ('控え室に置く' in text or '控え室に置いて' in text):
            cost['destination'] = 'discard'
            cost['cost_type'] = 'move_cards'
        elif cost['source'] == 'discard' and '手札に加える' in text:
            cost['destination'] = 'hand'
            cost['cost_type'] = 'move_cards'
        else:
            cost['cost_type'] = 'custom'
    else:
        cost['cost_type'] = 'custom'
    
    return cost

def parse_effect(text: str) -> Dict[str, Any]:
    """Parse an effect text."""
    text = normalize_fullwidth_digits(text).strip()
    text = strip_suffix_period(text)
    
    # Check for per-unit scaling (check this FIRST before other parsing splits the text)
    # Exclude '各グループ名につき' pattern as it's handled differently
    if ('につき' in text or 'ごとに' in text) and '各グループ名につき' not in text and 'グループ名につき' not in text:
        per_unit_match = re.search(r'([^、。]+)(につき|ごとに)', text)
        if per_unit_match:
            per_unit_text = per_unit_match.group(1).strip()
            effect = {
                'text': text,
                # Don't set per_unit as string - Rust expects boolean
                # 'per_unit': per_unit_match.group(1).strip()
                'per_unit': True
            }
            # Extract per_unit_count if present (e.g., "1人につき")
            per_unit_count_match = re.search(r'(\d+)(人|枚|つ)(につき|ごとに)', text)
            if per_unit_count_match:
                effect['per_unit_count'] = int(per_unit_count_match.group(1))
                effect['per_unit_type'] = per_unit_count_match.group(2)
            else:
                # Try to infer the type from the per_unit text
                if 'メンバー' in per_unit_text or '人' in per_unit_text:
                    effect['per_unit_type'] = 'member'
                elif 'カード' in per_unit_text or '枚' in per_unit_text:
                    effect['per_unit_type'] = 'card'
                elif 'ブレード' in per_unit_text:
                    effect['per_unit_type'] = 'blade'
                else:
                    effect['per_unit_type'] = 'unknown'
            return effect
    
    # Initialize effect dict
    effect = {
        'text': text
    }
    
    # Check for activation conditions at end of text (～場合のみ起動できる)
    # Do this early to prevent incorrect sequential action splitting
    activation_match = re.search(r'この能力は、(.+?)場合のみ起動できる', text)
    if activation_match:
        activation_condition_text = 'この能力は、' + activation_match.group(1).strip() + '場合のみ起動できる'
        # Remove the activation condition from the main text
        action_text = text.replace(activation_condition_text, '').strip()
        # Handle case where there's a period before the activation condition
        if action_text.endswith('。'):
            action_text = action_text[:-1].strip()
        action = parse_action(action_text)
        effect = {
            'text': text,
            'activation_condition': activation_condition_text
        }
        effect.update(action)
        # Parse the activation condition
        activation_condition = parse_condition(activation_match.group(1).strip() + '場合')
        if activation_condition.get('type') != 'custom':
            effect['activation_condition_parsed'] = activation_condition
        return effect
    
    # Extract parenthetical notes BEFORE stripping them from text
    parenthetical = extract_parenthetical(text)
    if parenthetical:
        # Store parenthetical notes
        # Note: We'll add them to the effect dict after we create it
        pass
    
    effect = {
        'text': text,
    }
    
    # Check for per-unit scaling (e.g., "メンバー1人につき") - CHECK THIS FIRST before any text splitting
    # Exclude '各グループ名につき' pattern as it's handled differently
    if PER_UNIT_MARKER in text and '各グループ名につき' not in text and 'グループ名につき' not in text:
        # Extract the per-unit pattern
        per_unit_match = re.search(r'(.*?)につき', text)
        if per_unit_match:
            per_unit_text = per_unit_match.group(1).strip()
            # Don't set per_unit as string - Rust expects boolean
            # effect['per_unit'] = per_unit_text
            effect['per_unit'] = True
            # Extract the count if present (e.g., "メンバー1人")
            count_match = re.search(r'(\d+)(?:人|枚|つ)', per_unit_text)
            if count_match:
                effect['per_unit_count'] = int(count_match.group(1))
            # Extract the unit type (e.g., "メンバー")
            if 'メンバー' in per_unit_text:
                effect['per_unit_type'] = 'member'
            elif 'カード' in per_unit_text:
                effect['per_unit_type'] = 'card'
            # Infer action from text
            if 'ブレードを得る' in text or '選んだブレードを得る' in text:
                effect['action'] = 'gain_resource'
                effect['resource'] = 'blade'
                # Extract resource icon count
                icon_count = text.count('{{icon_blade.png|ブレード}}')
                if icon_count > 0:
                    effect['count'] = icon_count
            elif 'ハートを得る' in text or '選んだハートを得る' in text:
                effect['action'] = 'gain_resource'
                effect['resource'] = 'heart'
            elif '引く' in text:
                effect['action'] = 'draw_card'
            # Set duration if present
            if 'ライブ終了時まで' in text:
                effect['duration'] = 'live_end'
    
    # Add parenthetical activation notes to effect if they were extracted
    if parenthetical:
        effect['parenthetical'] = parenthetical
        # Check if parenthetical contains activation condition
        for note in parenthetical:
            if '起動できる' in note or '発動する' in note:
                effect['activation_condition'] = note
                # Extract position from activation condition if present
                if 'センターエリア' in note:
                    effect['activation_position'] = 'center'
                elif '左サイドエリア' in note or '左サイド' in note:
                    effect['activation_position'] = 'left_side'
                elif '右サイドエリア' in note or '右サイド' in note:
                    effect['activation_position'] = 'right_side'
                break
    
    # Strip parenthetical notes from text for further processing
    text = strip_parenthetical(text)
    
    # Check for each-time triggers (たび)
    if EACH_TIME_MARKER in text:
        effect['trigger_type'] = 'each_time'
        # Extract the trigger event
        trigger_match = re.search(r'([^たび]+)たび', text)
        if trigger_match:
            effect['trigger_event'] = trigger_match.group(1).strip()
    
    # Check for "自分か相手を選ぶ" (choose self or opponent) pattern
    if text.startswith('自分か相手を選ぶ。'):
        # Extract the choice part
        choice_text = '自分か相手を選ぶ'
        remaining_text = text[len(choice_text) + 1:].strip()  # Remove choice and period
        effect['target_choice'] = {
            'type': 'choice_condition',
            'options': ['self', 'opponent'],
            'text': choice_text
        }
        # Parse the remaining text as the main effect
        remaining_effect = parse_effect(remaining_text)
        effect.update(remaining_effect)
        return effect
    
    # Check for opponent choice/action patterns
    if text.startswith('相手は'):
        # Extract the opponent action part (include comma in marker)
        opponent_match = re.match(r'相手は、(.+?)。', text)
        if opponent_match:
            opponent_action_text = opponent_match.group(0)
            remaining_text = text[len(opponent_action_text):].strip()
            effect['action_by'] = 'opponent'
            # Parse the opponent action (no comma to strip now)
            opponent_action = parse_action(opponent_match.group(1).strip())
            effect['opponent_action'] = opponent_action
            # Parse remaining text if any
            if remaining_text:
                remaining_effect = parse_effect(remaining_text)
                effect.update(remaining_effect)
            return effect
    
    # Check for opponent actions after conditional markers (e.g., "そうした場合、相手は～")
    if '、相手は' in text:
        # Split by the opponent action marker (including comma)
        parts = text.split('、相手は、', SPLIT_LIMIT)
        if len(parts) == 2:
            first_part = parts[0].strip()
            opponent_part = '相手は、' + parts[1]
            # Extract the opponent action (comma already in marker)
            opponent_match = re.match(r'相手は、(.+?)。', opponent_part)
            if opponent_match:
                # Remove conditional marker from first part for parsing
                first_action_text = first_part.replace('そうした場合、', '').strip()
                # Parse as simple action to avoid nested sequential
                first_action = parse_action(first_action_text)
                # Extract any remaining text after opponent action
                remaining_text = opponent_part[len(opponent_match.group(0)):].strip()
                # Create sequential structure
                effect['action'] = 'sequential'
                effect['actions'] = [first_action]
                # Add opponent action with metadata (no comma to strip)
                opponent_action = parse_action(opponent_match.group(1).strip())
                opponent_action['action_by'] = 'opponent'
                effect['actions'].append(opponent_action)
                # Add remaining action if any
                if remaining_text:
                    remaining_effect = parse_action(remaining_text)
                    effect['actions'].append(remaining_effect)
                effect['conditional'] = True
                effect['text'] = text
                # Only return if actions array is not empty
                if effect['actions']:
                    return effect
    
    # Check for "その中から" (from among them) pattern - indicates look_at + select + action
    # Check this BEFORE comma-separated sequential to prevent incorrect parsing
    if 'その中から' in text:
        effect['action'] = 'look_and_select'
        # Extract the look part (before "その中から")
        look_match = re.search(r'(.+?)その中から', text)
        if look_match:
            effect['look_action'] = parse_action(look_match.group(1).strip())
        # Extract the action part (after "その中から")
        action_match = re.search(r'その中から(.+)', text)
        if action_match:
            select_text = action_match.group(1).strip()
            
            # Check for "X枚を手札に加え、残りを控え室に置く" pattern
            # This should be parsed as sequential actions
            if '手札に加え' in select_text and '残りを控え室に置く' in select_text:
                # Split into two sequential actions
                parts = select_text.split('、', SPLIT_LIMIT)
                # Skip quoted_text extraction for now to avoid errors

            # Check for "好きな枚数を好きな順番でデッキの上に置き" in select_text and '残りを控え室に置く' in select_text:
            # This should be parsed as sequential actions with deck_top destination
            if '好きな枚数を好きな順番でデッキの上に置き' in select_text and '残りを控え室に置く' in select_text:
                # Split into two sequential actions
                parts = select_text.split('、', SPLIT_LIMIT)
                if len(parts) == 2:
                    # First action: put cards on deck top
                    first_action = parse_action(parts[0].strip())
                    # Override destination to deck_top
                    if first_action.get('action') == 'move_cards':
                        first_action['destination'] = 'deck_top'
                        first_action['any_number'] = True  # "好きな枚数"
                    # Second action: discard rest
                    second_action = parse_action(parts[1].strip())
                    effect['select_action'] = {
                        'action': 'sequential',
                        'actions': [first_action, second_action],
                        'text': select_text
                    }
                    return effect
            
            # Default parsing
            effect['select_action'] = parse_action(select_text)
        return effect
    
    # Strip parenthetical notes for sequential action check
    text_without_parens = strip_parenthetical(text) if parenthetical else text
    
    # Check for sequential conditional effects with "さらに" pattern
    # Pattern: "～場合、～。～場合、さらに～"
    # Check this BEFORE other conditional parsing to avoid incorrect parsing
    if 'さらに' in text and text.count('場合') >= 2:
        # Split by period to get separate conditional effects
        parts = text.split('。')
        if len(parts) >= 2:
            # Check if the second part contains "さらに"
            if 'さらに' in parts[1]:
                # Parse first conditional effect
                first_effect = parse_effect(parts[0].strip())
                # Parse second conditional effect (strip "さらに")
                second_effect_text = parts[1].strip().replace('さらに', '', 1).strip()
                second_effect = parse_effect(second_effect_text)
                
                # Only set sequential if both effects have valid actions
                if first_effect.get('action') or first_effect.get('actions') or second_effect.get('action') or second_effect.get('actions'):
                    effect['action'] = 'sequential'
                    effect['actions'] = [first_effect, second_effect]
                    effect['text'] = text
                    # Post-processing: fix nested "draw" actions
                    for action in effect['actions']:
                        if action.get('action') == 'draw':
                            action['action'] = 'draw_card'
                    return effect
    
    # Check for sequential with duration condition ("その後、[condition]かぎり、[action]")
    if 'その後、' in text and 'かぎり、' in text:
        # Split by "その後、"
        parts = text.split('その後、', SPLIT_LIMIT)
        if len(parts) == 2:
            first_action_text = parts[0].strip()
            second_part = parts[1].strip()
            # Split second part by "かぎり、"
            if 'かぎり、' in second_part:
                condition_parts = second_part.split('かぎり、', 1)
                condition_text = condition_parts[0].strip()
                second_action_text = condition_parts[1].strip()
                # Parse actions
                first_action = parse_action(first_action_text)
                second_action = parse_action(second_action_text)
                # Parse condition
                condition = parse_condition(condition_text)
                # Attach condition to second action
                second_action['condition'] = condition
                second_action['duration'] = 'unless'  # "かぎり" means "unless/while"
                # Create sequential structure
                effect['action'] = 'sequential'
                effect['actions'] = [first_action, second_action]
                effect['text'] = text
                return effect
    
    # Check for choice marker BEFORE implicit sequential to prevent incorrect parsing
    if CHOICE_MARKER in text:
        # This will be handled later in the choice section
        pass
    # Check for implicit sequential (comma-separated actions) - check AFTER "その中から" pattern
    elif '、' in text_without_parens and not any(marker in text_without_parens for marker in CONDITION_MARKERS):
        parts = text_without_parens.split('、')
        if len(parts) >= 2:
            # Check if each part looks like a separate action
            actions = []
            for part in parts:
                # Strip leading comma if present (this can happen from splitting)
                cleaned_part = part.strip().lstrip('、')
                # Strip "その後" from action text if present
                if cleaned_part.endswith('その後'):
                    cleaned_part = cleaned_part[:-len('その後')].strip()
                elif cleaned_part.endswith('その後。'):
                    cleaned_part = cleaned_part[:-len('その後。')].strip()
                action = parse_action(cleaned_part)
                if action.get('action') != 'custom':
                    actions.append(action)
            if len(actions) >= 2:
                effect['action'] = 'sequential'
                effect['actions'] = actions
                # Post-processing: fix nested "draw" actions
                for action in effect['actions']:
                    if action.get('action') == 'draw':
                        action['action'] = 'draw_card'
                return effect
            # If no valid sequential actions were found, fall through to single action parsing
    
    # Check for conditional sequential actions ("そうした場合" - if so/then)
    if CONDITIONAL_SEQUENTIAL_MARKER in text:
        parts = text.split(CONDITIONAL_SEQUENTIAL_MARKER, SPLIT_LIMIT)
        first_action = parse_action(parts[0].strip())
        # Strip leading comma from second action if present
        second_part = parts[1].strip().lstrip('、')
        second_action = parse_action(second_part)
        effect['action'] = 'sequential'
        effect['actions'] = [first_action, second_action]
        effect['conditional'] = True
        # Post-processing: fix nested "draw" actions
        for action in effect['actions']:
            if action.get('action') == 'draw':
                action['action'] = 'draw_card'
        return effect
    
    # Check for conditional alternative effects ("代わりに" - instead/otherwise)
    if '代わりに' in text:
        # Pattern: "～場合、～。～場合、代わりに～"
        if '場合' in text:
            parts = text.split('代わりに', SPLIT_LIMIT)
            if len(parts) == 2:
                # Parse the alternative effect (after "代わりに")
                alternative_effect = parse_effect(parts[1].strip())
                
                # The first part contains: primary_effect + alternative_condition
                # Pattern: "primary_effect. condition、代わりに"
                first_part = parts[0].strip()
                
                # First, split by period to separate primary effect from alternative condition
                # This handles: "primary_effect. condition、代わりに"
                if '。' in first_part:
                    period_parts = first_part.split('。')
                    if len(period_parts) >= 2:
                        # The last period-separated part is the alternative condition
                        # Everything before is the primary effect
                        primary_text = '。'.join(period_parts[:-1]).strip()
                        condition_text = period_parts[-1].strip()
                        # Remove trailing "場合、" from condition_text if present
                        if condition_text.endswith('場合、'):
                            condition_text = condition_text[:-len('場合、')].strip()
                        
                        # Store primary effect as raw text (don't parse it since it contains its own condition)
                        primary_effect = {'text': primary_text}
                        
                        # Parse alternative condition
                        condition = parse_condition(condition_text)
                        # If condition text is empty or condition has empty text, set to None
                        if not condition_text or (condition and condition.get('text') == ''):
                            condition = None
                        
                        effect['action'] = 'conditional_alternative'
                        if primary_effect:
                            effect['primary_effect'] = primary_effect
                        effect['alternative_condition'] = condition
                        effect['alternative_effect'] = alternative_effect
                        effect['text'] = text
                        return effect
                
                # Fallback: try to find the last "場合、" to split
                last_case_index = first_part.rfind('場合、')
                if last_case_index != -1:
                    # Everything before the last "場合、" is the primary effect
                    primary_text = first_part[:last_case_index].strip()
                    # Everything after "場合、" is the alternative condition
                    condition_text = first_part[last_case_index + len('場合、'):].strip()
                    
                    # Store primary effect as raw text (don't parse it since it contains its own condition)
                    primary_effect = {'text': primary_text}
                    
                    # Parse alternative condition
                    condition = parse_condition(condition_text)
                    # If condition text is empty or condition has empty text, set to None
                    if not condition_text or (condition and condition.get('text') == ''):
                        condition = None
                else:
                    # If no clear split, treat entire first part as condition
                    primary_effect = None
                    condition = parse_condition(first_part)
                    # If condition has empty text, set to None
                    if condition and condition.get('text') == '':
                        condition = None
                
                effect['action'] = 'conditional_alternative'
                if primary_effect:
                    effect['primary_effect'] = primary_effect
                effect['alternative_condition'] = condition
                effect['alternative_effect'] = alternative_effect
                effect['text'] = text
                return effect
    
    # Check for sequential actions
    if SEQUENTIAL_MARKER in text:
        parts = text.split(SEQUENTIAL_MARKER, 1)
        first_action = parse_action(parts[0].strip())
        # Strip leading comma from second action if present
        second_part = parts[1].strip().lstrip('、')
        # Also strip any remaining transition markers like "その後" that might have been included
        if second_part.startswith('その後'):
            second_part = second_part[len('その後'):].strip()
        second_action = parse_action(second_part)
        effect['action'] = 'sequential'
        effect['actions'] = [first_action, second_action]
        return effect
    
    # Check for choice effects
    if CHOICE_MARKER in text:
        effect['action'] = 'choice'
        # Parse options by splitting on bullet points (・)
        # First, split by CHOICE_MARKER to get the part after "以下から1つを選ぶ"
        parts = text.split(CHOICE_MARKER, SPLIT_LIMIT)
        if len(parts) > 1:
            options_text = parts[1].strip()
            
            # Check for conditional modifier before bullet points
            # e.g., "自分の成功ライブカード置き場に『虹ヶ咲』のカードがある場合、代わりに1つ以上を選ぶ。\n・"
            conditional_modifier = None
            lines = options_text.split('\n')
            option_lines = []
            
            for i, line in enumerate(lines):
                line = line.strip()
                if line.startswith('・'):
                    # Found first bullet point, add remaining lines
                    option_lines.append(line[1:].strip())  # Remove the bullet point
                    # Add all subsequent lines
                    for j in range(i + 1, len(lines)):
                        subsequent_line = lines[j].strip()
                        if subsequent_line:
                            if subsequent_line.startswith('・'):
                                option_lines.append(subsequent_line[1:].strip())
                            elif option_lines:
                                # Continuation of previous option
                                option_lines[-1] += ' ' + subsequent_line
                    break
                elif not conditional_modifier and line:
                    # This is the conditional modifier before the first bullet
                    conditional_modifier = line
            
            # Store conditional modifier if present (and not just a period)
            if conditional_modifier and conditional_modifier != '。' and conditional_modifier != '.':
                effect['choice_modifier'] = conditional_modifier
                # Parse the conditional modifier
                condition = parse_condition(conditional_modifier)
                if condition.get('type') != 'custom':
                    effect['choice_condition'] = condition
            
            # Parse each option
            options = []
            for option_text in option_lines:
                if option_text:
                    parsed_option = parse_action(option_text)
                    options.append(parsed_option)
            
            if options:
                effect['options'] = options
                return effect
            # If no valid options, fall through to continue parsing
        # If no valid parts, fall through to continue parsing
    
    # Check for conditional effects
    condition_text, action_text = split_condition_action(text)
    if condition_text and action_text:
        condition = parse_condition(condition_text)
        # Strip leading comma from action text if present
        action_text = action_text.lstrip('、')
        # Extract duration prefix from action_text
        duration_prefixes = ['ライブ終了時まで、', 'このターンの間、', 'このライブの間、', 'ライブ終了時まで 、', 'このターンの間 、', 'このライブの間 、']
        duration = None
        for prefix in duration_prefixes:
            if action_text.startswith(prefix):
                duration = 'live_end'  # Simplified for now
                action_text = action_text[len(prefix):].strip()
                break
        # Strip period from action_text
        action_text = strip_suffix_period(action_text)
        
        # Special handling for yell count modification
        if 'エールによって公開される自分のカードの枚数が' in action_text:
            count_match = re.search(r'(\d+)枚', action_text)
            count = int(count_match.group(1)) if count_match else None
            effect['condition'] = condition
            effect['action'] = 'modify_yell_count'
            if '減る' in action_text or '減らす' in action_text:
                effect['operation'] = 'subtract'
            elif '増える' in action_text or '増やす' in action_text:
                effect['operation'] = 'add'
            if count:
                effect['count'] = count
            if duration:
                effect['duration'] = duration
            return effect
        
        action = parse_effect(action_text)
        if duration:
            action['duration'] = duration
        effect['condition'] = condition
        # If action is sequential, merge it properly to preserve condition
        if action.get('action') == 'sequential':
            effect['action'] = 'sequential'
            effect['actions'] = action.get('actions', [])
            if 'text' in action:
                effect['text'] = action['text']
        else:
            effect.update(action)
        # Remove redundant card_type from effect if condition already has it
        if 'card_type' in condition and 'card_type' in effect:
            del effect['card_type']
        # Only return if effect has valid action
        if effect.get('action') or effect.get('actions'):
            return effect
    
    # Check for ability activation effects (～能力を発動させる)
    ability_activation_match = re.search(r'(.+?)能力を発動させる', text)
    if ability_activation_match:
        target_text = ability_activation_match.group(1).strip()
        effect['action'] = 'activate_ability'
        effect['target'] = target_text
        # Extract trigger type if present (e.g., "{{toujyou.png|登場}}")
        trigger_match = re.search(r'\{\{(.+?)\}\}', target_text)
        if trigger_match:
            effect['target_trigger'] = trigger_match.group(1)
        # Extract parenthetical notes about cost payment
        if '(' in text and ')' in text:
            parenthetical_start = text.find('(')
            parenthetical_end = text.rfind(')')
            if parenthetical_start > 0 and parenthetical_end > parenthetical_start:
                parenthetical = text[parenthetical_start+1:parenthetical_end]
                effect['activation_note'] = parenthetical
        return effect
    
    # Check for baton touch specific conditions
    if 'バトンタッチ' in text and '場合' in text:
        baton_match = re.search(r'([^場合]+)場合', text)
        if baton_match:
            condition_text = baton_match.group(0)
            action_text = text.replace(condition_text, '').strip()
            condition = parse_condition(condition_text)
            condition['condition_type'] = 'baton_touch'
            # Extract source group if present (e.g., "『スリーズブーケ』のメンバーから")
            group_match = re.search(r'「([^」]+)」のメンバーから', condition_text)
            if group_match:
                condition['source_group'] = group_match.group(1)
            # Extract cost comparison if present (e.g., "コストが低い", "コストが高い")
            if 'コストが低い' in condition_text:
                condition['cost_comparison'] = 'lower'
            elif 'コストが高い' in condition_text:
                condition['cost_comparison'] = 'higher'
            action = parse_action(action_text)
            effect['condition'] = condition
            effect.update(action)
            return effect
    
    # Check for "これにより～した場合" (if done this way) pattern - ability invalidation follow-up
    if 'これにより無効にした場合' in text or 'これにより控え室に置いた場合' in text:
        parts = text.split('これにより', 1)
        first_part = parts[0].strip()
        second_part = 'これにより' + parts[1].strip()
        # Split the second part on "場合"
        if '場合' in second_part:
            condition_part, followup_part = second_part.split('場合', 1)
            condition_part = condition_part.strip() + '場合'
            followup_part = followup_part.strip()
            effect['action'] = 'conditional_on_result'
            effect['primary_action'] = parse_action(first_part)
            effect['result_condition'] = parse_condition(condition_part)
            effect['followup_action'] = parse_action(followup_part)
            return effect
    
    # Check for "そうした場合" (if so) pattern
    if 'そうした場合' in text:
        parts = text.split('そうした場合', SPLIT_LIMIT)
        first_part = parts[0].strip()
        action_text = parts[1].strip()
        # First part should be an optional action
        effect['action'] = 'conditional_on_optional'
        effect['optional_action'] = parse_action(first_part)
        # Check if conditional action has multiple targets (e.g., "～と、～は、それぞれ～を得る")
        if 'それぞれ' in action_text:
            # Parse the multiple target pattern
            effect['conditional_action'] = parse_action(action_text)
            effect['conditional_action']['multiple_targets'] = True
        else:
            effect['conditional_action'] = parse_action(action_text)
        return effect
    
    # Check for multiple actions joined by "し" (e.g., "スコアを＋2し、必要ハートは...になる")
    if '、' in text and 'し' in text:
        # Check if it's a "Aし、B" pattern
        parts = text.split('、')
        if len(parts) >= 2 and 'し' in parts[0]:
            actions = []
            for part in parts:
                action = parse_action(part.strip().rstrip('、'))
                if action.get('action') != 'custom':
                    actions.append(action)
            if len(actions) >= 2:
                effect['action'] = 'sequential'
                effect['actions'] = actions
                # Post-processing: fix nested "draw" actions
                for action in effect['actions']:
                    if action.get('action') == 'draw':
                        action['action'] = 'draw_card'
                return effect
    
    # Check for global modifier pattern (～は、～) e.g., "all cards are X"
    if re.search(r'.+は、.+', text) and 'ある場合' not in text:
        # Check if it's a global state change
        if '必要ハート' in text and ('多くなる' in text or '少なくなる' in text):
            effect['action'] = 'modify_required_hearts_global'
            if '多くなる' in text:
                effect['operation'] = 'increase'
            elif '少なくなる' in text:
                effect['operation'] = 'decrease'
            # Extract target
            target_match = re.search(r'([^は]+)は', text)
            if target_match:
                effect['target'] = target_match.group(1).strip()
            return effect
    
    # Check for play baton touch pattern (プレイに際し、バトンタッチしてもよい)
    if 'プレイに際し' in text and 'バトンタッチ' in text:
        effect['action'] = 'play_baton_touch'
        baton_match = re.search(r'(\d+)人のメンバーとバトンタッチ', text)
        if baton_match:
            effect['count'] = int(baton_match.group(1))
        return effect
    
    # Check for duration effects
    if DURATION_MARKER in text:
        parts = text.split(DURATION_MARKER, SPLIT_LIMIT)
        condition_text = parts[0].strip() + DURATION_MARKER
        action_text = parts[1].strip()
        # Strip leading comma from action text if present
        action_text = action_text.lstrip('、')
        condition = parse_condition(condition_text)
        action = parse_action(action_text)
        effect['condition'] = condition
        effect['duration'] = 'as_long_as'
        effect.update(action)
        return effect
    
    # Check for energy placement under member with condition
    # Check this early before other parsing
    if 'source' in effect and effect['source'] == 'under_member':
        effect['action'] = 'place_energy_under_member'
        effect['energy_count'] = effect.get('count', 1)
        effect['target_member'] = 'this_member'
        return effect
    
    # Check for cost reduction in effect text (e.g., "この能力を起動するためのコストは...減る")
    if 'コスト' in text and '減る' in text:
        reduction_match = re.search(r'(\d+)種類につき.*?(\d+)減る', text)
        if reduction_match:
            effect['cost_reduction'] = {
                'per_unit_type': 'group_variety',
                'per_unit_count': int(reduction_match.group(1)),
                'reduction_amount': int(reduction_match.group(2))
            }
        else:
            # Try simpler pattern: "X減る"
            simple_match = re.search(r'(\d+)減る', text)
            if simple_match:
                effect['cost_reduction'] = {
                    'reduction_amount': int(simple_match.group(1))
                }
        # Don't return yet, continue parsing the main effect
    
    # Check for re-yell action: "そのエールで得たブレードハートを失い、もう一度エールを行う"
    if 'もう一度エールを行う' in text and 'ブレードハートを失い' in text:
        effect['action'] = 're_yell'
        effect['lose_blade_hearts'] = True
        return effect
    
    # Check for restriction effect: "効果によってはアクティブにならない"
    if '効果によってはアクティブにならない' in text:
        effect['action'] = 'restriction'
        effect['restriction_type'] = 'cannot_activate_by_effect'
        # Extract target
        if '自分と相手の' in text:
            effect['target'] = 'both'
        elif '自分の' in text:
            effect['target'] = 'self'
        elif '相手の' in text:
            effect['target'] = 'opponent'
        # Extract duration if present
        if 'このターン' in text:
            effect['duration'] = 'this_turn'
        # Extract card type
        if 'メンバー' in text:
            effect['card_type'] = 'member_card'
        return effect
    
    # Check for energy placement under member with condition
    if 'source' in effect and effect['source'] == 'under_member':
        effect['action'] = 'place_energy_under_member'
        effect['energy_count'] = effect.get('count', 1)
        effect['target_member'] = 'this_member'
        return effect
    
    # Check for gain resource with equality condition
    # This can be in the text directly or already parsed in the condition field
    if 'を得る' in text and 'コストが同じ' in text:
        effect['action'] = 'gain_resource'
        effect['resource'] = 'blade'
        # Extract resource icon count
        icon_count = text.count('{{icon_blade.png|ブレード}}')
        if icon_count > 0:
            effect['count'] = icon_count
        # The equality condition should already be in the condition field
        return effect
    
    # Check for custom action that should be gain_resource with equality condition
    # If the effect is about doing the same thing with equality condition
    # This needs to be checked after the condition is parsed
    if '同じことを行う' in text:
        # Check if there's a comparison_condition in the effect
        if 'condition' in effect and effect['condition'].get('type') == 'comparison_condition':
            effect['action'] = 'gain_resource'
            effect['resource'] = 'blade'
            # Extract resource icon count from condition text
            condition_text = effect['condition'].get('text', '')
            icon_count = condition_text.count('{{icon_blade.png|ブレード}}')
            if icon_count > 0:
                effect['count'] = icon_count
            else:
                effect['count'] = 1
            # Keep the duration if present
            if 'duration' not in effect and 'ライブ終了時まで' in text:
                effect['duration'] = 'live_end'
            return effect
    
    # Check for set blade count action: "ブレードの数はXつになる"
    if 'ブレードの数は' in text and ('つになる' in text or 'になる' in text):
        effect['action'] = 'set_blade_count'
        # Extract the count
        count_match = re.search(r'(\d+)つになる', text)
        if count_match:
            effect['count'] = int(count_match.group(1))
        else:
            # Try alternative pattern
            count_match = re.search(r'(\d+)になる', text)
            if count_match:
                effect['count'] = int(count_match.group(1))
        return effect
    
    # Fallback: parse as a single action
    action = parse_action(text)
    effect.update(action)
    
    # Post-processing: check for energy placement under member
    # This needs to be checked after parse_action sets the source
    if effect.get('source') == 'under_member' and effect.get('action') == 'custom':
        effect['action'] = 'place_energy_under_member'
        effect['energy_count'] = effect.get('count', 1)
        effect['target_member'] = 'this_member'
        return effect
    
    # Post-processing: check for remaining custom actions by exact text match
    # Normalize text by removing all whitespace and icon tags
    # Remove all whitespace including newlines, spaces, tabs
    normalized_text = re.sub(r'\s+', '', text)
    # Also remove icon tags but preserve the text content between pipes
    # Use a different approach: replace {{...|...}} with just the content after the pipe
    pattern_text = re.sub(r'\{\{[^|]+\|([^}]+)\}\}', r'\1', normalized_text)
    
    # For Liella! set blade count ability
    if 'ブレードの数は3つになる' in pattern_text:
        effect['action'] = 'set_blade_count'
        effect['count'] = 3
        if 'duration' not in effect and 'ライブ終了時まで' in pattern_text:
            effect['duration'] = 'live_end'
        return effect
    
    # For Emma Punch with "何もしない" text
    if '何もしない' in pattern_text:
        effect['action'] = 'choice'
        effect['choice_type'] = 'emma_punch'
        return effect
    
    # For gain blade with equality condition
    if '元々の' in pattern_text and 'ブレード' in pattern_text and '同じ場合についても同じことを行う' in pattern_text:
        effect['action'] = 'gain_resource'
        effect['resource'] = 'blade'
        # Try to get count from effect if it exists
        if 'count' not in effect:
            effect['count'] = 1
        if 'duration' not in effect and 'ライブ終了時まで' in pattern_text:
            effect['duration'] = 'live_end'
        return effect
    
    # For optional draw action "カードを1枚引いてもよい"
    if 'カードを1枚引いてもよい' in pattern_text:
        effect['action'] = 'draw_card'
        effect['count'] = 1
        effect['optional'] = True
        return effect
    
    # Ensure effect has at least an action field and no empty actions array
    if 'action' not in effect and 'actions' not in effect:
        effect['action'] = 'custom'
    # Remove empty actions array if present
    if 'actions' in effect and not effect['actions']:
        del effect['actions']
    
    # Post-processing: fix "draw" action in main effect (do this last to ensure it's always executed)
    if effect.get('action') == 'draw':
        effect['action'] = 'draw_card'
    
    # Post-processing: infer action for per_unit effects that are missing the action field
    if effect.get('per_unit') and 'action' not in effect:
        # Infer action from text
        if 'ブレードを得る' in text or '選んだブレード' in text:
            effect['action'] = 'gain_resource'
            effect['resource'] = 'blade'
            # Extract resource icon count
            icon_count = text.count('{{icon_blade.png|ブレード}}')
            if icon_count > 0:
                effect['count'] = icon_count
        elif 'ハートを得る' in text or '選んだハート' in text:
            effect['action'] = 'gain_resource'
            effect['resource'] = 'heart'
        elif '引く' in text:
            effect['action'] = 'draw_card'
        # Set duration if present
        if 'ライブ終了時まで' in text:
            effect['duration'] = 'live_end'
    
    # Post-processing: handle choice + per_unit pattern (choice first, then per_unit gain)
    if 'のうち、1つを選ぶ' in text and 'につき' in text and 'action' not in effect:
        # This is a choice action followed by a per_unit gain action
        # Infer the resource type from the choice options
        if 'ハート' in text:
            effect['action'] = 'gain_resource'
            effect['resource'] = 'heart'
        elif 'ブレード' in text:
            effect['action'] = 'gain_resource'
            effect['resource'] = 'blade'
        else:
            effect['action'] = 'choice'
        # The per_unit info is already set
        # Add duration if present
        if 'ライブ終了時まで' in text:
            effect['duration'] = 'live_end'
    
    # Post-processing: handle duration prefix + per_unit pattern
    if 'につき' in text and 'action' not in effect and effect.get('per_unit'):
        # Infer action from text
        if 'ブレードを得る' in text:
            effect['action'] = 'gain_resource'
            effect['resource'] = 'blade'
            # Extract resource icon count
            icon_count = text.count('{{icon_blade.png|ブレード}}')
            if icon_count > 0:
                effect['count'] = icon_count
        elif 'ハートを得る' in text or '選んだハート' in text:
            effect['action'] = 'gain_resource'
            effect['resource'] = 'heart'
        elif '引く' in text:
            effect['action'] = 'draw_card'
        # Duration is already set by the duration prefix stripping
    
    # Post-processing: ensure gain_resource effects have count and resource fields
    if effect.get('action') == 'gain_resource':
        # Ensure resource field is set
        if 'resource' not in effect:
            # Infer from text
            if 'ブレード' in effect.get('text', ''):
                effect['resource'] = 'blade'
            elif 'ハート' in effect.get('text', ''):
                effect['resource'] = 'heart'
        # Ensure count field is set
        if 'count' not in effect:
            text = effect.get('text', '')
            # Try to extract from numeric text
            count_match = re.search(r'(\d+)つ', text)
            if count_match:
                effect['count'] = int(count_match.group(1))
            else:
                # Try to extract from icon counts
                blade_count = text.count('{{icon_blade.png|ブレード}}')
                heart_count = len(re.findall(r'{{heart_\d+\.png|heart\d+}}', text))
                if blade_count > 0:
                    effect['count'] = blade_count
                elif heart_count > 0:
                    effect['count'] = heart_count
                else:
                    # Default to 1 if not specified
                    effect['count'] = 1
    
    return effect

def parse_ability(triggerless_text: str) -> Dict[str, Any]:
    """Parse a complete ability text."""
    ability = {
        'triggerless_text': triggerless_text,
    }
    
    # Identify structure
    structure = identify_structure(triggerless_text)
    
    # Split cost and effect
    cost_text, effect_text = split_cost_effect(triggerless_text)
    
    # Parse cost
    if cost_text:
        ability['cost'] = parse_cost(cost_text)
    
    # Parse effect
    if effect_text:
        ability['effect'] = parse_effect(effect_text)
    
    return ability

# ============== PROCESSING ==============

def process_abilities(data: Dict[str, Any]) -> Dict[str, Any]:
    """Process all abilities in the data."""
    for ability in data['unique_abilities']:
        triggerless = ability.get('triggerless_text', '')
        if triggerless:
            parsed = parse_ability(triggerless)
            ability['parsed'] = parsed

    return data


if __name__ == '__main__':
    import json
    from pathlib import Path
    
    abilities_file = Path(__file__).parent.parent / 'abilities.json'
    
    with open(abilities_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    # Process abilities
    result = process_abilities(data)
    
    # Update cost fields from parsed results
    for ability in result['unique_abilities']:
        parsed = ability.get('parsed', {})
        if 'cost' in parsed:
            # Update existing cost with fields from parsed cost
            cost = ability.get('cost', {})
            parsed_cost = parsed['cost']
            
            # Merge fields from parsed cost into existing cost
            for key, value in parsed_cost.items():
                if key not in cost:
                    cost[key] = value
            
            ability['cost'] = cost
    
    # Update effect fields from parsed results
    for ability in result['unique_abilities']:
        parsed = ability.get('parsed', {})
        if 'effect' in parsed:
            # Update existing effect with fields from parsed effect
            effect = ability.get('effect', {})
            parsed_effect = parsed['effect']
            
            # Merge fields from parsed effect into existing effect
            for key, value in parsed_effect.items():
                if key == 'actions' and isinstance(value, list):
                    # For sequential actions, merge each action
                    existing_actions = effect.get('actions', [])
                    for i, parsed_action in enumerate(value):
                        if i < len(existing_actions):
                            # Merge fields into existing action
                            for action_key, action_value in parsed_action.items():
                                # Always overwrite position with PositionInfo format
                                if action_key == 'position' and isinstance(action_value, dict):
                                    existing_actions[i][action_key] = action_value
                                elif action_key not in existing_actions[i]:
                                    existing_actions[i][action_key] = action_value
                        else:
                            # Add new action
                            existing_actions.append(parsed_action)
                    effect['actions'] = existing_actions
                elif key not in effect:
                    effect[key] = value
            
            ability['effect'] = effect
    
    # Remove parsed field
    for ability in result['unique_abilities']:
        if 'parsed' in ability:
            del ability['parsed']
    
    with open(abilities_file, 'w', encoding='utf-8') as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    
    print("Processed abilities.json with parser.py")

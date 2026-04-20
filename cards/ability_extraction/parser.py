"""Parser for ability extraction - structural approach based on actual data analysis."""
import re
from typing import Dict, List, Optional, Any, Tuple, Union
from parser_utils import (
    extract_count,
    extract_group_name,
    detect_group_type,
    normalize_fullwidth_digits,
    strip_suffix_period,
)

# ============== SOURCE PATTERNS (FROM) ==============
SOURCE_PATTERNS = [
    ('手札から', 'hand'),
    ('手札にある', 'hand'),
    ('控え室から', 'discard'),
    ('自分の控え室にある', 'discard'),
    ('ステージから', 'stage'),
    ('デッキの上から', 'deck_top'),
    ('エネルギー置き場から', 'energy_zone'),
    ('エネルギー置き場にある', 'energy_zone'),
    ('自分のエネルギー置き場にある', 'energy_zone'),
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
    ('手札に', 'hand'),
    ('ステージに登場させる', 'stage'),
    ('ステージに置く', 'stage'),
    ('デッキの上に置く', 'deck_top'),
    ('デッキの一番下に置く', 'deck_bottom'),
    ('デッキの一番下に置いて', 'deck_bottom'),  # Handle te-form
    ('成功ライブカード置き場に置く', 'success_live_card_zone'),
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
CONDITION_MARKERS = ['場合、', '場合', 'とき、', 'とき', 'なら、', 'なら']

# ============== STRUCTURAL MARKERS ==============
SEQUENTIAL_MARKER = 'その後、'
CONDITIONAL_SEQUENTIAL_MARKER = 'そうした場合'
CHOICE_MARKER = '以下から1つを選ぶ'
DURATION_MARKER = 'かぎり'
COMPOUND_OPERATOR = 'かつ'
PER_UNIT_MARKER = 'につき'
EACH_TIME_MARKER = 'たび'
EITHER_CASE_MARKER = 'いずれかの場合'

# ============== UTILITY FUNCTIONS ==============

def extract_source(text: str) -> Optional[str]:
    """Extract source location (FROM)."""
    for pattern, code in SOURCE_PATTERNS:
        if pattern in text:
            return code
    return None

def extract_destination(text: str) -> Optional[str]:
    """Extract destination location (TO)."""
    for pattern, code in DESTINATION_PATTERNS:
        if pattern in text:
            return code
    return None

def extract_location(text: str) -> Optional[str]:
    """Extract location (general)."""
    for pattern, code in LOCATION_PATTERNS:
        if pattern in text:
            return code
    return None

def extract_state_change(text: str) -> Optional[str]:
    """Extract state change (wait/active)."""
    for pattern, code in STATE_CHANGE_PATTERNS:
        if pattern in text:
            return code
    return None

def extract_card_type(text: str) -> Optional[str]:
    """Extract card type."""
    for pattern, code in CARD_TYPE_PATTERNS:
        if pattern in text:
            return code
    return None

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
    for pattern, code in OPERATOR_PATTERNS:
        if pattern in text:
            return code
    return None

def extract_group(text: str) -> Optional[Dict[str, str]]:
    """Extract group information."""
    if '『' in text:
        group = extract_group_name(text)
        if group:
            return {
                'name': group,
                'type': detect_group_type(group)
            }
    return None

def extract_cost_limit(text: str) -> Optional[Union[int, List[int]]]:
    """Extract cost limit."""
    match = re.search(r'コスト(\d+)(?:以上|以下)', text)
    if match:
        return int(match.group(1))
    return None

def extract_position(text: str) -> Optional[str]:
    """Extract position requirement."""
    if 'センターエリア' in text:
        return 'center'
    if '左サイドエリア' in text or '【左サイド】' in text or '左サイド' in text:
        return 'left_side'
    if '右サイドエリア' in text or '【右サイド】' in text or '右サイド' in text:
        return 'right_side'
    return None

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
        'text': text,
    }
    
    # Check for baton touch conditions
    if 'バトンタッチして登場した' in text:
        condition['type'] = 'baton_touch_condition'
        # Extract specific member if quoted (e.g., 「中須かすみ」からバトンタッチ)
        quoted = extract_quoted_text(text)
        if quoted:
            condition['source_member'] = quoted[0] if quoted else None
        # Check for negation (能力を持たないメンバーから)
        if '能力を持たない' in text:
            condition['ability_negation'] = True
        # Check for cost comparison (このメンバーよりコストが低い)
        if 'コストが低い' in text or 'コストが小さい' in text:
            condition['cost_comparison'] = 'lower'
        elif 'コストが高い' in text or 'コストが大きい' in text:
            condition['cost_comparison'] = 'higher'
        return condition
    
    # Check for movement conditions
    if 'エリアを移動した' in text:
        condition['type'] = 'movement_condition'
        condition['movement'] = True
        # Check for negation (移動していない)
        if '移動していない' in text:
            condition['negated'] = True
        return condition
    
    # Check for temporal conditions
    if 'このターン' in text:
        condition['type'] = 'temporal_condition'
        condition['temporal'] = 'this_turn'
        # Check for specific phase
        if 'ライブフェイズ' in text:
            condition['phase'] = 'live_phase'
        elif 'メインフェイズ' in text:
            condition['phase'] = 'main_phase'
        return condition
    
    # Check for distinct conditions
    if '名前が異なる' in text:
        condition['type'] = 'distinct_condition'
        condition['distinct'] = 'name'
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
            condition['type'] = 'position_condition'
            condition['position'] = position
            return condition
    
    # Check for state conditions
    if 'ウェイト状態である' in text or 'ウェイト状態にある' in text:
        condition['type'] = 'state_condition'
        condition['state'] = 'wait'
        return condition
    if 'アクティブ状態である' in text or 'アクティブ状態にある' in text:
        condition['type'] = 'state_condition'
        condition['state'] = 'active'
        return condition
    
    # Check for ability negation
    if '能力も持たない' in text or '能力を持たない' in text:
        condition['type'] = 'ability_negation_condition'
        condition['ability_negation'] = True
        return condition
    
    # Check for heart negation (ブレードハートを持たない)
    if 'ブレードハートを持たない' in text or 'ハートを持たない' in text:
        condition['type'] = 'heart_negation_condition'
        condition['heart_negation'] = True
        return condition
    
    # Check for same group name condition
    if '同じグループ名を持つ' in text:
        condition['type'] = 'same_group_condition'
        condition['same_group'] = True
        return condition
    
    # Check for heart variety condition (6種類以上ある)
    if '種類以上ある' in text or '種類以上含まれる' in text:
        condition['type'] = 'heart_variety_condition'
        variety_count = extract_count(text)
        if variety_count:
            condition['variety_count'] = variety_count
        return condition
    
    # Check for energy payment negation (E支払わないかぎり)
    if '支払わないかぎり' in text:
        condition['type'] = 'payment_negation_condition'
        condition['negated'] = True
        # Extract payment amount
        payment_count = extract_count(text)
        if payment_count:
            condition['payment_count'] = payment_count
        return condition
    
    # Check for negative choice (そうしなかった)
    if 'そうしなかった' in text:
        condition['type'] = 'negative_choice_condition'
        return condition
    
    # Check for any_of conditions (いずれか)
    if 'いずれか' in text:
        # Extract values
        values_match = re.search(r'(\d+)[、\s]*(\d+)[、\s]*(\d+)[、\s]*(\d+)[、\s]*(\d+)', text)
        if values_match:
            values = [int(g) for g in values_match.groups()]
            condition['type'] = 'any_of_condition'
            condition['values'] = values
            condition['any_of'] = True
            return condition
    
    # Check for comparison conditions (after extraction, if comparison fields exist)
    # This will be set after the extraction phase below
    
    # Check for OR conditions (か、 = or)
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
    
    # Check for compound conditions (かつ)
    if COMPOUND_OPERATOR in text:
        parts = [p.strip() for p in text.split(COMPOUND_OPERATOR) if p.strip()]
        if len(parts) >= 2:
            parsed_conditions = [parse_condition(p) for p in parts]
            # Don't filter out custom conditions - keep structure even if unparsed
            if len(parsed_conditions) >= 2:
                compound = {
                    'type': 'compound',
                    'operator': 'and',
                    'conditions': parsed_conditions,
                    'text': text
                }
                # Don't set target on compound - let sub-conditions have their own targets
                return compound
    
    # Check for except conditions (以外)
    if '以外' in text:
        condition['except'] = True
        # Extract the thing being excluded
        except_match = re.search(r'([^以外]+)以外', text)
        if except_match:
            condition['except_target'] = except_match.group(1).strip()
        # Check if the exclusion is quoted (e.g., 「name」以外)
        quoted_exclusions = extract_quoted_text(text)
        if quoted_exclusions:
            condition['except_quoted'] = quoted_exclusions
    
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
            if 'heart_01' in text:
                condition['resource_type'] = 'heart_01'
            elif 'heart_02' in text:
                condition['resource_type'] = 'heart_02'
            elif 'heart_06' in text:
                condition['resource_type'] = 'heart_06'
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
    if '相手より' in text:
        condition['comparison_target'] = 'opponent'
        if '高い' in text:
            condition['comparison_operator'] = '>'
        elif '低い' in text or '少ない' in text:
            condition['comparison_operator'] = '<'
    elif '自分より' in text:
        condition['comparison_target'] = 'self'
    
    # Extract comparison type (score, cost, etc.)
    if 'スコア' in text:
        condition['comparison_type'] = 'score'
    elif 'コスト' in text:
        condition['comparison_type'] = 'cost'
    
    # Extract aggregate (total/sum)
    if '合計' in text:
        condition['aggregate'] = 'total'
    
    # Extract exact match (ちょうど)
    if 'ちょうど' in text:
        condition['operator'] = '='
    
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
        condition['movement'] = True
    
    # Extract baton touch trigger condition
    if 'バタンタッチ' in text:
        condition['trigger_type'] = 'baton_touch'
    
    # Extract movement state (～ている = ongoing state vs ～た = completed)
    if '移動している' in text:
        condition['movement_state'] = 'has_moved'
    
    # Extract temporal scope
    if 'このターン' in text:
        condition['temporal'] = 'this_turn'
    elif 'このライブ' in text:
        condition['temporal'] = 'this_live'
    
    # Extract distinct/unique flags
    if '名前が異なる' in text:
        condition['distinct'] = 'name'
    elif 'カード名が異なる' in text:
        condition['distinct'] = 'card_name'
    elif 'グループ名が異なる' in text:
        condition['distinct'] = 'group_name'
    if 'コストがそれぞれ異なる' in text:
        condition['distinct'] = 'cost'
    
    # Extract all areas flag
    if 'エリアすべて' in text:
        condition['all_areas'] = True
    
    # Extract exclude_self flag (other members)
    if 'ほかのメンバー' in text or 'このメンバー以外' in text:
        condition['exclude_self'] = True
    
    # Extract any_of pattern with multiple values (e.g., "10、20、30のいずれか")
    if 'いずれか' in text:
        # Try to extract the values
        import re
        values_match = re.search(r'(\d+)(?:、(\d+))+(?:のいずれか)', text)
        if values_match:
            values = re.findall(r'\d+', values_match.group(0))
            condition['values'] = [int(v) for v in values]
            condition['any_of'] = True
    
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
        condition['position'] = position
    
    # Determine condition type
    if location and count and operator:
        condition['type'] = 'location_count_condition'
        # If group is present, it's a group-specific location count condition
        if group:
            condition['type'] = 'group_location_count_condition'
    elif cost_limit:
        condition['type'] = 'cost_limit_condition'
    elif condition.get('resource_type') and count and operator:
        # Heart count or energy count conditions with group/location context
        condition['type'] = 'resource_count_condition'
        if group:
            condition['type'] = 'group_resource_count_condition'
    elif group or group_names:
        condition['type'] = 'group_condition'
    elif location and card_type:
        condition['type'] = 'location_condition'
    elif location and position:
        condition['type'] = 'position_condition'
    elif condition.get('resource_type') == 'energy' and count:
        condition['type'] = 'energy_count_condition'
    elif condition.get('resource_type') == 'surplus_heart':
        condition['type'] = 'surplus_heart_condition'
    elif source and destination:
        condition['type'] = 'move_action_condition'
    elif condition.get('comparison_target') or condition.get('comparison_type'):
        condition['type'] = 'comparison_condition'
    else:
        condition['type'] = 'custom'
    
    return condition

def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text."""
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
    
    # Extract per-unit modifier (につき)
    if PER_UNIT_MARKER in text:
        per_unit_match = re.search(r'([^につき]+)につき', text)
        if per_unit_match:
            action['per_unit'] = per_unit_match.group(1).strip()
    # Extract per-unit for modify_cost (e.g., "カード1枚につき、コストを＋1する")
    if '枚につき' in text and 'コスト' in text:
        action['per_unit'] = re.search(r'([^枚]+)枚につき', text).group(1).strip() if re.search(r'([^枚]+)枚につき', text) else 'card'
    # Extract per-unit for modify_required_hearts (e.g., "カード1枚につき、必要ハートを＋1する")
    if '枚につき' in text and '必要ハート' in text:
        action['per_unit'] = re.search(r'([^枚]+)枚につき', text).group(1).strip() if re.search(r'([^枚]+)枚につき', text) else 'card'
    
    # Extract source - handle "手札を" pattern for discard
    if '手札を' in text and '控え室に置く' in text:
        action['source'] = 'hand'
    
    # Extract source
    source = extract_source(text)
    if source and 'source' not in action:
        action['source'] = source
    # Check for under_member source (e.g., "下に置かれているエネルギーカード")
    if '下に置かれているエネルギーカード' in text:
        action['source'] = 'under_member'
        action['card_type'] = 'energy_card'
    
    # Extract destination
    destination = extract_destination(text)
    if destination:
        action['destination'] = destination
    # Check for destination choice (e.g., "好きなエリアに移動させる")
    if '好きなエリア' in text:
        action['destination_choice'] = True
    
    # Extract state change
    state_change = extract_state_change(text)
    if state_change:
        action['state_change'] = state_change
    
    # Extract count
    count = extract_count(text)
    if count:
        action['count'] = count
    else:
        # If no numeric count, count resource icons
        blade_count = text.count('{{icon_blade.png|ブレード}}')
        heart_count = text.count('{{icon_all.png|ハート}}') + text.count('{{heart_')
        energy_count = text.count('{{icon_energy.png|E}}')
        if blade_count > 0:
            action['count'] = blade_count
        elif heart_count > 0:
            action['count'] = heart_count
        elif energy_count > 0:
            action['count'] = energy_count
    
    # Extract card type
    card_type = extract_card_type(text)
    if card_type:
        action['card_type'] = card_type
    
    # Extract target
    target = extract_target(text)
    if target:
        action['target'] = target
    
    # Extract group
    group = extract_group(text)
    if group:
        action['group'] = group
    
    # Extract group names from 『』 brackets
    group_names = extract_group_names(text)
    if group_names:
        action['group_names'] = group_names
    
    # Extract quoted text from 「」
    quoted_text = extract_quoted_text(text)
    if quoted_text:
        action['quoted_text'] = quoted_text
    
    # Extract cost limit
    cost_limit = extract_cost_limit(text)
    if cost_limit:
        action['cost_limit'] = cost_limit
    
    # Extract position
    position = extract_position(text)
    if position:
        action['position'] = position
    
    # Extract optional flag
    if extract_optional(text):
        action['optional'] = True
    
    # Extract max flag
    if extract_max(text):
        action['max'] = True
    
    # Extract effect constraints
    if '未満にはならない' in text:
        constraint_match = re.search(r'(\d+)未満にはならない', text)
        if constraint_match:
            action['constraint'] = {'type': 'minimum_value', 'value': int(constraint_match.group(1))}
    elif '以上にはならない' in text:
        constraint_match = re.search(r'(\d+)以上にはならない', text)
        if constraint_match:
            action['constraint'] = {'type': 'maximum_value', 'value': int(constraint_match.group(1))}
    
    # Check for shuffle action (5.5. シャッフルする)
    if 'シャッフルする' in text or 'シャッフルして' in text:
        action['action'] = 'shuffle'
        # Extract target location to shuffle
        if 'デッキ' in text:
            action['target'] = 'deck'
        elif 'エネルギーデッキ' in text:
            action['target'] = 'energy_deck'
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
        action['action'] = 'pay_energy'
        action['energy'] = text.count('{{icon_energy.png|E}}')
        # Extract target (self/opponent)
        if target:
            action['target'] = target
        else:
            action['target'] = 'self'  # Default to self
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
        elif quoted_text:
            action['target_member'] = quoted_text
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
    elif source and destination:
        action['action'] = 'move_cards'
    # Check for "加える" (add to) pattern - common for adding cards to hand
    elif '加える' in text or '加え' in text:
        action['action'] = 'move_cards'
        if not destination:
            action['destination'] = 'hand'
    elif destination:
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
        action['action'] = 'move_cards'
        # If destination not already set, infer it from context
        if 'destination' not in action:
            if '控え室に置く' in text or '控え室に置いて' in text:
                action['destination'] = 'discard'
            elif '手札に置く' in text:
                action['destination'] = 'hand'
            elif 'ステージに置く' in text:
                action['destination'] = 'stage'
            elif 'デッキの上に置く' in text:
                action['destination'] = 'deck_top'
            elif 'デッキの一番下に置く' in text:
                action['destination'] = 'deck_bottom'
    # Check for destination-only moves (inferred source)
    elif destination:
        action['action'] = 'move_cards'
    elif '引く' in text or '引き' in text:
        action['action'] = 'draw'
        action['source'] = 'deck'
        action['destination'] = 'hand'
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
    elif '公開する' in text:
        action['action'] = 'reveal'
    elif 'エールによって公開される自分のカードの枚数が' in text:
        action['action'] = 'modify_eale_count'
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
        # Check for character-specific resource mapping: "「X」はYを得る"
        character_mapping_match = re.search(r'「([^」]+)」は(.+)を得る', text)
        if character_mapping_match:
            action['action'] = 'character_resource_mapping'
            action['character'] = character_mapping_match.group(1)
            action['resource_text'] = character_mapping_match.group(2)
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
    elif 'を得る' in text and '能力' in text:
        action['action'] = 'gain_ability'
        # Extract ability name from quoted_text if available
        if quoted_text:
            action['ability'] = quoted_text
        else:
            ability_match = re.search(r'「([^」]+)」を得る', text)
            if ability_match:
                action['ability'] = [ability_match.group(1)]
    # Check for gain_ability via quoted text (even without explicit "能力" keyword)
    elif quoted_text and any('ライブ' in q or 'スコア' in q or 'ブレード' in q or 'ハート' in q for q in quoted_text):
        action['action'] = 'gain_ability'
        action['ability'] = quoted_text
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
    
    # Check for choice cost (～か、～)
    if 'か、' in text:
        parts = text.split('か、', 1)
        if len(parts) == 2:
            # Parse each option as a separate cost
            option1 = parse_cost(parts[0].strip())
            option2 = parse_cost(parts[1].strip())
            return {
                'text': text,
                'type': 'choice',
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
    
    # Extract destination
    destination = extract_destination(text)
    if destination:
        cost['destination'] = destination
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
        cost['type'] = 'state_change'  # Set type for wait/active costs
    
    # Check for reveal card pattern (公開してもよい)
    if '公開してもよい' in text:
        cost['type'] = 'reveal_card'
    
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
    
    # Determine cost type - check move_cards first
    if cost.get('source') and cost.get('destination'):
        cost['type'] = 'move_cards'
    elif 'ウェイトにする' in text or 'ウェイト状態で置く' in text or 'ウェイト状態で登場させる' in text or 'アクティブにする' in text:
        cost['type'] = 'change_state'
    elif state_change:
        cost['type'] = 'change_state'
    elif '公開する' in text:
        cost['type'] = 'reveal'
    elif '{{icon_energy.png|E}}' in text:
        cost['type'] = 'pay_energy'
        cost['energy'] = text.count('{{icon_energy.png|E}}')
    elif cost.get('source'):
        # If source but no destination, infer based on common patterns
        if cost['source'] == 'hand' and ('控え室に置く' in text or '控え室に置いて' in text):
            cost['destination'] = 'discard'
            cost['type'] = 'move_cards'
        elif cost['source'] == 'discard' and '手札に加える' in text:
            cost['destination'] = 'hand'
            cost['type'] = 'move_cards'
        else:
            cost['type'] = 'custom'
    else:
        cost['type'] = 'custom'
    
    return cost

def parse_effect(text: str) -> Dict[str, Any]:
    """Parse an effect text."""
    text = normalize_fullwidth_digits(text).strip()
    text = strip_suffix_period(text)
    
    effect = {
        'text': text,
    }
    
    # Extract parenthetical notes
    parenthetical = extract_parenthetical(text)
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
            'type': 'player_choice',
            'options': ['self', 'opponent'],
            'text': choice_text
        }
        # Parse the remaining text as the main effect
        remaining_effect = parse_effect(remaining_text)
        effect.update(remaining_effect)
        return effect
    
    # Check for opponent choice/action patterns
    if text.startswith('相手は'):
        # Extract the opponent action part
        opponent_match = re.match(r'相手は(.+?)。', text)
        if opponent_match:
            opponent_action_text = opponent_match.group(0)
            remaining_text = text[len(opponent_action_text):].strip()
            effect['action_by'] = 'opponent'
            # Parse the opponent action
            opponent_action = parse_action(opponent_match.group(1).strip())
            effect['opponent_action'] = opponent_action
            # Parse remaining text if any
            if remaining_text:
                remaining_effect = parse_effect(remaining_text)
                effect.update(remaining_effect)
            return effect
    
    # Check for opponent actions after conditional markers (e.g., "そうした場合、相手は～")
    if '、相手は' in text:
        # Split by the opponent action marker
        parts = text.split('、相手は', 1)
        if len(parts) == 2:
            first_part = parts[0].strip()
            opponent_part = '相手は' + parts[1]
            # Extract the opponent action
            opponent_match = re.match(r'相手は(.+?)。', opponent_part)
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
                # Add opponent action with metadata
                opponent_action = parse_action(opponent_match.group(1).strip())
                opponent_action['action_by'] = 'opponent'
                effect['actions'].append(opponent_action)
                # Add remaining action if any
                if remaining_text:
                    remaining_effect = parse_action(remaining_text)
                    effect['actions'].append(remaining_effect)
                effect['conditional'] = True
                effect['text'] = text
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
                parts = select_text.split('、', 1)
                if len(parts) == 2:
                    first_action = parse_action(parts[0].strip())
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
    
    # Check for sequential with duration condition ("その後、[condition]かぎり、[action]")
    if 'その後、' in text and 'かぎり、' in text:
        # Split by "その後、"
        parts = text.split('その後、', 1)
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
    
    # Check for implicit sequential (comma-separated actions) - check AFTER "その中から" pattern
    if '、' in text_without_parens and not any(marker in text_without_parens for marker in CONDITION_MARKERS):
        parts = text_without_parens.split('、')
        if len(parts) >= 2:
            # Check if each part looks like a separate action
            actions = []
            for part in parts:
                action = parse_action(part.strip())
                if action.get('action') != 'custom':
                    actions.append(action)
            if len(actions) >= 2:
                effect['action'] = 'sequential'
                effect['actions'] = actions
                return effect
    
    # Check for conditional sequential actions ("そうした場合" - if so/then)
    if CONDITIONAL_SEQUENTIAL_MARKER in text:
        parts = text.split(CONDITIONAL_SEQUENTIAL_MARKER, 1)
        first_action = parse_action(parts[0].strip())
        second_action = parse_action(parts[1].strip())
        effect['action'] = 'sequential'
        effect['actions'] = [first_action, second_action]
        effect['conditional'] = True
        return effect
    
    # Check for conditional alternative effects ("代わりに" - instead/otherwise)
    if '代わりに' in text:
        # Pattern: "～場合、代わりに～"
        if '場合' in text:
            parts = text.split('代わりに', 1)
            if len(parts) == 2:
                # Parse the condition part (before "代わりに")
                condition_text = parts[0].strip()
                # Extract the condition from the condition part
                condition_match = re.search(r'([^、]+)場合、', condition_text)
                if condition_match:
                    condition = parse_condition(condition_match.group(1).strip())
                else:
                    condition = parse_condition(condition_text)
                # Parse the alternative effect
                alternative_effect = parse_effect(parts[1].strip())
                # Parse the primary effect (before the condition)
                primary_match = re.search(r'^(.+?)[^、]+場合', text)
                if primary_match:
                    primary_effect = parse_effect(primary_match.group(1).strip())
                else:
                    primary_effect = None
                
                effect['action'] = 'conditional_alternative'
                if primary_effect:
                    effect['primary_effect'] = primary_effect
                effect['alternative_condition'] = condition
                effect['alternative_effect'] = alternative_effect
                return effect
    
    # Check for sequential actions
    if SEQUENTIAL_MARKER in text:
        parts = text.split(SEQUENTIAL_MARKER, 1)
        first_action = parse_action(parts[0].strip())
        second_action = parse_action(parts[1].strip())
        effect['action'] = 'sequential'
        effect['actions'] = [first_action, second_action]
        return effect
    
    # Check for choice effects
    if CHOICE_MARKER in text:
        effect['action'] = 'choice'
        # Parse options by splitting on bullet points (・)
        # First, split by CHOICE_MARKER to get the part after "以下から1つを選ぶ"
        parts = text.split(CHOICE_MARKER, 1)
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
            else:
                effect['options'] = []
        else:
            effect['options'] = []
        return effect
    
    # Check for conditional effects
    condition_text, action_text = split_condition_action(text)
    if condition_text and action_text:
        condition = parse_condition(condition_text)
        # Extract duration prefix from action_text
        duration_prefixes = ['ライブ終了時まで、', 'このターンの間、', 'このライブの間、']
        duration = None
        for prefix in duration_prefixes:
            if action_text.startswith(prefix):
                duration = 'live_end'  # Simplified for now
                action_text = action_text[len(prefix):].strip()
                break
        # Strip period from action_text
        action_text = strip_suffix_period(action_text)
        
        # Special handling for eale count modification
        if 'エールによって公開される自分のカードの枚数が' in action_text:
            count_match = re.search(r'(\d+)枚', action_text)
            count = int(count_match.group(1)) if count_match else None
            effect['condition'] = condition
            effect['action'] = 'modify_eale_count'
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
        return effect
    
    # Check for either-case conditions (いずれかの場合)
    if EITHER_CASE_MARKER in text:
        parts = text.split(EITHER_CASE_MARKER, 1)
        condition_text = parts[0].strip() + EITHER_CASE_MARKER
        action_text = parts[1].strip()
        condition = parse_condition(condition_text)
        condition['type'] = 'either_case'
        action = parse_action(action_text)
        effect['condition'] = condition
        effect.update(action)
        return effect
    
    # Check for baton touch specific conditions
    if 'バトンタッチ' in text and '場合' in text:
        baton_match = re.search(r'([^場合]+)場合', text)
        if baton_match:
            condition_text = baton_match.group(0)
            action_text = text.replace(condition_text, '').strip()
            condition = parse_condition(condition_text)
            condition['type'] = 'baton_touch'
            action = parse_action(action_text)
            effect['condition'] = condition
            effect.update(action)
            return effect
    
    # Check for "そうした場合" (if so) pattern
    if 'そうした場合' in text:
        parts = text.split('そうした場合', 1)
        first_part = parts[0].strip()
        action_text = parts[1].strip()
        # First part should be an optional action
        effect['action'] = 'conditional_on_optional'
        effect['optional_action'] = parse_action(first_part)
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
        parts = text.split(DURATION_MARKER, 1)
        condition_text = parts[0].strip() + DURATION_MARKER
        action_text = parts[1].strip()
        condition = parse_condition(condition_text)
        action = parse_action(action_text)
        effect['condition'] = condition
        effect['duration'] = 'as_long_as'
        effect.update(action)
        return effect
    
    # Parse as simple action
    action = parse_action(text)
    effect.update(action)
    
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

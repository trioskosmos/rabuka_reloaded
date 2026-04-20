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
    ('控え室', 'discard'),  # Handle cases where verb might be different
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
    """Extract target (self/opponent/both)."""
    if ('自分の' in text and '相手の' in text) or '自分と相手の' in text:
        return 'both'
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
    """Split text into cost and effect parts."""
    if '：' in text:
        parts = text.split('：', 1)
        return parts[0].strip(), parts[1].strip()
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
    
    # Check for compound conditions (かつ)
    if COMPOUND_OPERATOR in text:
        parts = [p.strip() for p in text.split(COMPOUND_OPERATOR) if p.strip()]
        if len(parts) >= 2:
            parsed_conditions = [parse_condition(p) for p in parts]
            parsed_conditions = [c for c in parsed_conditions if c and c.get('type') != 'custom']
            if len(parsed_conditions) >= 2:
                compound = {
                    'type': 'compound',
                    'operator': 'and',
                    'conditions': parsed_conditions,
                    'text': text
                }
                target = extract_target(text)
                if target:
                    compound['target'] = target
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
    
    # Extract group
    group = extract_group(text)
    if group:
        condition['group'] = group
    
    # Extract group names from 『』 brackets
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
        # If group_names is present, it's a group-specific location count condition
        if group_names:
            condition['type'] = 'group_location_count_condition'
    elif cost_limit:
        condition['type'] = 'cost_limit_condition'
    elif group or group_names:
        condition['type'] = 'group_condition'
    elif location and card_type:
        condition['type'] = 'location_condition'
    else:
        condition['type'] = 'custom'
    
    return condition

def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text."""
    action = {
        'text': text,
    }
    
    # Extract per-unit modifier (につき)
    if PER_UNIT_MARKER in text:
        per_unit_match = re.search(r'([^につき]+)につき', text)
        if per_unit_match:
            action['per_unit'] = per_unit_match.group(1).strip()
    
    # Extract source - handle "手札を" pattern for discard
    if '手札を' in text and '控え室に置く' in text:
        action['source'] = 'hand'
    
    # Extract source
    source = extract_source(text)
    if source and 'source' not in action:
        action['source'] = source
    
    # Extract destination
    destination = extract_destination(text)
    if destination:
        action['destination'] = destination
    
    # Extract state change
    state_change = extract_state_change(text)
    if state_change:
        action['state_change'] = state_change
    
    # Extract count
    count = extract_count(text)
    if count:
        action['count'] = count
    
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
    
    # Extract optional flag
    if extract_optional(text):
        action['optional'] = True
    
    # Extract max flag
    if extract_max(text):
        action['max'] = True
    
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
    elif source and destination:
        action['action'] = 'move_cards'
    # Check for "加える" (add to) pattern - common for adding cards to hand
    elif '加える' in text or '加え' in text:
        action['action'] = 'move_cards'
        # If destination not already set, infer it
        if 'destination' not in action:
            if '手札に' in text:
                action['destination'] = 'hand'
            elif 'ステージに' in text:
                action['destination'] = 'stage'
            elif '控え室に' in text:
                action['destination'] = 'discard'
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
    elif '引く' in text:
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
    elif '得る' in text:
        action['action'] = 'gain_resource'
        if 'ハート' in text:
            action['resource'] = 'heart'
        elif 'ブレード' in text:
            action['resource'] = 'blade'
    elif 'スコアを' in text:
        action['action'] = 'modify_score'
        if '＋' in text:
            action['operation'] = 'add'
        elif '－' in text:
            action['operation'] = 'subtract'
    elif '必要ハート' in text and '少なくなる' in text:
        action['action'] = 'modify_required_hearts'
        action['operation'] = 'decrease'
    elif 'ポジションチェンジ' in text:
        action['action'] = 'position_change'
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
    else:
        action['action'] = 'custom'
    
    return action

# ============== MAIN PARSING FUNCTIONS ==============

def parse_cost(text: str) -> Dict[str, Any]:
    """Parse a cost text."""
    cost = {
        'text': text,
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
    
    # Determine cost type - check move_cards first
    if cost.get('source') and cost.get('destination'):
        cost['type'] = 'move_cards'
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
            if '起動できる' in note:
                effect['activation_condition'] = note
                break
    
    # Check for each-time triggers (たび)
    if EACH_TIME_MARKER in text:
        effect['trigger_type'] = 'each_time'
        # Extract the trigger event
        trigger_match = re.search(r'([^たび]+)たび', text)
        if trigger_match:
            effect['trigger_event'] = trigger_match.group(1).strip()
    
    # Check for implicit sequential (comma-separated actions) - check BEFORE specific action types
    if '、' in text and not any(marker in text for marker in CONDITION_MARKERS):
        parts = text.split('、')
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
    
    # Check for "その中から" (from among them) pattern - indicates look_at + select + action
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
            
            # Store conditional modifier if present
            if conditional_modifier:
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
        action = parse_action(action_text)
        effect['condition'] = condition
        effect.update(action)
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

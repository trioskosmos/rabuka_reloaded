"""Condition parsing for ability extraction."""
import re
from parser_utils import (
    annotate_tree,
    detect_group_type,
    extract_all_quoted_names,
    extract_group_name,
    extract_heart_types,
    extract_int,
    normalize_fullwidth_digits,
)


LOCATION_PATTERNS = [
    ('成功ライブカード置き場', 'success_live_card_zone'),
    ('エールにより公開された', 'cheer_revealed'),
    ('ライブカード置き場', 'live_card_zone'),
    ('控え室', 'discard'),
    ('エネルギー', 'energy'),
    ('手札', 'hand'),
]

CARD_TYPE_PATTERNS = [
    ('ブレードを持つカード', 'blade_card'),
    ('ライブカード', 'live_card'),
    ('メンバーカード', 'member_card'),
    ('メンバー', 'member_card'),
    ('エネルギーカード', 'energy_card'),
]

POSITION_PATTERNS = [
    ('センターエリア', 'center'),
    ('{{center.png|センター}}', 'center'),
    ('左サイドエリア', 'left_side'),
    ('【左サイド】', 'left_side'),
    ('右サイドエリア', 'right_side'),
    ('【右サイド】', 'right_side'),
    ('ドルチェストラエリア', 'dollchestra'),
    ('【ドルチェストラ】', 'dollchestra'),
]


def _extract_location(text):
    """Extract location from condition text."""
    for pattern, location in LOCATION_PATTERNS:
        if pattern in text:
            return location
    if 'ライブ中の' in text or 'ライブカード' in text:
        return 'live'
    return None


def _extract_card_type(text):
    """Extract card type from condition text."""
    for pattern, card_type in CARD_TYPE_PATTERNS:
        if pattern in text:
            return card_type
    return None


def _set_condition(condition, condition_type, *, value=None, operator=None, **extra):
    """Populate common condition fields and return the same dict."""
    condition['type'] = condition_type
    if value is not None:
        condition['value'] = value
    if operator is not None:
        condition['operator'] = operator
    # Allow None values for group field to preserve it even if not found
    condition.update({key: item for key, item in extra.items() if item is not None or key == 'group'})
    return condition


def _extract_target(text):
    if ('自分の' in text and '相手の' in text) or '自分と相手の' in text or '自分と相手' in text:
        return 'both'
    if '相手の' in text:
        return 'opponent'
    if '自分の' in text:
        return 'self'
    return None


def _extract_position_value(text):
    for pattern, position in POSITION_PATTERNS:
        if pattern in text or (pattern.startswith('【') and text.startswith(pattern)):
            return position
    return None


def _extract_count_value(text, pattern):
    return extract_int(pattern, text)


# Pattern registry: list of (matcher, condition_type, extractor_fn) tuples
# This replaces 100+ elif blocks with a data-driven approach
# matcher: string (substring check) or callable (returns truthy if matches)
# condition_type: string for the condition type
# extractor_fn: callable that extracts additional fields from text, or None
CONDITION_PATTERN_REGISTRY = [
    # COUNT_ENERGY: Active energy conditions (e.g., "アクティブ状態の自分のエネルギーがある場合")
    # Must come before general 'state' pattern to avoid conflict
    (lambda t: 'アクティブ状態の自分のエネルギー' in t and ('ある場合' in t or 'がある' in t),
     'count_energy', lambda t, m: {'location': 'energy', 'state': 'active', 'target': 'self', 'comparison': 'GE', 'value': 1}),
    (lambda t: 'アクティブ状態のエネルギー' in t and ('ある場合' in t or 'がある' in t),
     'count_energy', lambda t, m: {'location': 'energy', 'state': 'active', 'target': 'self', 'comparison': 'GE', 'value': 1}),
    # COUNT_STAGE with min_cost: Cost-based stage conditions (e.g., "コスト13以上のメンバーがいる場合")
    # Must come before general 'cost_at_least' pattern to avoid conflict
    (lambda t: re.search(r'コスト(\d+)以上のメンバー.*?いる', t) or re.search(r'コスト(\d+)以上.*?メンバーが.*?ステージ', t),
     'count_stage', lambda t, m: {'min_cost': int(m.group(1)), 'target': _extract_target(t) or 'self', 'location': 'stage',
                                 'comparison': 'GE', 'value': 1, 'card_type': 'member_card'}),
    (lambda t: re.search(r'コスト(\d+)以上.*?ステージ', t),
     'count_stage', lambda t, m: {'min_cost': int(m.group(1)), 'target': _extract_target(t) or 'self', 'location': 'stage',
                                 'comparison': 'GE', 'value': 1, 'card_type': 'member_card'}),
    # Energy conditions
    (lambda t: re.search(r'エネルギーが(\d+)枚以上', t), 'energy_at_least', lambda t, m: {'value': int(m.group(1))}),
    ('エネルギーが相手より少ない', 'energy_comparison', lambda t: {'operator': '<', 'target': 'opponent', 'compares': 'energy'}),
    ('自分のエネルギーが相手より少ない', 'energy_comparison', lambda t: {'operator': '<', 'target': 'opponent', 'compares': 'energy'}),
    # Card count conditions
    (lambda t: '枚以上' in t and re.search(r'(\d+)枚以上', t), 'card_count_at_least',
     lambda t, m: {'value': int(m.group(1)), 'location': _extract_location(t), 'card_type': _extract_card_type(t)}),
    # UNIQUE_NAMES_COUNT: Multi-name counting (e.g., "名前が異なるメンバーが3人以上いる場合")
    # Must come before general 'member_count_at_least' pattern to avoid conflict
    (lambda t: '名前が異なる' in t and re.search(r'(\d+)人以上', t),
     'unique_names_count', lambda t, m: {'value': int(m.group(1)), 'location': 'stage', 'target': 'self'}),
    (lambda t: '名前とコストが両方ともそれぞれ異なる' in t and re.search(r'(\d+)人以上', t),
     'unique_names_count', lambda t, m: {'value': int(m.group(1)), 'location': 'stage', 'target': 'self', 'unique_costs': True}),
    # UNIQUE_MEMBER_COSTS_COUNT: Cost uniqueness (e.g., "コストが異なるメンバーが3人以上いる場合")
    # Must come before general 'member_count_at_least' pattern to avoid conflict
    (lambda t: 'コストが異なる' in t and re.search(r'(\d+)人以上', t),
     'unique_member_costs_count', lambda t, m: {'value': int(m.group(1)), 'location': 'stage', 'target': 'self'}),
    # Member count conditions
    (lambda t: re.search(r'(\d+)人以上', t), 'member_count_at_least',
     lambda t, m: {'value': int(m.group(1))}),
    # Cost conditions
    (lambda t: re.search(r'コスト(\d+)以上のメンバー', t) and not 'ステージ' in t,
     'cost_at_least',
     lambda t, m: {'value': int(m.group(1)), 'cost_limit': int(m.group(1))}),
    # Energy conditions
    (lambda t: re.search(r'エネルギーが(\d+)枚以上', t), 'energy_at_least', lambda t, m: {'value': int(m.group(1))}),
    ('エネルギーが相手より少ない', 'energy_comparison', lambda t: {'operator': '<', 'target': 'opponent', 'compares': 'energy'}),
    ('自分のエネルギーが相手より少ない', 'energy_comparison', lambda t: {'operator': '<', 'target': 'opponent', 'compares': 'energy'}),
    # Card count conditions
    (lambda t: '枚以上' in t and re.search(r'(\d+)枚以上', t), 'card_count_at_least',
     lambda t, m: {'value': int(m.group(1)), 'location': _extract_location(t), 'card_type': _extract_card_type(t)}),
    # Member count conditions
    (lambda t: re.search(r'(\d+)人以上', t), 'member_count_at_least',
     lambda t, m: {'value': int(m.group(1))}),
    # Cost conditions (note: cost-based stage conditions handled by count_stage pattern above)
    # Score conditions
    (lambda t: re.search(r'スコアの合計が(\d+)以上', t), 'score_sum_at_least',
     lambda t, m: {'value': int(m.group(1)), 'location': 'success_live_card_zone' if '成功ライブカード置き場' in t else None}),
    ('スコアが相手より高い', 'score_comparison', lambda t: {'operator': '>', 'target': 'opponent'}),
    ('スコアが相手より低い', 'score_comparison', lambda t: {'operator': '<', 'target': 'opponent'}),
    # Hand count conditions
    (lambda t: re.search(r'手札が(\d+)枚以下', t), 'hand_card_count_at_most',
     lambda t, m: {'value': int(m.group(1)), 'location': 'hand'}),
    (lambda t: re.search(r'手札が(\d+)枚以上', t), 'hand_card_count_at_least',
     lambda t, m: {'value': int(m.group(1)), 'location': 'hand'}),
    # Deck refresh
    ('デッキがリフレッシュしていた', 'deck_refresh', None),
    # State conditions
    (lambda t: 'ウェイト状態の' in t, 'state', lambda t: {'value': 'wait', 'operator': '==', 'group': extract_group_name(t), 'card_type': 'member_card'}),
    # Note: 'かぎり' (while/as long as) marker - must come after state pattern to allow state conditions to match first
    # Note: 'アクティブ状態の' is handled by count_energy pattern for energy conditions
    # Area movement
    ('エリアを移動した', 'area_move', None),
    ('エリアを移動している', 'member_area_move', lambda t: {'target': 'self'}),
    # Deploy conditions
    (lambda t: re.search(r'(\d+)回登場した', t), 'member_deploy_count',
     lambda t, m: {'value': int(m.group(1)), 'target': 'self', 'source': 'stage', 'event': 'deploy',
                   'scope': 'turn' if 'このターン' in t else None}),
    (lambda t: '回以上登場している' in t and re.search(r'(\d+)回以上登場している', t), 'member_deploy_count',
     lambda t, m: {'value': int(m.group(1)), 'operator': '>=', 'target': 'self', 'source': 'stage', 'event': 'deploy',
                   'scope': 'turn' if 'このターン' in t else None}),
    # Special conditions
    (lambda t: '余剰ハート' in t, 'surplus_heart', lambda t: {
        'value': 0 if '持たない' in t else (_extract_count_value(t, r'余剰ハート.*?(\d+)つ以上') or 1),
        'type': 'surplus_heart_equal' if '持たない' in t else 'surplus_heart'
    }),
    ('登場か、エリアを移動した', 'or_trigger', lambda t: {'triggers': ['deploy', 'move']}),
    ('このメンバーがエリアを移動するか自分のエネルギー置き場にエネルギーが置かれた', 'or_trigger',
     lambda t: {'triggers': ['move_by_effect', 'energy_placed'], 'target': 'self'}),
    ('ステージから控え室に置かれた', 'move_to_waitroom_trigger',
     lambda t: {'operator': 'triggered', 'source': 'stage', 'destination': 'waitroom',
                'target': 'self' if 'このメンバー' in t else ('selected_member' if 'そのメンバー' in t else None)}),
    ('効果によってはアクティブにならない', 'cannot_become_active', None),
    ('センターエリアにいるメンバーが最も大きいコストを持つ', 'highest_cost_center', None),
    ('エリアすべてに', 'all_areas', lambda t: {'group': extract_group_name(t), 'names_different': '名前が異なる' in t}),
    # Opponent live cards location
    ('相手のライブカード置き場にあるすべてのライブカードは', 'opponent_live_cards_location', None),
    # Success live card count equality (must come before general card count patterns)
    (lambda t: '自分と相手の成功ライブカード置き場にあるカードの枚数が同じ' in t, 'success_live_count_equal_opponent', None),
    # Waitroom location
    ('自分の控え室にある', 'waitroom_location', None),
    # Cost comparison conditions
    (lambda t: re.search(r'コストの大きい|コストが高い', t), 'cost_comparison', lambda t: {'operator': '>'}),
    (lambda t: re.search(r'コストが低い', t), 'cost_comparison', lambda t: {'operator': '<'}),
    # Card count equal comparison
    ('カードの枚数が同じ', 'card_count', lambda t: {'operator': '=='}),
    ('枚数が同じ', 'card_count', lambda t: {'operator': '=='}),
    # Exact count conditions
    (lambda t: re.search(r'ちょうど(\d+)枚', t), 'exact_count',
     lambda t, m: {'value': int(m.group(1)), 'count_type': 'energy' if 'エネルギー' in t else ('heart' if 'ハート' in t else 'card')}),
    # Per-unit modifier
    (lambda t: 'につき' in t, 'per_unit',
     lambda t: {'value': _extract_count_value(t, r'(\d+)枚につき') or 1, 'operator': '*',
                'unit_type': ('energy_under_member' if 'このメンバーの下にあるエネルギーカード' in t
                             else 'wait_by_this_effect' if 'これによりウェイト状態にしたメンバー' in t
                             else 'energy_paid_by_this_effect' if 'これにより支払った' in t
                             else 'stage_member' if 'ステージにいる' in t
                             else 'live_card' if 'ライブカード置き場にある' in t
                             else 'success_live_card' if '成功ライブカード置き場にある' in t
                             else None)}),
    # Combined location count
    (lambda t: '自分と相手の' in t and '合計' in t, 'combined_location_count_at_least',
     lambda t, m: {'value': _extract_count_value(t, r'(\d+)枚以上'), 'location': _extract_location(t)}),
    # Heart member presence
    (lambda t: '{{heart_' in t and 'メンバーが' in t, 'heart_member_presence',
     lambda t: {'heart_types': extract_heart_types(t), 'presence': 'present' if 'いる' in t else 'absent',
                'target': _extract_target(t) or 'self'}),
    # Batôn touch deploy
    (lambda t: 'バトンタッチして登場' in t, 'baton_touch_deploy',
     lambda t: {'cost_comparison': 'lower' if 'コストが低い' in t or 'コストより低い' in t else None,
                'source_group': extract_group_name(t)}),
    # Comparison conditions with 'より'
    (lambda t: 'より' in t and 'につき' not in t and re.search(r'高い|低い|少ない|多い', t), 'comparison',
     lambda t: {'operator': '>' if '高い' in t or '多い' in t else '<',
                'compares': ('member_cost_total' if 'メンバーのコストの合計' in t
                            else 'live_total_score' if 'ライブの合計スコア' in t
                            else 'card_count' if 'カード枚数' in t
                            else 'heart_total' if 'ハートの総数' in t
                            else 'score_sum' if 'スコア' in t and '合計' in t
                            else 'hand_card_count' if '手札の枚数' in t
                            else 'energy_count' if 'エネルギー' in t
                            else 'cost' if 'コスト' in t
                            else None)}),
    # All revealed cards match pattern
    (lambda t: 'それらがすべて' in t and 'メンバーカード' in t, 'all_revealed_cards_match',
     lambda t: {'card_type': 'member_card', 'location': 'waitroom', 'target': 'self',
                'heart_types': extract_heart_types(t) if '{{heart_' in t else None,
                'reference': 'those_cards' if '公開したカード' in t or 'それらがすべて' in t else None,
                'cost_limit': (_extract_count_value(t, r'コスト(\d+)') or None) if 'コスト' in t else None}),
    # Heart total at least on members
    (lambda t: 'メンバーが持つハートに' in t and '合計' in t, 'heart_total_at_least',
     lambda t: {'value': _extract_count_value(t, r'(\d+)個以上') or _extract_count_value(t, r'(\d+)以上'),
                'location': 'stage', 'target': 'self', 'heart_types': extract_heart_types(t) if '{{heart_' in t else None,
                'group': extract_group_name(t) if '『' in t else None}),
    # Heart card presence in cheer revealed
    (lambda t: 'エールにより公開された自分の' in t and 'メンバーカードが持つハートの中に' in t and 'がある場合' in t,
     'heart_card_presence', lambda t: {'location': 'cheer_revealed', 'card_type': 'member_card', 'target': 'self',
                                        'presence': 'present', 'heart_types': extract_heart_types(t) if '{{heart_' in t else None,
                                        'group': extract_group_name(t) if '『' in t else None}),
    # COUNT_BLADES with threshold: Blade threshold conditions (e.g., "ブレードが6つ以上の場合")
    # Must come before general 'group' pattern to avoid conflict
    (lambda t: re.search(r'{{icon_blade.*?}}.*?(\d+)つ以上', t) or re.search(r'ブレード.*?(\d+)つ以上', t) or re.search(r'(\d+)つ以上.*?ブレード', t),
     'count_blades', lambda t, m: {'threshold': int(m.group(1)), 'target': 'self', 'comparison': 'GE', 'value': 1}),
]


def _apply_pattern_registry(condition, text):
    """Apply the pattern registry to extract condition type and fields."""
    for matcher, condition_type, extractor in CONDITION_PATTERN_REGISTRY:
        matched = False
        match_obj = None
        if callable(matcher) and not isinstance(matcher, str):
            match_obj = matcher(text)
            matched = bool(match_obj)
        elif isinstance(matcher, str):
            matched = matcher in text
        if matched:
            _set_condition(condition, condition_type)
            if extractor:
                if callable(extractor):
                    try:
                        fields = extractor(text, match_obj) if match_obj else extractor(text)
                        if fields:
                            # Allow None values for group field to preserve it even if not found
                            condition.update({k: v for k, v in fields.items() if v is not None or k == 'group'})
                            # Allow extractor to override condition type
                            if 'type' in fields:
                                condition['type'] = fields['type']
                    except Exception:
                        pass
            return True
    return False


def parse_condition(condition_part):
    """Parse condition part of conditional effect."""
    condition = {}
    condition_part = normalize_fullwidth_digits(condition_part).strip()
    condition_part = condition_part.lstrip('「『').rstrip('」』')
    condition['text'] = condition_part
    
    # Check for "その後、" (then) separator in condition text - MUST BE FIRST
    # This handles patterns like "activate energy. Then, if all energy is active, +1 score"
    if 'その後、' in condition_part:
        before_then, after_then = condition_part.split('その後、', 1)
        # Check if before_then ends with a period (is an action, not a condition)
        # If so, this should be split into two separate parts
        if before_then.rstrip('。 、').endswith('。'):
            # This is an action followed by a condition
            # Return a special marker to indicate this should be split
            condition['_sequential_marker'] = True
            condition['_before_then'] = before_then.rstrip('。')
            condition['_after_then'] = after_then
            return condition

    if '、' in condition_part and any(marker in condition_part for marker in ['とき', '場合', 'かぎり']):
        fragments = [frag.strip('。 ') for frag in condition_part.split('、') if frag.strip('。 ')]
        parsed_conditions = []
        for fragment in fragments:
            fragment_condition = parse_condition(fragment)
            if fragment_condition and fragment_condition.get('type') != 'raw':
                parsed_conditions.append(fragment_condition)
        if len(parsed_conditions) >= 2:
            compound = {'type': 'compound', 'conditions': parsed_conditions, 'text': condition_part}
            target = _extract_target(condition_part)
            if target:
                compound['target'] = target
            return annotate_tree(compound, condition_part)

    if 'かつ' in condition_part and 'バトンタッチ' in condition_part:
        fragments = [frag.strip('。 ') for frag in condition_part.split('かつ') if frag.strip('。 ')]
        parsed_conditions = []
        for fragment in fragments:
            fragment_condition = parse_condition(fragment)
            if fragment_condition and fragment_condition.get('type') != 'raw':
                parsed_conditions.append(fragment_condition)
        if len(parsed_conditions) >= 2:
            compound = {'type': 'compound', 'conditions': parsed_conditions, 'text': condition_part}
            target = _extract_target(condition_part)
            if target:
                compound['target'] = target
            return annotate_tree(compound, condition_part)

    if 'エネルギーが相手より少ない' in condition_part:
        _set_condition(condition, 'energy_comparison', operator='<', target='opponent', compares='energy')
        return annotate_tree(condition, condition_part)

    # Extract target if not already set
    if 'target' not in condition:
        target = _extract_target(condition_part)
        if target:
            condition['target'] = target

    # Check for movement-trigger conditions.
    if 'ステージから控え室に置かれた' in condition_part:
        _set_condition(
            condition,
            'move_to_waitroom_trigger',
            operator='triggered',
            source='stage',
            destination='waitroom',
        )
        if 'このメンバー' in condition_part:
            condition['target'] = 'self'
        elif 'そのメンバー' in condition_part:
            condition['target'] = 'selected_member'
        return condition
    
    # Check for activation restriction at the end of condition (e.g., "のみ起動できる")
    if 'のみ起動できる' in condition_part:
        condition['activation_restriction'] = 'only_this_card'
        condition_part = condition_part.replace('のみ起動できる', '').strip()
    elif 'のみ発動する' in condition_part:
        condition['activation_restriction'] = 'only_this_card'
        condition_part = condition_part.replace('のみ発動する', '').strip()
    
    # Check for different card names condition (e.g., "カード名が異なる")
    if 'カード名が異なる' in condition_part:
        condition['different'] = 'card_name'
    
    # Check for exclude_this_member pattern (e.g., "このメンバー以外")
    if 'このメンバー以外' in condition_part or 'exclude_this_member' in str(condition.get('exclude_this_member', '')):
        if 'type' not in condition:
            condition['type'] = 'member_exclusion'
    
    # Check for different group names condition (e.g., "グループ名が異なる")
    if 'グループ名が異なる' in condition_part:
        condition['different'] = 'group_name'

    # Check for different unit names condition (e.g., "ユニット名がそれぞれ異なる")
    if 'ユニット名がそれぞれ異なる' in condition_part or 'ユニット名が異なる' in condition_part:
        condition['different'] = 'unit_name'
    
    # Check for heart type selection with "のうち" (among) pattern (e.g., "{{heart_01}}か{{heart_03}}か{{heart_06}}のうち")
    if re.search(r'{{heart_\d+\.png.*?}}.*?のうち', condition_part):
        heart_matches = extract_heart_types(condition_part)
        if heart_matches:
            condition['heart_types'] = heart_matches
            condition['operator'] = 'or'
    
    # Apply pattern registry for common conditions (replaces many elif blocks)
    if not _apply_pattern_registry(condition, condition_part):
        # Fallback for conditions not in registry
        pass
    
    # Post-process: Extract group name if not already set and condition contains group markers
    if 'group' not in condition and '『' in condition_part:
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group
            condition['group_type'] = detect_group_type(group)
    
    # Check for member count condition modifiers
    if condition.get('type') == 'member_count_at_least':
        if '名前とコストが両方ともそれぞれ異なる' in condition_part:
            condition['different_name'] = True
            condition['different_cost'] = True
        elif '名前の異なる' in condition_part:
            condition['different_name'] = True
        elif 'コストの異なる' in condition_part:
            condition['different_cost'] = True
    
    # Check for per-unit modifier (～につき)
    if 'につき' in condition_part:
        _set_condition(
            condition,
            'per_unit',
            value=_extract_count_value(condition_part, r'(\d+)枚につき') or 1,
            operator='*',
        )
        
        # Check for specific per-unit contexts
        if 'このメンバーの下にあるエネルギーカード' in condition_part:
            condition['unit_type'] = 'energy_under_member'
        elif 'これによりウェイト状態にしたメンバー' in condition_part:
            condition['unit_type'] = 'wait_by_this_effect'
        elif 'これにより支払った' in condition_part:
            condition['unit_type'] = 'energy_paid_by_this_effect'
        elif 'ステージにいる' in condition_part:
            condition['unit_type'] = 'stage_member'
        elif 'ライブカード置き場にある' in condition_part:
            condition['unit_type'] = 'live_card'
        elif '成功ライブカード置き場にある' in condition_part:
            condition['unit_type'] = 'success_live_card'
        card_type = _extract_card_type(condition_part)
        if card_type:
            condition['card_type'] = card_type
        location = _extract_location(condition_part)
        if location:
            condition['location'] = location
        elif '控え室' in condition_part:
            condition['location'] = 'waitroom'
        
        # Check for target and state modifiers
        target = _extract_target(condition_part)
        if target in {'opponent', 'self'}:
            condition['target'] = target
        
        if 'ウェイト状態の' in condition_part:
            condition['state'] = 'wait'
        elif 'アクティブ状態の' in condition_part:
            condition['state'] = 'active'
        
        # Extract group name if present
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group
            condition['group_type'] = detect_group_type(group)
        
        # Check for exclusion modifiers
        if 'このメンバー以外の' in condition_part:
            condition['exclusion'] = 'this_member'
        elif 'ほかの' in condition_part:
            condition['exclusion'] = 'other'
        elif '名前の異なる' in condition_part:
            condition['exclusion'] = 'different_name'
    
    # Check for group condition (special handling for group extraction)
    if '『' in condition_part and '』' in condition_part:
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group
            condition['group_type'] = detect_group_type(group)
            # Extract additional fields that may be present with group condition
            position = _extract_position_value(condition_part)
            if position:
                condition['position_requirement'] = position
            card_type = _extract_card_type(condition_part)
            if card_type:
                condition['card_type'] = card_type
            # Extract cost limit (e.g., "コスト9以上")
            cost_limit_match = re.search(r'コスト(\d+)以上', condition_part)
            if cost_limit_match:
                condition['cost_limit'] = int(cost_limit_match.group(1))
            # Extract location
            location = _extract_location(condition_part)
            if location:
                condition['location'] = location

    # Check for heart-icon member presence conditions (special handling for value extraction).
    if condition.get('type') == 'heart_member_presence':
        heart_types = extract_heart_types(condition_part)
        if heart_types:
            condition['type'] = 'heart_member_presence'
            condition['heart_types'] = heart_types
            condition['presence'] = 'present' if 'いる' in condition_part else 'absent'
            condition['target'] = _extract_target(condition_part) or 'self'
            heart_value = _extract_count_value(condition_part, r'(\d+)つ以上')
            if heart_value:
                condition['heart_count'] = heart_value
            location = _extract_location(condition_part)
            if location:
                condition['location'] = location
    
    # Check for baton touch deployment condition
    elif 'バトンタッチして登場した' in condition_part or 'バトンタッチして登場している' in condition_part or 'バトンタッチして登場しており' in condition_part or 'バトンタッチしていた場合' in condition_part:
        condition['type'] = 'baton_touch_deploy'
        
        # Extract source member detail
        if '能力を持たないメンバーから' in condition_part:
            condition['source_member'] = 'no_ability'
        count_match = re.search(r'(\d+)人からバトンタッチ', condition_part)
        if count_match:
            condition['source_member_count'] = int(count_match.group(1))
        
        # Extract character name from quotes
        char_match = re.search(r'「(.+?)」からバトンタッチ', condition_part)
        if char_match:
            condition['source_character'] = char_match.group(1)
        
        # Extract cost comparison if present
        cost_match = re.search(r'コストが(低い|高い)の', condition_part)
        if cost_match:
            condition['cost_comparison'] = cost_match.group(1)
        
        # Extract group if present
        source_group = extract_group_name(condition_part)
        if not source_group:
            group_match = re.search(r'([^』]+)』のメンバー', condition_part)
            if group_match:
                source_group = group_match.group(1)
        if source_group:
            condition['source_group'] = source_group
        if '登場しており' in condition_part:
            condition['ongoing'] = True
        if 'コストが低い' in condition_part or 'コストより低い' in condition_part:
            condition['cost_comparison'] = 'lower'
        elif 'コストが大きい' in condition_part or 'コストより大きい' in condition_part:
            condition['cost_comparison'] = 'higher'
    
    # Check for position markers at the beginning of text
    if condition_part.startswith('【左サイド】'):
        _set_condition(condition, 'position', value='left_side', operator='==')
        return condition
    elif condition_part.startswith('【右サイド】'):
        _set_condition(condition, 'position', value='right_side', operator='==')
        return condition
    elif condition_part.startswith('{{center.png|センター}}'):
        _set_condition(condition, 'position', value='center', operator='==', position_requirement='center')
        return condition
    
    # Check for center + group condition (position + condition structure)
    elif '{{center.png|センター}}' in condition_part and 'のメンバーと' in condition_part:
        _set_condition(condition, 'group', value=extract_group_name(condition_part))
        # Mark that position is required
        condition['position_required'] = 'center'
        condition['position_requirement'] = 'center'
    
    # Check for character names in quotes (individual members)
    char_names = extract_all_quoted_names(condition_part)
    if char_names:
        condition['type'] = 'character_presence'
        condition['characters'] = char_names
        condition['presence'] = 'present' if 'いる' in condition_part or '登場' in condition_part else 'absent'
        if '自分のステージに' in condition_part:
            condition['target'] = 'self'
        elif '相手のステージに' in condition_part:
            condition['target'] = 'opponent'
        return condition
    
    # Check for heart count condition (e.g., "必要ハートに{{heart_xx}}を3以上含む")
    heart_count_match = re.search(r'必要ハートに.*?を(\d+)以上含む', condition_part)
    if heart_count_match:
        _set_condition(condition, 'heart_count_at_least', value=int(heart_count_match.group(1)))
        return condition
    
    # Check for blade count condition (e.g., "元々持つ{{icon_blade.png|ブレード}}の数が1つ以下")
    blade_count_match = re.search(r'元々持つ.*?ブレード.*?の数が(\d+)以下', condition_part)
    if blade_count_match:
        _set_condition(condition, 'blade_count_at_most', value=int(blade_count_match.group(1)))
        return condition
    
    # Check for member presence/absence conditions
    elif re.search(r'(自分|相手|自分と相手)のステージに.*?メンバーが(いる|いない)', condition_part):
        condition['type'] = 'heart_member_presence' if '{{heart_' in condition_part else 'member_presence'
        if 'いない' in condition_part:
            condition['presence'] = 'absent'
        # Extract position, cost, and card_type if present
        position = _extract_position_value(condition_part)
        if position:
            condition['position_requirement'] = position
        cost_limit_match = re.search(r'コスト(\d+)以上', condition_part)
        if cost_limit_match:
            condition['cost_limit'] = int(cost_limit_match.group(1))
        card_type = _extract_card_type(condition_part)
        if card_type:
            condition['card_type'] = card_type
        else:
            condition['presence'] = 'present'
        condition['target'] = _extract_target(condition_part) or 'self'
        
        # Extract cost requirement
        cost_match = re.search(r'コスト(\d+)以上のメンバー', condition_part)
        if cost_match:
            condition['cost'] = int(cost_match.group(1))
        
        # Extract heart count requirement
        heart_match = re.search(r'heart\d+.*?(\d+)つ以上', condition_part)
        if heart_match:
            condition['heart_count'] = int(heart_match.group(1))
        if '{{heart_' in condition_part:
            heart_types = extract_heart_types(condition_part)
            if heart_types:
                condition['heart_types'] = heart_types
        
        # Extract exclusion details
        if 'このメンバー以外の' in condition_part:
            condition['exclusion'] = 'this_member'
        elif 'ほかの' in condition_part:
            condition['exclusion'] = 'other'
    
    # Check for card count conditions
    if re.search(r'(カード|メンバーカード|ライブカード)が(\d+)枚以上', condition_part):
        count_value = _extract_count_value(condition_part, r'(\d+)枚以上')
        if count_value:
            _set_condition(condition, 'card_count_at_least', value=count_value)
        
        # Extract card type
        card_type = _extract_card_type(condition_part)
        if card_type:
            condition['card_type'] = card_type
        
        # Extract location
        if '控え室' in condition_part:
            condition['location'] = 'waitroom'
        
        # Extract position if present
        position = _extract_position_value(condition_part)
        if position:
            condition['position_requirement'] = position
    
    # Check for cost conditions
    elif re.search(r'コスト(\d+)以上のメンバー', condition_part):
        cost_value = _extract_count_value(condition_part, r'コスト(\d+)以上')
        if cost_value:
            _set_condition(condition, 'cost_at_least', value=cost_value)
            condition['cost_limit'] = cost_value
            # Remove cost field if it was set by previous logic
            if 'cost' in condition and isinstance(condition['cost'], int):
                del condition['cost']
        
        # Extract position if present
        position = _extract_position_value(condition_part)
        if position:
            condition['position_requirement'] = position
        
        # Extract card type
        card_type = _extract_card_type(condition_part)
        if card_type:
            condition['card_type'] = card_type
    
    # Check for score sum conditions
    elif re.search(r'スコアの合計が(\d+)以上', condition_part):
        score_value = _extract_count_value(condition_part, r'(\d+)以上')
        if score_value:
            location = 'success_live_card_zone' if '成功ライブカード置き場にあるカード' in condition_part else ('live_total' if 'ライブの合計スコア' in condition_part else None)
            if location == 'live_total':
                _set_condition(condition, 'live_total_score_at_least', value=score_value)
            else:
                _set_condition(condition, 'score_sum_at_least', value=score_value, location=location)
    
    # Check for opponent score comparison conditions
    elif 'スコア' in condition_part and '相手より' in condition_part:
        if '合計スコアが相手より高い' in condition_part:
            _set_condition(condition, 'score_comparison', operator='>', target='opponent', compares='live_total_score')
        elif '高い' in condition_part:
            _set_condition(condition, 'score_comparison', operator='>', target='opponent')
        elif '低い' in condition_part:
            _set_condition(condition, 'score_comparison', operator='<', target='opponent')
    
    # Check for hand card count conditions
    elif re.search(r'手札が(\d+)枚以下', condition_part) or re.search(r'手札の枚数が(\d+)枚以下', condition_part):
        count_value = _extract_count_value(condition_part, r'(\d+)枚以下')
        if count_value is not None:
            _set_condition(condition, 'hand_card_count_at_most', value=count_value, location='hand')
    
    elif re.search(r'手札が(\d+)枚以上', condition_part) or re.search(r'手札の枚数が(\d+)枚以上', condition_part):
        count_value = _extract_count_value(condition_part, r'(\d+)枚以上')
        if count_value is not None:
            _set_condition(condition, 'hand_card_count_at_least', value=count_value, location='hand')
    
    elif re.search(r'手札の枚数が', condition_part):
        if 'より2枚以上多い' in condition_part:
            _set_condition(condition, 'hand_card_count_at_least_2_more', value=2, location='hand')
        elif 'より多い' in condition_part:
            condition['type'] = 'hand_card_count_greater_than'
    
    # Check for energy comparison conditions
    elif re.search(r'エネルギーが.*?より', condition_part):
        if '多い' in condition_part:
            condition['type'] = 'energy_count_greater_than'
        elif '低い' in condition_part:
            condition['type'] = 'energy_count_less_than'
    
    # Check for position conditions
    elif re.search(r'ステージの.*?エリアに(いる|登場している)', condition_part):
        _set_condition(condition, 'position', value=_extract_position_value(condition_part))
    
    # Check for answer conditions
    elif '回答が' in condition_part:
        condition['type'] = 'answer'
        
        # Extract answer values
        if 'か' in condition_part:
            # Split by 'か' to get multiple answers
            answers_text = condition_part.replace('回答が', '').replace('の場合', '').strip()
            answers = answers_text.split('か')
            # Strip trailing particles from each answer
            condition['answers'] = [a.rstrip('の').strip() for a in answers]
        else:
            # Single answer - extract the word between "回答が" and "の場合"
            # Remove "回答が" prefix and "の場合" suffix
            answer_text = condition_part.replace('回答が', '').replace('の場合', '').strip()
            if answer_text:
                # Strip trailing particles
                condition['answers'] = [answer_text.rstrip('の').strip()]
    
    # Check for trigger condition (ライブ成功時能力が解決するたび)
    elif '{{live_success.png|ライブ成功時}}能力が解決するたび' in condition_part:
        _set_condition(condition, 'live_success_trigger', trigger_type='live_success')
    elif '{{live_success.png|ライブ成功時}}' in condition_part and 'エールにより公開された自分のカードの中にライブカードが' in condition_part:
        count_value = _extract_count_value(condition_part, r'(\d+)枚以上')
        if count_value:
            _set_condition(
                condition,
                'card_count_at_least',
                value=count_value,
                location='cheer_revealed',
                card_type='live_card',
                trigger_type='live_success',
            )

    # Check for visibility conditions
    elif '自分のエールによって公開されている場合のみ発動する' in condition_part:
        _set_condition(condition, 'visibility', visibility='cheer_revealed_self')
    elif 'このカードが自分のエールによって公開されている場合のみ発動する' in condition_part:
        _set_condition(condition, 'visibility', visibility='cheer_revealed_self')

    # Check for discarded-card conditions
    elif 'これにより控え室に置いたカードが' in condition_part and 'の場合' in condition_part:
        group = extract_group_name(condition_part)
        if group:
            _set_condition(condition, 'discarded_card_group', value=group)
        elif 'メンバーカード' in condition_part:
            _set_condition(condition, 'discarded_card', value='member_card')
    elif 'これにより控え室に置いたカードがメンバーカード' in condition_part:
        _set_condition(condition, 'discarded_card', value='member_card')
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group
    
    # Check for "これにより無効にした場合" (if invalidated by this action) pattern
    elif 'これにより無効にした場合' in condition_part:
        _set_condition(condition, 'invalidation_happened', type='conditional_check')

    # Check for selection-target conditions
    elif '自分のステージにいるの' in condition_part and 'メンバー1人は' in condition_part:
        group = extract_group_name(condition_part)
        _set_condition(condition, 'member_selection', target='self')
        if group:
            condition['group'] = group
        condition['selection'] = 'selected_member'
    
    # Check for opponent live cards location condition
    elif '相手のライブカード置き場にあるすべてのライブカードは' in condition_part:
        _set_condition(condition, 'opponent_live_cards', operator='present')
    
    # Check for move action target condition (自分のステージにいるメンバーを)
    elif '自分のステージにいるメンバーを' in condition_part and '移動' not in condition_part:
        _set_condition(condition, 'stage_members_target', operator='present')
    
    # Check for blade count conditions
    elif re.search(r'ブレード.*?合計が(\d+)以上', condition_part):
        blade_value = _extract_count_value(condition_part, r'(\d+)以上')
        if blade_value:
            _set_condition(condition, 'blade_count_at_least', value=blade_value)
    
    # Check for heart count conditions
    elif re.search(r'ハート.*?(\d+)つ以上', condition_part):
        heart_value = _extract_count_value(condition_part, r'(\d+)つ以上')
        if heart_value:
            _set_condition(condition, 'heart_count_at_least', value=heart_value)
    
    # Check for state conditions
    elif 'ウェイト状態の' in condition_part:
        _set_condition(condition, 'state', value='wait', operator='==')
    elif 'アクティブ状態の' in condition_part:
        _set_condition(condition, 'state', value='active', operator='==')
    
    # Check for card score conditions
    elif re.search(r'このカードのスコアが(\d+)', condition_part):
        score_value = _extract_count_value(condition_part, r'(\d+)')
        if score_value:
            _set_condition(condition, 'card_score', value=score_value, operator='==')
    
    # Check for combined location conditions (自分と相手の～)
    elif '自分と相手の' in condition_part and '合計' in condition_part:
        _set_condition(
            condition,
            'combined_location_count_at_least',
            value=_extract_count_value(condition_part, r'(\d+)枚以上'),
            location=_extract_location(condition_part),
        )

    # Check for explicit energy comparison against the opponent before generic comparison parsing.
    elif '自分のエネルギーが相手より少ない' in condition_part:
        _set_condition(condition, 'energy_comparison', operator='<', target='opponent', compares='energy')

    # Check for explicit self-vs-opponent member cost comparison.
    elif '自分の' in condition_part and '相手の' in condition_part and 'メンバーのコストが' in condition_part:
        if '高い' in condition_part:
            _set_condition(
                condition,
                'cost_comparison',
                operator='>',
                compares='cost',
                target='self',
                comparison_target='opponent',
            )
        elif '低い' in condition_part:
            _set_condition(
                condition,
                'cost_comparison',
                operator='<',
                compares='cost',
                target='self',
                comparison_target='opponent',
            )
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group

    # Check for discarded live-card follow-up conditions.
    elif 'これによりライブカードを控え室に置いた場合' in condition_part:
        _set_condition(
            condition,
            'discarded_card',
            value='live_card',
            location='waitroom',
            target='self',
        )

    # Check for turn-based movement absence.
    elif 'このターンにこのメンバーが移動していない' in condition_part:
        _set_condition(condition, 'movement_state', value='not_moved_this_turn', target='self')

    # Check for success-live-card zone location.
    elif 'このカードが自分の成功ライブカード置き場にある' in condition_part:
        _set_condition(condition, 'location', value='success_live_card_zone', target='self')

    # Check for costed deployment triggers.
    elif ('自分のステージにコスト' in condition_part or '自分のステージにのコスト' in condition_part) and 'メンバーが登場した' in condition_part:
        cost_match = re.search(r'コスト(\d+)', condition_part)
        if cost_match:
            _set_condition(
                condition,
                'member_deploy_cost',
                value=int(cost_match.group(1)),
                target='self',
                location='stage',
            )

    # Check for energy-payment condition.
    elif '{{icon_energy' in condition_part and '支払わない' in condition_part:
        _set_condition(
            condition,
            'pay_energy_condition',
            value=condition_part.count('{{icon_energy.png|E}}'),
            target='self',
        )

    # Check for explicit wait-state membership.
    elif 'このメンバーがウェイト状態である' in condition_part:
        _set_condition(condition, 'state', value='wait', operator='==', target='self')

    # Check for active-to-wait state changes.
    elif 'アクティブ状態からウェイト状態になった' in condition_part:
        _set_condition(
            condition,
            'state_change',
            from_state='active',
            to_state='wait',
            target='self',
        )

    # Check for cheer action trigger.
    elif '自分がエールした' in condition_part:
        _set_condition(condition, 'cheer_action', target='self')

    # Check for first turn live phase.
    elif 'このゲームの1ターン目のライブフェイズの' in condition_part:
        _set_condition(condition, 'turn_phase', value='live_phase', target='self')
        condition['turn'] = 1

    # Check for face-up live card zone placement.
    elif 'このカードが表向きでライブカード置き場に置かれた' in condition_part:
        _set_condition(condition, 'location', value='live_card_zone', target='self')
        condition['face_up'] = True

    # Check for revealed top-card member-card pattern.
    elif '自分のデッキの一番上のカードを公開する' in condition_part and 'メンバーカード' in condition_part:
        _set_condition(condition, 'reveal_cards', source='deck_top', target='self', card_type='member_card')

    # Check for live-card-zone heart total conditions.
    elif 'ライブカード置き場にあるカードの必要ハートに含まれる' in condition_part and '合計が' in condition_part:
        heart_types = extract_heart_types(condition_part)
        value = _extract_count_value(condition_part, r'(\d+)以上')
        _set_condition(
            condition,
            'heart_total_at_least',
            value=value,
            location='live_card_zone',
            target='self',
        )
        if heart_types:
            condition['heart_types'] = heart_types

    # Check for live-start trigger resolution timing.
    elif 'ライブ開始時能力が解決するたび' in condition_part or '{{live_start.png|ライブ開始時}}能力が解決するたび' in condition_part:
        _set_condition(condition, 'live_start_trigger', trigger_type='live_start', target='self')

    # Check for heart-set requirement in live-card zones.
    elif '必要ハートの中に' in condition_part and 'それぞれ1以上含まれる' in condition_part:
        heart_types = extract_heart_types(condition_part)
        _set_condition(
            condition,
            'heart_set_requirement',
            value=1,
            location='live_card_zone',
            target='self',
        )
        if heart_types:
            condition['heart_types'] = heart_types

    # Check for blade-heart absence on a discarded member card.
    elif 'ブレードハートを持たないメンバーカードが自分のライブカード置き場から控え室に置かれている' in condition_part:
        _set_condition(
            condition,
            'card_discard_condition',
            card_type='member_card',
            source='live_card_zone',
            destination='waitroom',
            exclusion='blade_heart',
            target='self',
        )

    # Check for area-selection movement condition.
    elif 'このメンバーがいるエリアとは別の自分のエリア1つを選ぶ' in condition_part:
        _set_condition(
            condition,
            'area_selection_condition',
            target='self',
            destination='selected_area',
            follow_up='selected_area_occupied' if '選んだエリアにメンバーがいる' in condition_part else None,
        )

    # Check for opponent optional discard conditions.
    elif '相手は手札からライブカードを1枚控え室に置いてもよい' in condition_part:
        _set_condition(
            condition,
            'opponent_optional_discard',
            source='hand',
            card_type='live_card',
            destination='waitroom',
            target='opponent',
            optional=True,
            follow_up='opponent_did_not_act' if 'そうしなかった' in condition_part else None,
        )
    elif '相手は手札を1枚控え室に置いてもよい' in condition_part:
        _set_condition(
            condition,
            'opponent_optional_discard',
            source='hand',
            card_type='card',
            destination='waitroom',
            target='opponent',
            optional=True,
            follow_up='opponent_did_not_act' if 'そうしなかった' in condition_part else None,
        )

    # Check for baton-touch cost comparison conditions before the generic comparison parser.
    elif 'バトンタッチして登場' in condition_part and ('コストが低い' in condition_part or 'コストより低い' in condition_part):
        condition['type'] = 'baton_touch_deploy'
        condition['cost_comparison'] = 'lower'
        condition['ongoing'] = '登場しており' in condition_part
        source_group = extract_group_name(condition_part)
        if not source_group:
            group_match = re.search(r'([^』]+)』のメンバー', condition_part)
            if group_match:
                source_group = group_match.group(1)
        if source_group:
            condition['source_group'] = source_group

    # Check for cheer-revealed member-card heart presence conditions before generic comparison parsing.
    elif 'エールにより公開された自分の' in condition_part and 'メンバーカードが持つハートの中に' in condition_part and 'がある場合' in condition_part:
        _set_condition(
            condition,
            'heart_card_presence',
            location='cheer_revealed',
            card_type='member_card',
            target='self',
            presence='present',
        )
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group
        heart_types = extract_heart_types(condition_part)
        if heart_types:
            condition['heart_types'] = heart_types

    # Check for the "これによりライブカードを控え室に置いた場合" follow-up.
    elif 'これによりライブカードを控え室に置いた場合' in condition_part:
        _set_condition(
            condition,
            'discarded_card',
            value='live_card',
            location='waitroom',
            target='self',
        )

    # Check for explicit cost comparison with a named group.
    elif 'メンバーのコストが' in condition_part and 'より高い' in condition_part:
        _set_condition(condition, 'cost_comparison', operator='>', compares='cost', target='self')
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group
    elif 'メンバーのコストが' in condition_part and 'より低い' in condition_part:
        _set_condition(condition, 'cost_comparison', operator='<', compares='cost', target='self')
        group = extract_group_name(condition_part)
        if group:
            condition['group'] = group

    # Check for activate_ability pattern (発動させる) - must be before generic comparison check
    elif '発動させる' in condition_part:
        # This is not a comparison condition, it's an action
        # Return early to prevent it from being parsed as comparison
        condition['type'] = 'raw'
        return annotate_tree(condition, condition_part)
    
    # Check for comparison conditions (～より～)
    elif 'より' in condition_part and 'につき' not in condition_part:
        if '高い' in condition_part:
            condition['type'] = 'comparison'
            condition['operator'] = '>'
        elif '低い' in condition_part or '少ない' in condition_part:
            condition['type'] = 'comparison'
            condition['operator'] = '<'
        elif '多い' in condition_part:
            condition['type'] = 'comparison'
            condition['operator'] = '>'
        else:
            condition['type'] = 'comparison'
            condition['operator'] = '>'
        
        # Extract what's being compared
        if 'メンバーのコストの合計' in condition_part:
            condition['compares'] = 'member_cost_total'
        elif 'ライブの合計スコア' in condition_part:
            condition['compares'] = 'live_total_score'
        elif 'カード枚数' in condition_part:
            condition['compares'] = 'card_count'
        elif 'ハートの総数' in condition_part:
            condition['compares'] = 'heart_total'
        elif 'エールにより公開された自分のカードの枚数が、エールにより公開された相手のカードの枚数' in condition_part:
            condition['compares'] = 'cheer_revealed_card_count'
        elif 'エールにより公開されている自分のライブカードの枚数が、エールにより公開されている相手のライブカードの枚数' in condition_part:
            condition['compares'] = 'cheer_revealed_live_card_count'
        elif 'スコア' in condition_part and '合計' in condition_part:
            condition['compares'] = 'score_sum'
        elif '手札の枚数' in condition_part:
            condition['compares'] = 'hand_card_count'
        elif 'エネルギー' in condition_part:
            condition['compares'] = 'energy_count'
        elif '元々持つ{{icon_blade.png|ブレード}}の数' in condition_part:
            condition['compares'] = 'original_blade_count'
        elif 'コスト' in condition_part:
            condition['compares'] = 'cost'
        elif 'ハートを持つ' in condition_part and 'より' in condition_part:
            condition['compares'] = 'member_heart_count'
        elif '枚数が' in condition_part and 'より' in condition_part:
            condition['compares'] = 'card_count_comparison'

        value = condition.get('value')
        if isinstance(value, str) and not re.fullmatch(r'[\d０-９]+', value):
            if 'group' not in condition:
                group = extract_group_name(condition_part)
                if group:
                    condition['group'] = group
            condition.pop('value', None)
        
        # Extract location if applicable
        location = _extract_location(condition_part)
        if location:
            condition['location'] = location

    # Check for success-live-card zone location.
    elif 'このカードが自分の成功ライブカード置き場にある' in condition_part:
        _set_condition(condition, 'location', value='success_live_card_zone', target='self')

    # Check for "このターンにこのメンバーが移動していない" state.
    elif 'このターンにこのメンバーが移動していない' in condition_part:
        _set_condition(condition, 'movement_state', value='not_moved_this_turn', target='self')

    # Check for energy comparison against opponent.
    elif '自分のエネルギーが相手より少ない' in condition_part:
        _set_condition(condition, 'energy_comparison', operator='<', target='opponent', compares='energy')

    # Check for costed deployment triggers.
    elif '自分のステージにコスト' in condition_part and 'メンバーが登場した' in condition_part:
        cost_match = re.search(r'コスト(\d+)', condition_part)
        if cost_match:
            _set_condition(
                condition,
                'member_deploy_cost',
                value=int(cost_match.group(1)),
                target='self',
                location='stage',
            )

    # Check for simple card presence condition (～カードがある)
    elif 'カードがある' in condition_part:
        _set_condition(condition, 'card_presence', operator='present', location=_extract_location(condition_part))

    if condition.get('type') in {'comparison', 'cost_comparison'}:
        if 'ブレードハート' in condition_part:
            condition['type'] = 'blade_heart_presence'
            condition['presence'] = 'present'
            condition['target'] = _extract_target(condition_part) or 'self'
            condition.pop('operator', None)
            condition.pop('compares', None)
            condition.pop('value', None)
        elif 'エールにより公開された自分の' in condition_part and 'ハートの中に' in condition_part:
            condition['type'] = 'heart_card_presence'
            condition['location'] = 'cheer_revealed'
            condition['card_type'] = 'member_card'
            condition['target'] = 'self'
            condition['presence'] = 'present'
            group = extract_group_name(condition_part)
            if group:
                condition['group'] = group
            heart_types = extract_heart_types(condition_part)
            if heart_types:
                condition['heart_types'] = heart_types
            condition.pop('operator', None)
            condition.pop('compares', None)
            condition.pop('value', None)
        elif 'メンバーのコストが' in condition_part:
            condition['type'] = 'cost_comparison'
            condition['compares'] = 'cost'
            condition['target'] = _extract_target(condition_part) or 'self'
            group = extract_group_name(condition_part)
            if group:
                condition['group'] = group
            value = condition.get('value')
            if isinstance(value, str) and not re.fullmatch(r'[\d０-９]+', value):
                condition.pop('value', None)
        elif condition.get('type') == 'cost_comparison':
            value = condition.get('value')
            if isinstance(value, str) and not re.fullmatch(r'[\d０-９]+', value):
                group = extract_group_name(condition_part)
                if group:
                    condition['group'] = group
                condition.pop('value', None)

    # Default: raw condition if no type was set
    if 'type' not in condition:
        # Check if this is an exclude_this_member condition
        if condition.get('exclude_this_member') or 'このメンバー以外' in condition_part:
            condition['type'] = 'member_exclusion'
        # Check if this is a "かぎり" (while) condition
        elif 'かぎり' in condition_part:
            condition['type'] = 'while'
        else:
            condition['type'] = 'raw'
            # Only set text if not already set (preserve original text)
            if 'text' not in condition:
                condition['text'] = condition_part
    
    # Global exclusion check - run for ALL condition types to ensure 以外 is captured
    if 'exclusion' not in condition:
        if 'このメンバー以外の' in condition_part or 'このメンバー以外' in condition_part:
            condition['exclusion'] = 'this_member'
        elif '以外の' in condition_part:
            # Try to extract what's being excluded
            exclusion_match = re.search(r'([^「『]+?)以外の', condition_part)
            if exclusion_match:
                exclusion = exclusion_match.group(1)
                # Remove trailing brackets if present
                exclusion = exclusion.rstrip('」』')
                condition['exclusion'] = exclusion
        elif '以外' in condition_part:
            # Generic exclusion marker
            condition['exclusion'] = 'specified'
    
    # Global negation check for 持たない (does not have) patterns
    if '持たない' in condition_part and 'negate' not in condition:
        if condition.get('type') == 'card_presence':
            # For card presence, set operator to absent
            if 'operator' not in condition or condition['operator'] == 'present':
                condition['operator'] = 'absent'
        elif condition.get('type') == 'surplus_heart':
            # For surplus_heart, change type to surplus_heart_equal with value 0
            condition['type'] = 'surplus_heart_equal'
            condition['value'] = 0
        else:
            # Add explicit negation marker
            condition['negate'] = True
    
    # Global negation check for ない場合 (if not) patterns in presence conditions
    if 'ない場合' in condition_part and condition.get('type') in ['card_presence', 'member_presence']:
        if 'presence' not in condition:
            condition['presence'] = 'absent'
    
    return annotate_tree(condition, condition_part)

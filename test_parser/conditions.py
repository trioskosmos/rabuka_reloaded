"""Condition handler cascade — ALL handlers from parser.py with explicit priority."""

from __future__ import annotations
import re
from typing import Any, Dict, Optional
from dispatcher import Rule, registry

# ------------------------------------------------------------------
# Individual condition handlers (pure: text → Optional[Dict])
# ------------------------------------------------------------------

def _complex(text):
    """これにより〜場合 — complex cause-effect."""
    if 'これにより' not in text:
        return None
    parts = text.split('これにより', 1)
    cause_text = parts[0].strip()
    effect_text = parts[1].strip()
    if cause_text and not effect_text.startswith('場合'):
        return {'type': 'complex_condition', 'cause': parse_condition(cause_text),
                'effect': {'text': effect_text, 'type': 'custom'}, 'text': text}
    return None

def _compound(text):
    """かつ/あり、 — compound conditions."""
    if 'かつ' not in text and 'あり、' not in text:
        return None
    op = 'かつ' if 'かつ' in text else 'あり、'
    parts = [p.strip() for p in text.split(op) if p.strip()]
    if len(parts) < 2:
        return None
    parsed = [parse_condition(p) for p in parts]
    result = {'type': 'compound', 'operator': 'and', 'conditions': parsed, 'text': text}
    for sub in parsed:
        if sub.get('distinct'):
            result['distinct'] = True
            break
    for kw in ['名前が異なる', 'カード名が異なる', 'グループ名が異なる', 'コストがそれぞれ異なる']:
        if kw in text:
            result['distinct'] = True
            break
    # targets
    if '自分と相手' in text:
        result['target'] = 'both'
    elif '相手の' in text:
        result['target'] = 'opponent'
    elif '自分の' in text:
        result['target'] = 'self'
    return result

def _distinct(text):
    """名前が異なる — distinct name condition."""
    if '名前が異なる' not in text and '名前の異なる' not in text and 'ユニット名がそれぞれ異なる' not in text:
        return None
    loc = 'stage'
    if '控え室' in text:
        loc = 'discard'
    elif '手札' in text:
        loc = 'hand'
    result = {'type': 'location_condition', 'target': 'self', 'distinct': True, 'location': loc, 'text': text}
    m = re.search(r'(\d+)(人|枚|つ)以上', text)
    if m:
        result['count'] = int(m.group(1))
        result['operator'] = '>='
        result['unit'] = m.group(2)
    if 'エリアすべて' in text:
        result['all_areas'] = True
    return result

def _card_count(text):
    """N枚以上/人以上 — card count condition."""
    for pat, op in [(r'(\d+)枚以上ある', '>='), (r'(\d+)種類以上ある', '>='),
                    (r'(\d+)枚ある', '='), (r'(\d+)人以上', '>='),
                    (r'(\d+)(人|枚|つ)以上いる', '>=')]:
        m = re.search(pat, text)
        if m:
            result = {'type': 'card_count_condition', 'count': int(m.group(1)),
                      'operator': op, 'text': text}
            if len(m.groups()) >= 2 and m.group(2):
                result['unit'] = m.group(2)
            if result.get('unit') == '人':
                result['card_type'] = 'member_card'
            return result
    return None

def _both(text):
    """それらが両方ある — both condition."""
    if 'それらが両方ある' not in text:
        return None
    return {'type': 'both_condition', 'text': text}

def _temporal_this_turn(text):
    """このターン + temporal condition."""
    if 'このターン' not in text:
        return None
    for pattern, cond_type in [('移動していない', 'not_moved'), ('移動している', 'has_moved'),
                                ('ライブを成功させていた', 'opponent_live_success')]:
        if pattern in text:
            result = {'type': 'temporal_condition', 'temporal': 'this_turn',
                      'condition': {'type': cond_type}, 'text': text}
            return result
    return None

def _baton_touch(text):
    """バトンタッチして登場 — baton touch condition."""
    if 'バトンタッチして登場した' not in text:
        return None
    result = {'type': 'location_condition', 'location': 'stage', 'target': 'self',
              'baton_touch_trigger': True, 'text': text}
    if '能力を持たない' in text or '能力も持たない' in text:
        result['ability_negation'] = True
    if 'コスト' in text:
        if '低い' in text:
            result['comparison_type'] = 'cost'; result['operator'] = '<'
        elif '高い' in text:
            result['comparison_type'] = 'cost'; result['operator'] = '>'
    if 'このメンバー以外' in text:
        result['exclude_self'] = True
    return result

def _temporal_count(text):
    """このターン + 回 + 登場 — temporal count condition."""
    if not (('このターン' in text or 'ターン目' in text) and ('回' in text or '登場' in text)):
        return None
    result = {'type': 'temporal_condition', 'temporal': 'this_turn', 'text': text}
    m = re.search(r'(\d+)回', text)
    if m:
        result['count'] = int(m.group(1))
    elif '登場' in text and '回' not in text:
        result['count'] = 1
    if 'ライブフェイズ' in text: result['phase'] = 'live_phase'
    elif 'メインフェイズ' in text: result['phase'] = 'main_phase'
    if '自分の' in text: result['target'] = 'self'
    elif '相手の' in text: result['target'] = 'opponent'
    return result

def _or(text):
    """か、 — OR condition."""
    if 'か、' not in text:
        return None
    parts = [p.strip() for p in text.split('か、') if p.strip()]
    if len(parts) < 2:
        return None
    parsed = [parse_condition(p) for p in parts]
    return {'type': 'or_condition', 'conditions': parsed, 'text': text}

def _movement(text):
    """移動した/移動している — movement condition."""
    if '移動した' not in text and '移動している' not in text:
        return None
    result = {'type': 'movement_condition', 'movement': 'moved',
              'movement_state': 'has_moved', 'text': text}
    if '移動していない' in text:
        result['negation'] = True
    return result

def _appearance(text):
    """登場 — appearance condition."""
    if '登場' not in text:
        return None
    result = {'type': 'appearance_condition', 'appearance': True, 'text': text}
    if 'エリアすべて' in text:
        result['all_areas'] = True
    return result

def _state(text):
    """ウェイト状態/アクティブ状態 — state condition."""
    for patterns, state_name in [
        (['ウェイト状態である', 'ウェイト状態にある', 'ウェイト状態の'], 'wait'),
        (['アクティブ状態である', 'アクティブ状態にある', 'アクティブ状態の'], 'active'),
    ]:
        if any(p in text for p in patterns):
            result = {'type': 'state_condition', 'state': state_name, 'text': text}
            if 'エネルギー' in text:
                result['resource_type'] = 'energy'
            return result
    return None

def _position(text):
    """センター/左サイド/右サイド — position condition."""
    for kw in ['センターエリア', '左サイドエリア', '右サイドエリア', 'センター', '左サイド', '右サイド']:
        if kw in text:
            return {'type': 'position_condition', 'text': text}
    return None

def _energy_state(text):
    """エネルギーがある — energy state."""
    if 'エネルギーがある' not in text:
        return None
    result = {'type': 'energy_state_condition', 'text': text}
    if 'アクティブ状態' in text:
        result['state'] = 'active'
    return result

def _live_mid(text):
    """ライブ中 — during live."""
    if 'ライブ中' not in text:
        return None
    result = {'text': text}
    cm = re.search(r'(\d+)枚以上', text)
    if cm:
        result['type'] = 'card_count_condition'
        result['count'] = int(cm.group(1)); result['operator'] = '>='
        result['card_type'] = 'live_card'; result['target'] = 'self'
        result['temporal'] = 'during_live'
    else:
        result['type'] = 'temporal_condition'; result['temporal'] = 'during_live'
    return result

def _location(text):
    """Generic location-based — last resort."""
    loc = None
    if 'ステージ' in text:
        loc = 'stage'
    elif '控え室' in text:
        loc = 'discard'
    elif '手札' in text:
        loc = 'hand'
    elif 'デッキ' in text or '山札' in text:
        loc = 'deck'
    elif 'エネルギー置き場' in text or 'エネルギーゾーン' in text:
        loc = 'energy_zone'
    elif '成功ライブカード置き場' in text:
        loc = 'success_live_zone'
    elif 'ライブカード置き場' in text:
        loc = 'live_card_zone'
    if not loc:
        return None
    result = {'type': 'location_condition', 'location': loc, 'text': text}
    if '自分の' in text and '相手の' not in text:
        result['target'] = 'self'
    elif '相手の' in text and '自分の' not in text:
        result['target'] = 'opponent'
    return result


def _comparison(text):
    """Fallback: try generic comparison extraction."""
    for kw in ['スコア', 'コスト']:
        if kw in text and ('高い' in text or '低い' in text or '多い' in text or '少ない' in text):
            op = '>' if any(x in text for x in ['高い', '多い', '大きい']) else '<'
            return {'type': 'comparison_condition',
                    'comparison_type': 'score' if 'スコア' in text else 'cost',
                    'operator': op, 'text': text}
    return None


# ------------------------------------------------------------------
# Handler registry — priority-annotated cascade
# ------------------------------------------------------------------

CONDITIONS = registry(
    Rule(100, 'complex',        _complex),
    Rule(99,  'compound',       _compound),
    Rule(98,  'distinct',       _distinct),
    Rule(97,  'card_count',     _card_count),
    Rule(96,  'both',           _both),
    Rule(95,  'temporal_this',  _temporal_this_turn),
    Rule(94,  'baton_touch',    _baton_touch),
    Rule(93,  'temporal_count', _temporal_count),
    Rule(92,  'or',             _or),
    Rule(91,  'movement',       _movement),
    Rule(90,  'appearance',     _appearance),
    Rule(89,  'energy_state',   _energy_state),
    Rule(88,  'state',          _state),
    Rule(87,  'position',       _position),
    Rule(86,  'live_mid',       _live_mid),
    Rule(85,  'comparison',     _comparison),
    Rule(10,  'location',       _location),
)


def parse_condition(text: str) -> Optional[Dict]:
    """Parse a condition text using priority-annotated handler registry."""
    for rule in CONDITIONS:
        result = rule.match(text)
        if result and isinstance(result, dict) and result.get('type'):
            return result
    return {'type': 'custom', 'text': text}

"""Action dispatch — tests the RuleRegistry with real dispatch rules from parser.py."""

from __future__ import annotations
import re
from typing import Any, Dict
from dispatcher import registry, Rule

# ------------------------------------------------------------------
# Helper: extract fields from text before dispatch
# ------------------------------------------------------------------

def pre_extract(text: str) -> Dict[str, Any]:
    """Extract source, destination, count, etc. (simplified version)."""
    state: Dict[str, Any] = {'text': text}
    # source
    if '手札から' in text or '手札を' in text:
        state['source'] = 'hand'
    elif '控え室から' in text:
        state['source'] = 'discard'
    elif 'デッキから' in text or '山札から' in text:
        state['source'] = 'deck'
    elif 'ステージから' in text:
        state['source'] = 'stage'
    # destination
    if '控え室に置く' in text or '控え室に置いて' in text:
        state['destination'] = 'discard'
    elif '手札に加える' in text or '手札に加えて' in text:
        state['destination'] = 'hand'
    elif 'ステージに置く' in text or '登場させる' in text:
        state['destination'] = 'stage'
    elif 'デッキの上に置く' in text:
        state['destination'] = 'deck_top'
    elif 'デッキの下に置く' in text:
        state['destination'] = 'deck_bottom'
    elif 'エネルギー置き場に置く' in text or 'エネルギーゾーンに置く' in text:
        state['destination'] = 'energy_zone'
    # count
    m = re.search(r'(\d+)枚', text)
    if m:
        state['count'] = int(m.group(1))
    return state

# ------------------------------------------------------------------
# Setter helpers
# ------------------------------------------------------------------

def set_card_type(text: str, state: Dict) -> None:
    if 'メンバーカード' in text:
        state['card_type'] = 'member_card'
    elif 'ライブカード' in text:
        state['card_type'] = 'live_card'
    elif 'エネルギーカード' in text:
        state['card_type'] = 'energy_card'

# ------------------------------------------------------------------
# Dispatch rules — built once at module level
# ------------------------------------------------------------------

DISPATCH = registry(
    # Cost-modification before move_cards (source+dest both set)
    Rule(80, 'modify_cost',
         match=lambda t: bool(re.search(r'コスト[はが](\d+)(減る|減らす|増える|増やす)', t)),
         apply=lambda t, s: s.update({'operation': 'decrease' if '減' in t else 'increase'})
                                or s.update({'value': int(re.search(r'(\d+)', t.split('コスト')[1]).group(1))}),
         help='コストは2減る etc'),

    # move_cards when both source AND destination are known
    Rule(70, 'move_cards',
         match=lambda t, s: 'source' in s and 'destination' in s,
         apply=set_card_type),

    # draw_card — broad match
    Rule(60, 'draw_card',
         match=lambda t: '引く' in t or '引き' in t,
         apply=lambda t, s: s.update({'source': 'deck', 'destination': 'hand'})),

    # pay_energy
    Rule(55, 'pay_energy',
         match=lambda t: '{{icon_energy.png|E}}' in t and ('支払う' in t or '支払って' in t or '支払い' in t),
         apply=lambda t, s: s.update({'energy': t.count('{{icon_energy.png|E}}'),
                                      'optional': 'もよい' in t})),

    # change_state — wait/active
    Rule(50, 'change_state',
         match=lambda t: 'ウェイトにする' in t or 'アクティブにする' in t,
         apply=lambda t, s: s.update({'state_change': 'wait' if 'ウェイト' in t else 'active'})),

    # gain_resource — blade
    Rule(40, 'gain_resource',
         match=lambda t: '{{icon_blade.png|ブレード}}' in t and '得る' in t,
         apply=lambda t, s: s.update({'resource': 'blade',
                                      'count': t.count('{{icon_blade.png|ブレード}}')})),

    # gain_resource — heart
    Rule(35, 'gain_resource',
         match=lambda t: '{{heart' in t and '得る' in t,
         apply=lambda t, s: s.update({'resource': 'heart'})),

    # shuffle
    Rule(30, 'shuffle',
         match=lambda t: 'シャッフルする' in t),

    # invalidate_ability
    Rule(20, 'invalidate_ability',
         match=lambda t: '無効にする' in t or '無効に' in t),

    # position_change
    Rule(10, 'position_change',
         match=lambda t: '入れ替える' in t or 'ポジションチェンジ' in t),

    # restriction
    Rule(5, 'restriction',
         match=lambda t: 'できない' in t or '置くことができない' in t),
)


def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text using the static dispatch registry."""
    state = pre_extract(text)
    DISPATCH.dispatch(text, state)
    return state

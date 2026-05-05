"""Complete action dispatch — ALL rules from parser.py's _R table, in priority order."""

from __future__ import annotations
import re
from typing import Any, Dict
from dispatcher import registry, Rule

# ------------------------------------------------------------------
# Helper: pre-extract fields from text (mirrors parser.py lines ~1382-1470)
# ------------------------------------------------------------------

def _ic(text: str, tag: str) -> int:
    return text.count(tag) or 0

def _handle_cost_mod(text: str, state: Dict) -> None:
    if '減る' in text or '減らす' in text:
        state['operation'] = 'decrease'
    elif '増える' in text or '増やす' in text:
        state['operation'] = 'increase'
    vm = re.search(r'コスト[はが](\d+)(減る|減らす|増える|増やす)', text)
    if vm:
        state['value'] = int(vm.group(1))
    ic = text.count('{{icon_energy.png|E}}')
    if ic > 0:
        state['count'] = ic

def extract_count(text: str) -> int:
    m = re.search(r'(\d+)枚', text) or re.search(r'(\d+)人', text) or re.search(r'(\d+)つ', text) or re.search(r'(\d+)回', text)
    return int(m.group(1)) if m else 0

def extract_target(text: str) -> str:
    if ('自分の' in text and '相手の' in text) or '自分と相手の' in text:
        return 'both'
    if '自分か相手の' in text:
        return 'either'
    if '相手の' in text:
        return 'opponent'
    if '自分の' in text:
        return 'self'
    return ''

def extract_optional(text: str) -> bool:
    return 'もよい' in text or 'てもよい' in text

def extract_dest(text: str) -> str:
    if '控え室に置く' in text:
        return 'discard'
    if '手札に加える' in text:
        return 'hand'
    if 'デッキの上に置く' in text:
        return 'deck_top'
    if 'デッキの下に置く' in text:
        return 'deck_bottom'
    if 'エネルギー置き場に置く' in text or 'エネルギーゾーンに置く' in text:
        return 'energy_zone'
    if '登場させる' in text:
        return 'stage'
    return ''

def pre_extract(text: str) -> Dict[str, Any]:
    state: Dict[str, Any] = {'text': text}

    # source (hand-written subset of parser.py extract_source)
    if '手札から' in text or '手札を' in text:
        state['source'] = 'hand'
    elif '控え室から' in text or '控え室にある' in text:
        state['source'] = 'discard'
    elif 'デッキから' in text or '山札から' in text:
        state['source'] = 'deck'
    elif 'デッキの上から' in text:
        state['source'] = 'deck_top'
    elif 'ステージから' in text:
        state['source'] = 'stage'
    elif 'エネルギー置き場から' in text:
        state['source'] = 'energy_zone'

    # destination
    dest = extract_dest(text)
    if dest:
        state['destination'] = dest

    # count
    cnt = extract_count(text)
    if cnt:
        state['count'] = cnt

    # target
    tgt = extract_target(text)
    if tgt:
        state['target'] = tgt

    return state


# ------------------------------------------------------------------
# Complete dispatch table — ALL ~58 rules from parser.py, priority-ordered
# ------------------------------------------------------------------

DISPATCH = registry(
    # ── Shuffle ──────────────────────────────────────────────────
    Rule(100, 'shuffle',
         match=lambda t: 'シャッフルする' in t or 'シャッフルして' in t,
         apply=lambda t, s: s.update({'target': 'deck' if 'デッキ' in t else 'energy_deck'})),

    # ── Position change ──────────────────────────────────────────
    Rule(99, 'position_change',
         match=lambda t: '入れ替える' in t or '入れ替えて' in t),
    Rule(98, 'position_change',
         match=lambda t: 'フォーメーションチェンジ' in t,
         apply=lambda t, s: s.update({'optional': extract_optional(t)})),

    # ── Pay energy ───────────────────────────────────────────────
    Rule(96, 'pay_energy',
         match=lambda t: '{{icon_energy.png|E}}' in t and ('支払う' in t or '支払って' in t or '支払い' in t),
         apply=lambda t, s: s.update({'energy': t.count('{{icon_energy.png|E}}'),
                                      'optional': extract_optional(t)})),
    Rule(95, 'pay_energy',
         match=lambda t: '{{icon_energy.png|E}}' in t and 'エネルギー' in t),

    # ── Place energy under member ─────────────────────────────────
    Rule(94, 'place_energy_under_member',
         match=lambda t, s: s.get('destination') == 'under_member' and ('エネルギー' in t or 'energy_card' in t),
         apply=lambda t, s: s.update({'energy_count': s.get('count', 1)})),

    # ── Draw/discard until count ──────────────────────────────────
    Rule(93, 'draw_until_count',
         match=lambda t: '枚になるまで' in t and '引く' in t,
         apply=lambda t, s: s.update({'source': 'deck', 'destination': 'hand',
                                      'target_count': int(re.search(r'(\d+)枚になるまで', t).group(1))})),
    Rule(92, 'discard_until_count',
         match=lambda t: '枚になるまで' in t and ('控え室に置く' in t or '控え室に置き' in t),
         apply=lambda t, s: s.update({'target_count': int(re.search(r'(\d+)枚になるまで', t).group(1))})),

    # ── Draw card ────────────────────────────────────────────────
    Rule(91, 'draw_card',
         match=lambda t: 'カードを1枚引いてもよい' in t,
         apply=lambda t, s: s.update({'count': 1, 'optional': True, 'source': 'deck', 'destination': 'hand'})),
    Rule(90, 'draw_card',
         match=lambda t: '引く' in t or '引き' in t,
         apply=lambda t, s: s.update({'source': 'deck', 'destination': 'hand'})),
    Rule(89, 'draw_card',
         match=lambda t: '引いてもよい' in t,
         apply=lambda t, s: s.update({'source': 'deck', 'destination': 'hand', 'optional': True})),

    # ── Cost modification — BEFORE move_cards (both match) ────────
    Rule(88, 'modify_cost',
         match=lambda t: bool(re.search(r'コスト[はが](\d+)(減る|減らす|増える|増やす)', t))
                         or 'ためのコストは' in t and '減る' in t,
         apply=_handle_cost_mod),

    # ── move_cards with known source+destination ──────────────────
    Rule(85, 'move_cards',
         match=lambda t, s: 'source' in s and s.get('source') and 'destination' in s and s.get('destination')),

    # ── change_state ──────────────────────────────────────────────
    Rule(83, 'change_state',
         match=lambda t, s: s.get('state_change') not in (None, '')),
    Rule(82, 'change_state',
         match=lambda t: 'アクティブにしてもよい' in t or 'アクティブにする' in t,
         apply=lambda t, s: s.update({'state_change': 'active', **({'optional': True} if 'してもよい' in t else {})})),
    Rule(81, 'change_state',
         match=lambda t: 'ウェイトにする' in t or 'ウェイト状態で' in t or 'ウェイトにし' in t,
         apply=lambda t, s: s.update({'state_change': 'wait'})),

    # ── Activation restriction ────────────────────────────────────
    Rule(80, 'activation_restriction',
         match=lambda t: 'のみ起動できる' in t or 'のみ発動する' in t,
         apply=lambda t, s: s.update({'restriction_type': 'only'})),

    # ── Activate ability ──────────────────────────────────────────
    Rule(79, 'activate_ability',
         match=lambda t: '支払って発動させる' in t,
         apply=lambda t, s: s.update({'activation_type': 'pay_to_activate'})),

    # ── Restrictions ──────────────────────────────────────────────
    Rule(78, 'restriction',
         match=lambda t: 'ライブできない' in t,
         apply=lambda t, s: s.update({'restriction_type': 'cannot_live'})),
    Rule(77, 'restriction',
         match=lambda t: 'アクティブにしない' in t,
         apply=lambda t, s: s.update({'restriction_type': 'cannot_activate'})),
    Rule(76, 'restriction',
         match=lambda t: 'バトンタッチで控え室に置けない' in t,
         apply=lambda t, s: s.update({'restriction_type': 'cannot_baton_touch'})),
    Rule(75, 'restriction',
         match=lambda t: '置くことができない' in t,
         apply=lambda t, s: s.update({'restriction_type': 'cannot_place'})),
    Rule(74, 'restriction',
         match=lambda t: '置けない' in t,
         apply=lambda t, s: s.update({'restriction_type': 'cannot_place'})),
    Rule(73, 'restriction',
         match=lambda t: '登場できない' in t,
         apply=lambda t, s: s.update({'restriction_type': 'cannot_appear'})),
    Rule(72, 'restriction',
         match=lambda t: '移動できない' in t,
         apply=lambda t, s: s.update({'restriction_type': 'cannot_move'})),

    # ── move_cards (by keyword) ──────────────────────────────────
    Rule(71, 'move_cards',
         match=lambda t: '加える' in t or '加え' in t,
         apply=lambda t, s: s.update({'destination': 'hand'})),
    Rule(70, 'position_change',
         match=lambda t: 'ポジションチェンジ' in t,
         apply=lambda t, s: s.update({'target': extract_target(t)})),
    Rule(69, 'position_change',
         match=lambda t: '移動させ' in t and 'エリア' in t),
    Rule(68, 'move_cards',
         match=lambda t: '移動させ' in t and 'エリア' not in t),
    Rule(67, 'move_cards',
         match=lambda t: '置く' in t or '置いて' in t,
         apply=lambda t, s: s.update({'destination': extract_dest(t)}) if 'destination' not in s else None),

    # ── Gain resource ────────────────────────────────────────────
    Rule(66, 'gain_resource',
         match=lambda t: 'ブレードを得る' in t or '選んだブレード' in t,
         apply=lambda t, s: s.update({'resource': 'blade', 'count': _ic(t, '{{icon_blade.png|ブレード}}')})),
    Rule(65, 'gain_resource',
         match=lambda t: '{{icon_blade.png|ブレード}}' in t and '得る' in t,
         apply=lambda t, s: s.update({'resource': 'blade', 'count': t.count('{{icon_blade.png|ブレード}}') or None})),
    Rule(64, 'gain_resource',
         match=lambda t: ('{{heart' in t and '得る' in t) or 'ハートを得る' in t or '選んだハート' in t,
         apply=lambda t, s: s.update({'resource': 'heart', 'count': len(re.findall(r'{{heart_\d+\.png\|heart\d+}}', t)) or None})),

    # ── Re-yell ──────────────────────────────────────────────────
    Rule(63, 're_yell',
         match=lambda t: 'もう一度エール' in t or 'もう1度エール' in t,
         apply=lambda t, s: s.update({'lose_blade_hearts': True}) if 'できない' not in t else None),

    # ── Look at ───────────────────────────────────────────────────
    Rule(62, 'look_at',
         match=lambda t: '見る' in t or '見て' in t,
         apply=lambda t, s: s.update({'source': 'deck_top' if 'デッキの上' in t else 'deck'})),

    # ── Reveal ───────────────────────────────────────────────────
    Rule(61, 'reveal',
         match=lambda t: '公開する' in t,
         apply=lambda t, s: s.update({'source': s.get('source', 'hand')})),
    Rule(60, 'reveal',
         match=lambda t: '1枚ずつ公開' in t or '枚ずつ公開' in t,
         apply=lambda t, s: s.update({'per_unit': True, 'per_unit_count': 1, 'multiple_targets': True})),

    # ── Select ───────────────────────────────────────────────────
    Rule(59, 'select',
         match=lambda t: '選ぶ' in t or '選ん' in t),

    # ── Appear ───────────────────────────────────────────────────
    Rule(57, 'appear',
         match=lambda t: '登場させ' in t),

    # ── Activate ability (generic) ──────────────────────────────
    Rule(56, 'activate_ability',
         match=lambda t: '起動でき' in t or '起動して' in t),

    # ── Invalidate ability ───────────────────────────────────────
    Rule(55, 'invalidate_ability',
         match=lambda t: '無効に' in t),
    Rule(54, 'invalidate_ability',
         match=lambda t: '無効にできない' in t,
         apply=lambda t, s: s.update({'optional': True})),

    # ── Modify required hearts ────────────────────────────────────
    Rule(53, 'modify_required_hearts',
         match=lambda t: '必要ハート' in t or 'ハートを増やす' in t or 'ハートを減らす' in t),

    # ── Modify score ──────────────────────────────────────────────
    Rule(52, 'modify_score',
         match=lambda t: '追加' in t,
         apply=lambda t, s: s.update({'operation': 'add'})),
    Rule(51, 'modify_score',
         match=lambda t: 'スコアを1プラス' in t or 'スコアをプラス' in t,
         apply=lambda t, s: s.update({'operation': 'add', 'value': 1})),
    Rule(50, 'modify_score',
         match=lambda t: 'スコアを1マイナス' in t,
         apply=lambda t, s: s.update({'operation': 'remove', 'value': 1})),
    Rule(49, 'choice',
         match=lambda t: '以下から1つを選ぶ' in t),

    # ── Set blade type / count ────────────────────────────────────
    Rule(48, 'set_blade_type',
         match=lambda t: 'ブレードの色を' in t),
    Rule(47, 'gain_resource',
         match=lambda t: 'ハートの色を' in t or ('ハートを' in t and 'にする' in t),
         apply=lambda t, s: s.update({'resource': 'heart', 'heart_selection': True})),

    # ── Set required hearts (cost with heart icons) ─────────────
    Rule(46, 'set_required_hearts',
         match=lambda t: ('コストを' in t or 'コストが' in t or 'コストは' in t) and '{{heart_' in t,
         apply=lambda t, s: (
             s.update({'heart_colors': [m.group(1) for m in re.finditer(r'\|(heart\d{2})}', t)]})
             if re.finditer(r'\|(heart\d{2})}', t) else None) or
             s.update({'count': len(re.findall(r'{{heart_\d+\.png\|heart\d+}}', t))})),

    # ── modify_cost (generic) ────────────────────────────────────
    Rule(45, 'modify_cost',
         match=lambda t: 'コストを' in t or 'コストが' in t or 'コストは' in t,
         apply=_handle_cost_mod),

    # ── Repeat procedure ─────────────────────────────────────────
    Rule(44, 'repeat_procedure',
         match=lambda t: '繰り返してもよい' in t,
         apply=lambda t, s: (s.update({'max_repeats': int(re.search(r'(\d+)回', t).group(1))})
                             if re.search(r'(\d+)回', t) else None)),

    # ── Gain ability (generic "得る") ────────────────────────────
    Rule(43, 'gain_ability',
         match=lambda t: '得る' in t),

    # ── do_nothing / empty ───────────────────────────────────────
    Rule(42, 'do_nothing',
         match=lambda t: '何もしない' in t),
    Rule(41, 'do_nothing',
         match=lambda t: t.strip() == ''),

    # ── Baton touch ──────────────────────────────────────────────
    Rule(40, 'play_baton_touch',
         match=lambda t: 'バトンタッチ' in t),

    # ── Set score ────────────────────────────────────────────────
    Rule(39, 'set_score',
         match=lambda t: ('スコアは' in t or 'スコアが' in t) and ('になる' in t or 'なった' in t or 'なっている' in t),
         apply=lambda t, s: (s.update({'value': int(re.search(r'(\d+)', t.split('スコア')[1]).group(1))})
                             if re.search(r'(\d+)', t.split('スコア')[1] if 'スコア' in t else '') else None)),

    # ── modify_score (generic) ──────────────────────────────────
    Rule(38, 'modify_score',
         match=lambda t: 'スコアを' in t,
         apply=lambda t, s: s.update({
             'operation': 'add' if 'プラス' in t or '+' in t else ('remove' if 'マイナス' in t or '-' in t else None),
             'value': extract_count(t) or None})),

    # ── move_cards to deck_top ──────────────────────────────────
    Rule(37, 'move_cards',
         match=lambda t: 'デッキの上に置き' in t or 'デッキの上に置く' in t,
         apply=lambda t, s: s.update({
             'destination': 'deck_top',
             **({'placement_order': 'any_order'} if '好きな順番で' in t else {})})),

    # ── Modify yell count ────────────────────────────────────────
    Rule(36, 'modify_yell_count',
         match=lambda t: 'エール' in t and ('枚数' in t or '数' in t)),

    # ── Set card identity ────────────────────────────────────────
    Rule(35, 'set_card_identity',
         match=lambda t: ('セット' in t or '設定' in t) and 'コスト' not in t),

    # ── Blade conversion ─────────────────────────────────────────
    Rule(34, 'set_blade_type',
         match=lambda t: 'すべて[' in t and ']になる' in t,
         apply=lambda t, s: s.update({'blade_type': re.search(r'\[([^\]]+)\]', t).group(1)})),

    # ── Fallback move_cards (置く/置いて) ──────────────────────
    Rule(33, 'move_cards',
         match=lambda t: '置く' in t or '置いて' in t,
         apply=lambda t, s: s.update({'destination': extract_dest(t)})),
)


def parse_action(text: str) -> Dict[str, Any]:
    """Parse action text using the static dispatch registry."""
    state = pre_extract(text)
    DISPATCH.dispatch(text, state)
    return state

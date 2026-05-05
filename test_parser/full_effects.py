"""Complete effect handler cascade — ALL handlers from parser.py's _EFFECT_HANDLERS list."""

from __future__ import annotations
import re
from typing import Any, Dict, Optional
from dispatcher import Rule, RuleRegistry, registry

from full_actions import parse_action
from conditions import parse_condition

# ------------------------------------------------------------------
# Utility
# ------------------------------------------------------------------

def strip_parens(text):
    return re.sub(r'（[^）]*）', '', text).strip()

def extract_optional(text):
    return 'もよい' in text or 'てもよい' in text

def extract_dest(text):
    if '控え室に置く' in text: return 'discard'
    if '手札に加える' in text: return 'hand'
    if 'デッキの一番上に置く' in text: return 'deck_top'
    if 'デッキの上に置く' in text: return 'deck_top'
    if 'デッキの一番下に置く' in text: return 'deck_bottom'
    if 'デッキの下に置く' in text: return 'deck_bottom'
    if 'エネルギー置き場に置く' in text: return 'energy_zone'
    if 'エネルギーゾーンに置く' in text: return 'energy_zone'
    if '登場させる' in text: return 'stage'
    if 'いたエリアに' in text or '置かれていたエリアに' in text: return 'same_area'
    if 'メンバーのいないエリア' in text: return 'empty_area'
    if 'このメンバーの下に置く' in text: return 'under_member'
    return ''

# ------------------------------------------------------------------
# Effect handlers — each returns complete effect dict or None
# ------------------------------------------------------------------

def _per_unit(text):
    excludes = ('各グループ名につき', 'グループ名につき', 'グループ名')
    if 'につき' not in text and 'ごとに' not in text:
        return None
    if any(e in text for e in excludes):
        return None
    if 'コストは' in text and '減る' in text:
        return None
    m = re.search(r'(.+?)(につき|ごとに)', text)
    if not m:
        return None
    per_text = m.group(1).strip()
    if '。' in per_text:
        return None
    result = {'text': text, 'per_unit': True}
    pm = re.search(r'(\d+)(人|枚|つ)', per_text)
    if pm:
        result['per_unit_count'] = int(pm.group(1))
        result['per_unit_type'] = pm.group(2)
    else:
        result['per_unit_count'] = 1
        for kw, t in [('メンバー', 'member'), ('人', 'member'), ('カード', 'card'),
                       ('ブレード', 'blade'), ('ハート', 'heart')]:
            if kw in per_text:
                result['per_unit_type'] = t; break
    if 'ライブ終了時まで' in text:
        result['duration'] = 'live_end'
    action_text = text.split('につき', 1)[1].strip().lstrip('、')
    action = parse_action(action_text)
    for k, v in result.items():
        if k not in action:
            action[k] = v
    action['text'] = text
    return action

def _conditional_alternative(text):
    if '代わりに' not in text:
        return None
    parts = text.split('代わりに', 1)
    if len(parts) != 2:
        return None
    fa = parse_action(parts[0].strip())
    aa = parse_action(parts[1].strip())
    return {'text': text, 'action': 'conditional_alternative',
            'primary_effect': fa, 'alternative_effect': aa}

def _activation_suffix(text):
    m = re.search(r'この能力は、(.+?)場合のみ(?:起動できる|発動する)', text)
    if not m:
        return None
    suffix = m.group(0).split('場合のみ')[-1]
    cond_text = 'この能力は、' + m.group(1).strip() + '場合のみ' + suffix
    action_text = text.replace(cond_text, '').strip().rstrip('。')
    action = parse_action(action_text)
    result = {'text': text, 'activation_condition': cond_text}
    result.update(action)
    return result

def _look_and_select(text):
    if 'その中から' not in text:
        return None
    lm = re.search(r'(.+?)その中から', text)
    if not lm:
        return None
    look_text = lm.group(1).strip()
    rest = text[lm.end():].strip()
    look_action = parse_action(look_text)
    result = {'text': text, 'action': 'look_and_select', 'look_action': look_action}
    if '残りを控え室に置く' in rest:
        parts = re.split(r'[、。]', rest)
        if len(parts) >= 2:
            fa = parse_action(parts[0].strip())
            sa = parse_action(parts[1].strip())
            result['select_action'] = {'action': 'sequential', 'actions': [fa, sa]}
    else:
        result['select_action'] = parse_action(rest)
    return result

def _each_time(text):
    if 'たび' not in text:
        return None
    tm = re.search(r'([^たび]+)たび', text)
    if not tm:
        return None
    rest = text[tm.end():].strip()
    trigger_text = tm.group(1).strip()
    sub = parse_effect(rest)
    sub['trigger_type'] = 'each_time'
    sub['text'] = text
    if 'か、' in trigger_text:
        or_result = _or(trigger_text)
        if or_result:
            sub['trigger_condition'] = or_result
    return sub

def _or(text):
    if 'か、' not in text:
        return None
    parts = [p.strip() for p in text.split('か、') if p.strip()]
    if len(parts) < 2:
        return None
    return {'type': 'or_condition', 'conditions': [parse_condition(p) for p in parts], 'text': text}

def _furthermore(text):
    if 'さらに' not in text:
        return None
    parts = text.split('。')
    if len(parts) < 2:
        return None
    actions = []
    for p in parts:
        pt = p.strip()
        if not pt:
            continue
        if 'さらに' in pt:
            pt = pt.replace('さらに', '', 1).strip()
        actions.append(parse_effect(pt))
    if len(actions) >= 2:
        return {'text': text, 'action': 'sequential', 'actions': actions}
    return None

def _conditional_sequential(text):
    if 'そうした場合' not in text:
        return None
    parts = text.split('そうした場合', 1)
    fp = parts[0].strip()
    sp = parts[1].strip().lstrip('、').lstrip('。')
    fa = parse_action(fp)
    sa = parse_effect(sp)
    result = {'text': text, 'action': 'sequential', 'actions': [fa, sa], 'conditional': True}
    return result

def _sequential(text):
    if 'その後' not in text:
        return None
    parts = text.split('その後', 1)
    fa = parse_effect(parts[0].strip())
    sa = parse_effect(parts[1].strip().lstrip('、'))
    return {'text': text, 'action': 'sequential', 'actions': [fa, sa]}

def _duration_effect(text):
    if 'かぎり' not in text:
        return None
    parts = text.split('かぎり', 1)
    ct = parts[0].strip() + 'かぎり'
    at = parts[1].strip().lstrip('、')
    cond = parse_condition(ct)
    action = parse_action(at)
    result = {'text': text, 'condition': cond, 'duration': 'as_long_as'}
    result.update(action)
    return result

def _choice(text):
    if '以下から1つを選ぶ' not in text:
        return None
    parts = text.split('以下から1つを選ぶ', 1)
    opt_text = parts[1].strip()
    lines = opt_text.split('\n')
    opts = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        if line.startswith('・'):
            opts.append(parse_action(line[1:].strip()))
    if opts:
        return {'text': text, 'action': 'choice', 'options': opts}
    return None

def _play_baton_touch(text):
    if 'プレイに際し' not in text or 'バトンタッチ' not in text:
        return None
    result = {'text': text, 'action': 'play_baton_touch'}
    m = re.search(r'(\d+)人のメンバーとバトンタッチ', text)
    if m:
        result['count'] = int(m.group(1))
    return result

def _global_modifier(text):
    if '必要ハート' not in text or ('多くなる' not in text and '少なくなる' not in text):
        return None
    if 'ある場合' in text:
        return None
    result = {'text': text, 'action': 'restriction',
              'restriction_type': 'modify_required_hearts_global',
              'operation': 'increase' if '多くなる' in text else 'decrease'}
    if '相手の' in text:
        result['target'] = 'opponent'
    else:
        result['target'] = 'self'
    if 'すべて' in text: result['all'] = True
    return result

def _implicit_sequential(text):
    if '、' not in text and '。' not in text:
        return None
    if any(m in text for m in ['場合、', 'とき、', 'なら、']):
        return None
    if '以下から1つを選ぶ' in text:
        return None
    if '。' in text:
        clean = re.sub(r'（[^）]*）', '', text)
        clean = re.sub(r'\([^)]*\)', '', clean)
        parts = [p.strip() for p in clean.split('。') if p.strip()]
        parts = [p for p in parts if not re.match(r'^ライブ終了時まで[、，]?$', p)]
    else:
        parts = text.split('、')
    if len(parts) < 2:
        return None
    actions = []
    for p in parts:
        cp = p.strip().lstrip('、')
        a = parse_effect(cp)
        if a and a.get('action', 'custom') != 'custom':
            actions.append(a)
    if len(actions) >= 2:
        return {'text': text, 'action': 'sequential', 'actions': actions}
    return None

def _conditional(text):
    for keyword in ['場合', 'とき', 'なら']:
        pattern = keyword + '、'
        if pattern not in text:
            continue
        idx = text.find(keyword)
        comma_idx = idx + len(keyword)
        cond_text = text[:comma_idx].strip()
        action_text = text[comma_idx + 1:].strip()
        action = parse_effect(action_text)
        cond = parse_condition(cond_text)
        if cond.get('type') == 'custom':
            result = {'text': text, 'raw_condition': cond_text}
        else:
            result = {'text': text, 'condition': cond}
        result['action'] = action.get('action', 'custom')
        if action.get('action') == 'sequential':
            result['actions'] = action.get('actions', [])
        elif action.get('action') != 'custom':
            for k, v in action.items():
                if k not in result:
                    result[k] = v
        return result
    # Also check for 時、 (plain kanji)
    t_pos = text.find('時、')
    if t_pos > 0 and 'ライブ終了時まで' not in text[:t_pos+2]:
        ct = text[:t_pos+1].strip()
        at = text[t_pos+2:].strip()
        return {'text': text, 'raw_condition': ct, 'action': 'custom',
                **parse_effect(at)}
    return None

def _shi_sequential(text):
    if '、' not in text or 'し' not in text:
        return None
    parts = [p.strip().rstrip('、') for p in text.split('、')]
    if len(parts) < 2 or 'し' not in parts[0]:
        return None
    actions = []
    for p in parts:
        a = parse_action(p)
        if a.get('action', 'custom') != 'custom':
            actions.append(a)
    if len(actions) >= 2:
        return {'text': text, 'action': 'sequential', 'actions': actions}
    return None

def _kore_niyori_cascade(text):
    m = re.search(r'^(.*?)。これにより(.+?)場合、(.+)$', text, re.DOTALL)
    if not m:
        return None
    acts = [parse_action(p) for p in m.group(1).split('。') if p.strip()]
    if not acts:
        return None
    cp = parse_condition(m.group(2).strip() + '場合')
    rp = parse_effect(m.group(3).strip())
    follow = {'condition': cp}
    follow.update(rp)
    acts.append(follow)
    return {'text': text, 'action': 'sequential', 'actions': acts}

def _ability_activation(text):
    m = re.search(r'(.+?)能力.*?を発動させる', text)
    if not m:
        return None
    target_raw = m.group(1).strip() + '能力'
    result = {'text': text, 'action': 'activate_ability', 'target': target_raw}
    cm = re.search(r'(\d+)つ', text)
    result['count'] = int(cm.group(1)) if cm else 1
    return result

def _gain_equal(text):
    if 'を得る' not in text or 'コストが同じ' not in text:
        return None
    result = {'text': text, 'action': 'gain_resource', 'resource': 'blade'}
    ic = text.count('{{icon_blade.png|ブレード}}')
    if ic > 0:
        result['count'] = ic
    return result

def _same_thing(text):
    if '同じことを行う' not in text:
        return None
    result = {'text': text, 'action': 'gain_resource', 'resource': 'blade'}
    ic = text.count('{{icon_blade.png|ブレード}}')
    result['count'] = ic if ic > 0 else 1
    if 'duration' not in result and 'ライブ終了時まで' in text:
        result['duration'] = 'live_end'
    return result

def _set_blade_count(text):
    if 'ブレードの数は' not in text:
        return None
    m = re.search(r'(\d+)つになる', text) or re.search(r'(\d+)になる', text)
    if not m:
        return None
    result = {'text': text, 'action': 'set_blade_count'}
    result['count'] = int(m.group(1))
    return result

def _blade_conversion(text):
    if 'すべて[' not in text or ']になる' not in text:
        return None
    m = re.search(r'すべて\[([^\]]+)\]', text)
    if not m:
        return None
    result = {'text': text, 'action': 'set_blade_type', 'blade_type': m.group(1)}
    if 'ライブ終了時まで' in text:
        result['duration'] = 'live_end'
    return result

def _both_discard_until(text):
    if '自分と相手はそれぞれ' not in text or '枚になるまで' not in text:
        return None
    if '控え室に置き' not in text and '控え室に置く' not in text:
        return None
    result = {'text': text, 'action': 'sequential', 'target': 'both', 'multiple_targets': True}
    parts = re.split(r'その後[、。]?', text, maxsplit=1)
    if len(parts) == 2:
        fa_text = parts[0].strip()
        sa_text = parts[1].strip()
        fa = {'text': fa_text, 'action': 'discard_until_count', 'target': 'both', 'multiple_targets': True}
        m = re.search(r'(\d+)枚になるまで', fa_text)
        if m:
            fa['target_count'] = int(m.group(1))
        sa = parse_effect(sa_text)
        result['actions'] = [fa, sa]
    return result if result.get('actions') else None

def _re_yell(text):
    if 'もう一度エールを行う' not in text or 'ブレードハートを失い' not in text:
        return None
    return {'text': text, 'action': 're_yell', 'lose_blade_hearts': True}

def _energy_under_member(text):
    if '下に置かれているエネルギーカード' not in text:
        return None
    return {'text': text, 'action': 'place_energy_under_member',
            'source': 'under_member', 'card_type': 'energy_card',
            'energy_count': 1, 'target_member': 'this_member'}

def _restriction_effect(text):
    if '効果によってはアクティブにならない' not in text:
        return None
    result = {'text': text, 'action': 'restriction', 'restriction_type': 'cannot_activate_by_effect'}
    if '相手の' in text: result['target'] = 'opponent'
    elif '自分の' in text: result['target'] = 'self'
    if 'このターン' in text: result['duration'] = 'this_turn'
    if 'メンバー' in text: result['card_type'] = 'member_card'
    return result

def _gain_raw(text):
    if 'を得る' in text:
        action = parse_action(text)
        if action.get('action') == 'custom':
            if '能力' in text:
                return {'text': text, 'action': 'gain_ability'}
            return {'text': text, 'action': 'gain_resource', 'resource': 'generic', 'count': 1}
    return None

def _is_null(text):
    stripped = re.sub(r'\s', '', text)
    if stripped.startswith('（') and stripped.endswith('）'):
        return {'text': text, 'action': 'do_nothing', 'is_null': True}
    if stripped.startswith('(') and stripped.endswith(')'):
        return {'text': text, 'action': 'do_nothing', 'is_null': True}
    return None


# ------------------------------------------------------------------
# Effect handler registry — priority-annotated cascade
# ------------------------------------------------------------------

EFFECT_HANDLERS: RuleRegistry = registry(
    Rule(100, 'is_null',                _is_null),
    Rule(99,  'per_unit',               _per_unit),
    Rule(98,  'conditional_alternative', _conditional_alternative),
    Rule(97,  'activation_suffix',      _activation_suffix),
    Rule(96,  'look_and_select',        _look_and_select),
    Rule(95,  'each_time',              _each_time),
    Rule(94,  'furthermore',            _furthermore),
    Rule(93,  'conditional_sequential', _conditional_sequential),
    Rule(92,  'sequential',             _sequential),
    Rule(91,  'duration_effect',        _duration_effect),
    Rule(90,  'implicit_sequential',    _implicit_sequential),
    Rule(89,  'conditional',            _conditional),
    Rule(88,  'shi_sequential',         _shi_sequential),
    Rule(87,  'choice',                 _choice),
    Rule(86,  'kore_niyori_cascade',    _kore_niyori_cascade),
    Rule(85,  'ability_activation',     _ability_activation),
    Rule(84,  'play_baton_touch',       _play_baton_touch),
    Rule(83,  'global_modifier',        _global_modifier),
    Rule(82,  'energy_under_member',     _energy_under_member),
    Rule(81,  'blade_conversion',       _blade_conversion),
    Rule(80,  'both_discard_until',     _both_discard_until),
    Rule(79,  're_yell',               _re_yell),
    Rule(78,  'restriction_effect',     _restriction_effect),
    Rule(77,  'gain_equal',             _gain_equal),
    Rule(76,  'same_thing',             _same_thing),
    Rule(75,  'set_blade_count',        _set_blade_count),
    Rule(60,  'gain_raw',               _gain_raw),
)


def parse_effect(text: str) -> Dict[str, Any]:
    """Parse effect text using priority-annotated handler cascade."""
    # Handle duration prefix
    duration = None
    for prefix, code in [('ライブ終了時まで', 'live_end'),
                         ('ライブ終了まで', 'live_end'),
                         ('このターンの間', 'this_turn'),
                         ('このライブの間', 'this_live')]:
        if text.startswith(prefix):
            duration = code
            text = text[len(prefix):].lstrip('、').strip()
            break

    for rule in EFFECT_HANDLERS:
        result = rule.match(text)
        if result and isinstance(result, dict) and (result.get('action') not in (None, 'custom') or result.get('actions')):
            if duration and 'duration' not in result:
                result['duration'] = duration
            return result

    # Fallback: parse as single action
    action = parse_action(text)
    if duration and 'duration' not in action:
        action['duration'] = duration
    return action

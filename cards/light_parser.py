"""
light_parser.py — A composable primitive-based ability parser.

Architecture:
  - 10 structural primitives that detect and decompose text
  - ~30 effect patterns that map verb phrases to action types
  - 1 composition engine that walks the tree and fills slots

Primitives compose: COND + MODIFY_SCORE = conditional score modification.
No cascade, no handler priority, no variant gap hunting.

Primitives:
  COND     — 場合/とき/なら → split into condition + action
  COLON    — ： → split into cost + effect
  SEQ_TE   — 連用形、 → sequential sub-actions
  DURATION — かぎり → duration-bounded effect
  PER_UNIT — につき → per-unit scaling effect
  LOOK_SEL — その中から → look-and-select
  CHOICE   — 以下から1つを選ぶ → choice
  MULTI_SENT — 。→ multi-sentence sequential
  ALT      — 代わりに → conditional alternative
  FURTHER  — さらに → additive sequential

Effect patterns (30):
  Each is: (trigger_words, action_name, slot_filler)
  Examples: (["引く", "引いて"], "draw_card", fill_draw),
            (["置く", "加える"], "move_cards", fill_move),
            (["得る"], "gain_resource", fill_gain_resource)
"""

import re
import json
from typing import Any, Dict, List, Optional, Tuple, Callable

# =========================================================================
# NORMALIZATION (single place for all variant canonicalization)
# =========================================================================

def normalize(text: str) -> str:
    text = re.sub(r"'([^']{1,10})'", r'『\1』', text)  # 'name' → 『name』
    text = text.replace('\u3000', '')  # full-width spaces
    text = re.sub(r'[\s　]+', '', text)  # collapse whitespace
    return text

# =========================================================================
# STRUCTURAL PRIMITIVES
# =========================================================================
# Each primitive is: (name, detect_fn, split_fn, annotate_fn)
#   detect_fn(text) → bool: does this primitive apply?
#   split_fn(text) → [str]: break text into parts
#   annotate_fn(parts, text) → dict: metadata about the structure

Primitive = Tuple[str, Callable, Callable, Callable]

def _has_cond(text): return any(m in text for m in ['場合、', 'とき、', 'なら、'])
def _split_cond(text):
    for kw in ['場合', 'とき', 'なら']:
        p = kw + '、'
        if p in text:
            idx = text.find(kw)
            return [text[:idx+len(kw)].strip(), text[idx+len(kw)+1:].strip()]
    return [text]

def _has_colon(text): return '：' in text
def _split_colon(text):
    parts = text.split('：', 1)
    return [p.strip() for p in parts]

def _has_seq_te(text):
    if _has_cond(text) or 'その中から' in text:
        return False
    if '、' not in text:
        return False
    first = text.split('、')[0].strip()
    return any(first.endswith(e) for e in ['き','ぎ','し','じ','ち','び','み','り','い','え'])


def _split_seq_te(text):
    return [p.strip() for p in text.split('、') if p.strip()]

def _has_duration(text): return 'かぎり' in text
def _split_duration(text):
    parts = text.split('かぎり', 1)
    return [parts[0].strip() + 'かぎり', parts[1].strip().lstrip('、')]

def _has_per_unit(text): return 'につき' in text or 'ごとに' in text
def _split_per_unit(text):
    m = re.search(r'(.+?)(につき|ごとに)', text)
    if m:
        return [m.group(1).strip(), text[m.end():].strip().lstrip('、')]
    return [text]

def _has_look_sel(text): return 'その中から' in text
def _split_look_sel(text):
    m = re.search(r'(.+?)その中から(.+)', text)
    if m:
        return [m.group(1).strip(), m.group(2).strip()]
    return [text]

def _has_choice(text): return '以下から1つを選ぶ' in text
def _split_choice(text):
    return text.split('以下から1つを選ぶ', 1)

def _has_multi_sent(text):
    s = [p.strip() for p in text.split('。') if p.strip()]
    return len(s) >= 2
def _split_multi_sent(text):
    return [p.strip() for p in text.split('。') if p.strip()]

def _has_alt(text): return '代わりに' in text
def _split_alt(text):
    parts = text.split('代わりに', 1)
    return [p.strip() for p in parts]

def _has_further(text): return 'さらに' in text
def _split_further(text):
    parts = [p.strip() for p in text.split('。') if p.strip()]
    result = []
    for p in parts:
        p = p.replace('さらに', '', 1).strip() if 'さらに' in p else p
        result.append(p)
    return result

def _annotate_base(parts, text): return {}
def _annotate_cond(parts, text): return {'_cond_text': parts[0], '_is_conditional': True}
def _annotate_colon(parts, text): return {'_cost_text': parts[0], '_effect_text': parts[1], '_is_cost_effect': True}
def _annotate_seq_te(parts, text): return {'_is_sequential': True, '_seq_type': 'te'}
def _annotate_duration(parts, text): return {'_duration_text': parts[0], '_is_duration': True}
def _annotate_per_unit(parts, text): return {'_is_per_unit': True}
def _annotate_look_sel(parts, text): return {'_look_text': parts[0], '_select_text': parts[1]}
def _annotate_choice(parts, text): return {'_is_choice': True}
def _annotate_multi_sent(parts, text): return {'_is_sequential': True, '_seq_type': 'period'}
def _annotate_alt(parts, text): return {'_is_alt': True}
def _annotate_further(parts, text): return {'_is_sequential': True, '_seq_type': 'further'}

PRIMITIVES: List[Primitive] = [
    ('FURTHER',  _has_further,    _split_further,    _annotate_further),
    ('COND',     _has_cond,       _split_cond,       _annotate_cond),
    ('COLON',    _has_colon,      _split_colon,      _annotate_colon),
    ('CHOICE',   _has_choice,     _split_choice,     _annotate_choice),
    ('LOOK_SEL', _has_look_sel,   _split_look_sel,   _annotate_look_sel),
    ('DURATION', _has_duration,   _split_duration,   _annotate_duration),
    ('ALT',      _has_alt,        _split_alt,        _annotate_alt),
    ('MULTI_',   _has_multi_sent, _split_multi_sent, _annotate_multi_sent),
    ('SEQ_TE',   _has_seq_te,     _split_seq_te,     _annotate_seq_te),
    ('PER_UNIT', _has_per_unit,   _split_per_unit,   _annotate_per_unit),
]

# =========================================================================
# STRUCTURE INTERPRETER
# =========================================================================

def match_structure(text: str) -> Dict[str, Any]:
    result = {'_raw': text}
    for name, detect, split, annotate in PRIMITIVES:
        if not detect(text):
            continue
        parts = split(text)
        meta = annotate(parts, text)
        # Store as list if multiple parts, else string
        if name in ('SEQ_TE', 'MULTI_', 'FURTHER'):
            result[f'_{name}_parts'] = parts
        else:
            for i, p in enumerate(parts):
                result[f'_{name}_part{i}'] = p
        result.update(meta)
        break  # first match wins
    return result

# =========================================================================
# SLOT EXTRACTORS (parameter-level)
# =========================================================================

def extract_source(text: str) -> Optional[str]:
    if '手札を' in text or '手札から' in text or '手札の' in text or '手札にある' in text:
        return 'hand'
    if '控え室' in text and ('から' in text or 'にある' in text or 'を' in text):
        return 'discard'
    if 'デッキの上' in text or 'デッキの一番上' in text:
        return 'deck_top'
    if 'デッキ' in text or '山札' in text:
        return 'deck'
    if 'デッキの一番下' in text:
        return 'deck_bottom'
    if 'ステージ' in text:
        return 'stage'
    if 'エネルギー置き場' in text:
        return 'energy_zone'
    if 'ライブカード置き場' in text:
        return 'live_card_zone'
    if '成功ライブカード置き場' in text:
        return 'success_live_zone'
    if '公開' in text:
        return 'revealed_cards'
    if '下に置かれている' in text:
        return 'under_member'
    return None

def extract_destination(text: str) -> Optional[str]:
    if '手札に加える' in text or '手札に加えて' in text or '手札に置く' in text:
        return 'hand'
    if '控え室に置く' in text or '控え室に置いて' in text or '控え室に送る' in text:
        return 'discard'
    if 'デッキの一番上' in text or 'デッキの上に置く' in text:
        return 'deck_top'
    if 'デッキの一番下に置く' in text:
        return 'deck_bottom'
    if 'デッキに置く' in text or 'デッキに戻す' in text:
        return 'deck'
    if 'ステージ' in text and '登場させる' in text:
        return 'stage'
    if 'エネルギー置き場' in text:
        return 'energy_zone'
    if 'ライブカード置き場' in text:
        return 'live_card_zone'
    if '成功ライブカード置き場' in text:
        return 'success_live_zone'
    if 'メンバーのいないエリア' in text:
        return 'empty_area'
    if 'そのメンバーがいたエリア' in text:
        return 'same_area'
    if 'このメンバーの下' in text:
        return 'under_member'
    return None

def extract_count(text: str) -> Optional[int]:
    m = re.search(r'(\d+)枚', text)
    if m: return int(m.group(1))
    m = re.search(r'(\d+)人', text)
    if m: return int(m.group(1))
    m = re.search(r'(\d+)つ', text)
    if m: return int(m.group(1))
    return None

def extract_card_type(text: str) -> Optional[str]:
    if 'メンバーカード' in text or ('メンバー' in text and '人' in text):
        return 'member_card'
    if 'ライブカード' in text:
        return 'live_card'
    if 'エネルギーカード' in text:
        return 'energy_card'
    if 'カード' in text and not ('ライブ' in text or 'メンバー' in text):
        return 'card'
    return None

def extract_target(text: str) -> Optional[str]:
    if '自分と相手' in text: return 'both'
    if '自分か相手' in text: return 'either'
    if '相手の' in text or '相手は' in text: return 'opponent'
    if '自分の' in text or '自分は' in text: return 'self'
    return None

def extract_optional(text: str) -> bool:
    return 'もよい' in text or 'てもよい' in text

def extract_state_change(text: str) -> Optional[str]:
    if 'ウェイト' in text: return 'wait'
    if 'アクティブ' in text: return 'active'
    return None

def extract_group(text: str) -> Optional[Dict]:
    m = re.search(r'『([^』]+)』', text)
    if m and len(m.group(1)) <= 15:
        return {'name': m.group(1)}
    return None

def extract_duration(text: str) -> Optional[str]:
    for p, c in [('ライブ終了時まで', 'live_end'), ('このターンの間', 'this_turn'),
                  ('このライブの間', 'this_live')]:
        if p in text: return c
    return None

# =========================================================================
# EFFECT PATTERNS (verb phrases → action types)
# =========================================================================
# Each: (trigger_words, action_name, filler_fn)
# filler_fn(text, slots) → updated slots

def fill_move(text, s):
    s['source'] = extract_source(text) or s.get('source', '')
    s['destination'] = extract_destination(text) or s.get('destination', '')
    ct = extract_card_type(text)
    if ct: s['card_type'] = ct
    c = extract_count(text)
    if c: s['count'] = c
    return s

def fill_draw(text, s):
    s['source'] = 'deck'
    s['destination'] = 'hand'
    tgt = extract_target(text)
    if tgt: s['target'] = tgt
    return s

def fill_gain(text, s):
    if '{{icon_blade' in text or 'ブレード' in text:
        s['resource'] = 'blade'
        s['count'] = text.count('{{icon_blade.png|ブレード}}') or 1
    elif '{{heart' in text or 'ハート' in text:
        s['resource'] = 'heart'
        s['count'] = len(re.findall(r'{{heart_\d+\.png\|heart\d+}}', text)) or 1
    elif '{{icon_energy' in text:
        s['resource'] = 'energy'
    dur = extract_duration(text)
    if dur: s['duration'] = dur
    s.setdefault('count', 1)
    g = extract_group(text)
    if g: s['group'] = g
    ct = extract_card_type(text)
    if ct: s['card_type'] = ct
    tgt = extract_target(text)
    if tgt: s['target'] = tgt
    return s

def fill_change_state(text, s):
    sc = extract_state_change(text)
    if sc: s['state_change'] = sc
    return s

def fill_modify_score(text, s):
    s['operation'] = 'add'
    vm = re.search(r'[＋+](\d+)', text)
    if vm: s['value'] = int(vm.group(1))
    else: s['value'] = 1
    if 'このカード' in text: s['self_target'] = True
    return s

def fill_restriction(text, s):
    if 'ライブできない' in text: s['restriction_type'] = 'cannot_live'
    elif 'アクティブにしない' in text: s['restriction_type'] = 'cannot_activate'
    elif 'バトンタッチで控え室に置けない' in text: s['restriction_type'] = 'cannot_baton_touch'
    elif '登場できない' in text: s['restriction_type'] = 'cannot_appear'
    elif '置けない' in text: s['restriction_type'] = 'cannot_place'
    elif '移動できない' in text: s['restriction_type'] = 'cannot_move'
    return s

def fill_position_change(text, s):
    tgt = extract_target(text)
    if tgt: s['target'] = tgt
    return s

def fill_reveal(text, s):
    src = extract_source(text)
    if src: s['source'] = src
    return s

def fill_select(text, s):
    return s

# Action keyword → (action_name, filler_fn)
ACTION_PATTERNS: List[Tuple[List[str], str, Callable]] = [
    (['シャッフル'], 'shuffle', lambda t,s: s),
    (['入れ替える', '入れ替えて'], 'swap', lambda t,s: s),
    (['無効にする', '無効にし'], 'invalidate_ability', lambda t,s: s),
    (['何もしない'], 'do_nothing', lambda t,s: s),
    (['引く', '引き', '引いてもよい'], 'draw_card', fill_draw),
    (['置く', '置いて', '置き', '加える', '加えて', '加え', '送る', '戻す'], 'move_cards', fill_move),
    (['公開する', '公開し'], 'reveal', fill_reveal),
    (['見る', '見て'], 'look_at', lambda t,s: s),
    (['選ぶ', '選ん'], 'select', fill_select),
    (['得る', '得て'], 'gain_resource', fill_gain),
    (['アクティブにする', 'ウェイトにする'], 'change_state', fill_change_state),
    (['ポジションチェンジ'], 'position_change', fill_position_change),
    (['ブレードを得る'], 'gain_resource', lambda t,s: s.update({'resource': 'blade'}) or s),
    (['ハートを得る'], 'gain_resource', lambda t,s: s.update({'resource': 'heart'}) or s),
]

def detect_action(text: str) -> Tuple[str, Callable]:
    for keywords, action, filler in ACTION_PATTERNS:
        for kw in keywords:
            if kw in text:
                return action, filler
    return 'custom', lambda t,s: s

# =========================================================================
# COST PARSER
# =========================================================================

def parse_cost(text: str) -> Dict:
    cost = {'text': text, 'type': 'custom'}
    if not text.strip():
        return cost
    # Energy cost
    en = text.count('{{icon_energy.png|E}}')
    if en and text.strip().startswith('{{icon_energy.png|E}}'):
        return {'text': text, 'type': 'pay_energy', 'energy': en}
    # Reveal cost
    if '公開する' in text or '公開し' in text:
        cost['type'] = 'reveal'
        cost['action'] = 'reveal'
        src = extract_source(text)
        if src: cost['source'] = src
        c = extract_count(text)
        if c: cost['count'] = c
        return cost
    # State change cost
    sc = extract_state_change(text)
    if sc and 'このメンバー' in text:
        cost['type'] = 'change_state'
        cost['state_change'] = sc
        cost['card_type'] = 'member_card'
        cost['self_cost'] = True
        return cost
    # Move cards cost
    src = extract_source(text)
    dst = extract_destination(text)
    if src or dst:
        cost['type'] = 'move_cards'
        cost['action'] = 'move_cards'
        if src: cost['source'] = src
        if dst: cost['destination'] = dst
        c = extract_count(text)
        if c: cost['count'] = c
        ct = extract_card_type(text)
        if ct: cost['card_type'] = ct
        if extract_optional(text): cost['optional'] = True
        if 'このメンバー' in text and 'このメンバー以外' not in text: cost['self_cost'] = True
        return cost
    # Sequential cost
    if '、' in text:
        parts = text.split('、')
        if len(parts) >= 2 and parts[0].strip().endswith('し'):
            costs = [parse_cost(p.strip()) for p in parts]
            return {'text': text, 'type': 'sequential_cost', 'costs': costs}
    return cost

# =========================================================================
# CONDITION PARSER
# =========================================================================

def parse_condition(text: str, context: Dict = None) -> Dict:
    if context is None: context = {}
    text = re.sub(r'[（）()]', '', text).strip()
    result = {'text': text}
    
    # Compound (かつ) — must be checked BEFORE individual patterns
    if 'かつ' in text:
        parts = [p.strip() for p in text.split('かつ') if p.strip()]
        if len(parts) >= 2:
            sub_ctx = {}
            if '人' in text:
                sub_ctx['location'] = 'stage'
                sub_ctx['card_type'] = 'member_card'
            parsed = [parse_condition(p, sub_ctx) for p in parts]
            for sub in parsed:
                for k, v in sub_ctx.items():
                    if k not in sub:
                        sub[k] = v
            return {'type': 'compound', 'operator': 'and', 'conditions': parsed, 'text': text}
    
    # Card count condition (人)
    m = re.search(r'(\d+)人以上いる', text)
    if not m:
        m = re.search(r'(\d+)人以上', text)
    if m:
        result['type'] = 'card_count_condition'
        result['count'] = int(m.group(1))
        result['operator'] = '>='
        result['unit'] = '人'
        result['card_type'] = 'member_card'
        return result
    # Card count condition (枚)
    m = re.search(r'(\d+)枚以上', text)
    if m:
        result['type'] = 'card_count_condition'
        result['count'] = int(m.group(1))
        result['operator'] = '>='
        return result
    # Distinct names
    if '名前が異なる' in text:
        result['type'] = 'location_condition'
        result['location'] = 'stage'
        result['target'] = 'self'
        result['distinct'] = True
        m = re.search(r'(\d+)(人|枚)', text)
        if m:
            result['count'] = int(m.group(1))
            result['operator'] = '>='
        return result
    # Location condition
    loc = extract_source(text) or extract_destination(text)
    if loc:
        result['type'] = 'location_condition'
        result['location'] = loc
        result.setdefault('target', 'self')
        return result
    # Temporal
    if 'このターン' in text:
        result['type'] = 'temporal_condition'
        result['temporal'] = 'this_turn'
        return result
    if 'ライブ中' in text:
        result['type'] = 'temporal_condition'
        result['temporal'] = 'during_live'
        return result
    
    # Fallthrough: comparison
    for op_text, op in [('以上', '>='), ('以下', '<='), ('より少ない', '<'),
                         ('より多い', '>'), ('未満', '<'), ('超', '>')]:
        if op_text in text:
            result['operator'] = op
            break
    result.setdefault('type', 'comparison_condition')
    result.setdefault('target', 'self')
    result.setdefault('count', 1)
    # Apply inherited context
    for k, v in context.items():
        if k not in result:
            result[k] = v
    return result

# =========================================================================
# EFFECT ASSEMBLER
# =========================================================================

def assemble_effect(struct: Dict) -> Dict:
    text = struct.get('_raw', '')
    result = {'text': text}
    
    # Cost-effect: parse cost + recurse effect
    if struct.get('_is_cost_effect'):
        cost_text = struct.get('_COLON_part0', '')
        effect_text = struct.get('_COLON_part1', '')
        result['cost'] = parse_cost(cost_text)
        sub = assemble_effect(match_structure(effect_text))
        result.update(sub)
        return result
    
    # Conditional: parse condition + recurse effect
    if struct.get('_is_conditional'):
        cond_text = struct.get('_COND_part0', '')
        action_text = struct.get('_COND_part1', '')
        result['condition'] = parse_condition(cond_text)
        sub = assemble_effect(match_structure(action_text))
        result.update(sub)
        return result
    
    # Sequential (te-form): recurse each part
    if struct.get('_is_sequential') and struct.get('_seq_type') == 'te':
        parts = struct.get('_SEQ_TE_parts', [])
        actions = [assemble_effect(match_structure(p)) for p in parts if p.strip()]
        if len(actions) >= 2:
            valid = [a for a in actions if a.get('action') != 'custom']
            if len(valid) >= 2:
                return {'text': text, 'action': 'sequential', 'actions': valid}
        return actions[0] if actions else result
    
    # Sequential (period): recurse each part
    if struct.get('_is_sequential') and struct.get('_seq_type') == 'period':
        parts = struct.get('_MULTI__parts', [])
        actions = [assemble_effect(match_structure(p)) for p in parts if p.strip()]
        if len(actions) >= 2:
            valid = [a for a in actions if a.get('action') != 'custom']
            if len(valid) >= 2:
                return {'text': text, 'action': 'sequential', 'actions': valid}
        return actions[0] if actions else result
    
    # Sequential (further): recurse each part
    if struct.get('_is_sequential') and struct.get('_seq_type') == 'further':
        parts = struct.get('_FURTHER_parts', [])
        actions = [assemble_effect(match_structure(p)) for p in parts if p.strip()]
        if len(actions) >= 2:
            valid = [a for a in actions if a.get('action') != 'custom']
            if len(valid) >= 2:
                return {'text': text, 'action': 'sequential', 'actions': valid}
        return actions[0] if actions else result
    
    # Duration: mark duration + recurse
    if struct.get('_is_duration'):
        cond_text = struct.get('_DURATION_part0', '')
        action_text = struct.get('_DURATION_part1', '')
        result['condition'] = parse_condition(cond_text)
        result['duration'] = 'as_long_as'
        sub = assemble_effect(match_structure(action_text))
        result.update(sub)
        return result
    
    # Per-unit
    if struct.get('_is_per_unit'):
        per_text = struct.get('_PER_UNIT_part0', '')
        act_text = struct.get('_PER_UNIT_part1', '')
        result['per_unit'] = True
        pm = re.search(r'(\d+)(人|枚|つ)', per_text)
        if pm:
            result['per_unit_count'] = int(pm.group(1))
            result['per_unit_type'] = pm.group(2)
        for kw, t in [('メンバー', 'member'), ('人', 'member'), ('カード', 'card'), ('枚', 'card')]:
            if kw in per_text:
                result['per_unit_type'] = t
                break
        sub = assemble_effect(match_structure(act_text))
        result.update(sub)
        return result
    
    # Look-and-select
    if '_look_text' in struct:
        look_text = struct.get('_look_text', '')
        select_text = struct.get('_select_text', '')
        look_action = extract_slots(look_text)
        look_action.setdefault('action', 'look_at')
        look_action.setdefault('source', 'deck_top')
        select_action = _build_select_action(select_text)
        return {
            'text': text, 'action': 'look_and_select',
            'look_action': look_action, 'select_action': select_action,
        }
    
    # Choice
    if struct.get('_is_choice'):
        parts = text.split('以下から1つを選ぶ', 1)
        result['action'] = 'choice'
        if len(parts) > 1:
            lines = [l.strip() for l in parts[1].split('\n') if l.strip() and l.startswith('・')]
            options = []
            for line in lines:
                ot = line[1:].strip()
                po = assemble_effect(match_structure(ot))
                po['text'] = ot
                options.append(po)
            if options:
                result['options'] = options
        return result
    
    # Default: extract slots + detect action
    slots = extract_slots(text)
    result.update(slots)
    
    # Action detection
    action, filler = detect_action(text)
    result['action'] = action
    filler(text, result)
    
    # Post-processing
    if result.get('action') == 'gain_resource':
        if 'resource' not in result:
            result['resource'] = 'generic'
        dur = extract_duration(text)
        if dur: result['duration'] = dur
        if 'target' not in result:
            result['target'] = 'self'
    
    if result.get('action') == 'move_cards':
        result.setdefault('count', 1)
        if not result.get('source'): result['source'] = extract_source(text) or '?'
        if not result.get('destination'): result['destination'] = extract_destination(text) or '?'
    
    if result.get('action') == 'modify_score':
        if 'self_target' not in result and 'このカード' in text:
            result['self_target'] = True
    
    if result.get('action') == 'draw_card':
        result.setdefault('count', 1)
    
    return result

def extract_slots(text: str) -> Dict:
    slots = {}
    c = extract_count(text)
    if c is not None: slots['count'] = c
    ct = extract_card_type(text)
    if ct: slots['card_type'] = ct
    tgt = extract_target(text)
    if tgt: slots['target'] = tgt
    if extract_optional(text): slots['optional'] = True
    sc = extract_state_change(text)
    if sc: slots['state_change'] = sc
    g = extract_group(text)
    if g: slots['group'] = g
    return slots

def _build_select_action(text: str) -> Dict:
    if '手札に加え' in text and '残りを控え室に置く' in text:
        parts = re.split(r'[、。]', text)
        fa = {'action': 'move_cards', 'destination': 'hand', 'source': 'looked_at', 'text': parts[0].strip() if parts else text}
        sa = {'action': 'move_cards', 'destination': 'discard', 'source': 'looked_at_remaining',
              'dynamic_count': {'type': 'remaining_looked_at', 'reference': 'previous_look'}, 'text': parts[1].strip() if len(parts) > 1 else text}
        return {'action': 'sequential', 'actions': [fa, sa], 'text': text}
    slots = extract_slots(text)
    slots['text'] = text
    slots.setdefault('action', 'move_cards')
    src = extract_source(text)
    if src: slots['source'] = src
    dst = extract_destination(text)
    if dst: slots['destination'] = dst
    return slots

# =========================================================================
# TOP-LEVEL API
# =========================================================================

def parse_ability(triggerless_text: str) -> Dict:
    text = normalize(triggerless_text).strip()
    text = re.sub(r'[。.]$', '', text).strip()
    result = {'triggerless_text': triggerless_text}
    
    struct = match_structure(text)
    effect = assemble_effect(struct)
    
    if effect.get('cost'):
        result['cost'] = effect.pop('cost')
    if effect:
        result['effect'] = effect
    
    return result

def process_abilities(data: Dict) -> Dict:
    for ability in data.get('unique_abilities', []):
        triggerless = ability.get('triggerless_text', '')
        if triggerless:
            parsed = parse_ability(triggerless)
            if 'effect' in parsed:
                ability['effect'] = parsed['effect']
            if 'cost' in parsed:
                ability['cost'] = parsed['cost']
    return data

if __name__ == '__main__':
    from pathlib import Path
    
    # Test on SUNNY DAY SONG
    import json
    data = json.load(open(Path(__file__).parent.parent / 'cards' / 'abilities.json', encoding='utf-8'))
    entry = data['unique_abilities'][523]
    tt = entry['triggerless_text']
    
    print("Testing SUNNY DAY SONG:")
    print(f"Text: {tt[:100]}...")
    print()
    
    parsed = parse_ability(tt)
    print(json.dumps(parsed.get('effect', {}), indent=2, ensure_ascii=False)[:2000])

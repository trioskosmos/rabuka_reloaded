"""
parser_v2.py — Slot-extraction based ability parser.

Key principle: "move discard→hand" and "move discard→hand (up to 3, cost ≤4, DOLLCHESTRA)"
are the SAME core operation with different filters. Not separate templates.

Architecture:
  1. NORMALIZE — canonicalize variants, one function
  2. STRUCTURE — detect overall shape (cond, colon, sequential, simple)
  3. SLOT EXTRACTION — independent extractors for each field type
  4. ASSEMBLY — combine structure + slots into output
"""

import re
import json
from typing import Any, Dict, Optional, List
from pathlib import Path

# =========================================================================
# NORMALIZATION
# =========================================================================

def normalize(text: str) -> str:
    text = re.sub(r"'([^']{1,10})'", r'『\1』', text)
    return text.strip()

# =========================================================================
# SLOT EXTRACTORS — independent, table-driven
# =========================================================================

def extract_source(text: str) -> Optional[str]:
    for p, z in [
        ('手札を', 'hand'), ('手札から', 'hand'), ('手札の', 'hand'), ('手札にある', 'hand'),
        ('控え室から', 'discard'), ('控え室にある', 'discard'), ('控え室にある', 'discard'),
        ('デッキの一番上から', 'deck_top'), ('デッキの上から', 'deck_top'), ('デッキの一番下から', 'deck_bottom'),
        ('デッキから', 'deck'), ('山札から', 'deck'), ('ステージから', 'stage'),
        ('エネルギー置き場から', 'energy_zone'), ('ライブカード置き場から', 'live_card_zone'),
        ('成功ライブカード置き場から', 'success_live_zone'),
    ]:
        if p in text: return z
    if '公開' in text and 'カード' in text: return 'revealed_cards'
    if 'これにより公開されたほかのすべて' in text: return 'revealed_remaining'
    if 'その中から' in text and ('加え' in text or '置き' in text or '手札' in text): return 'looked_at'

def extract_destination(text: str) -> Optional[str]:
    for p, z in [
        ('手札に加える', 'hand'), ('手札に加えて', 'hand'), ('手札に置く', 'hand'),
        ('控え室に', 'discard'), ('控え室に送る', 'discard'),
        ('デッキの一番上に', 'deck_top'), ('デッキの上に', 'deck_top'),
        ('デッキの一番下に', 'deck_bottom'), ('デッキの下に', 'deck_bottom'),
        ('デッキに戻す', 'deck'), ('デッキに置く', 'deck'),
        ('ステージに登場させる', 'stage'), ('登場させる', 'stage'),
        ('エネルギー置き場に', 'energy_zone'), ('ライブカード置き場に', 'live_card_zone'),
        ('成功ライブカード置き場に', 'success_live_zone'),
        ('メンバーのいないエリア', 'empty_area'), ('そのメンバーがいたエリア', 'same_area'),
        ('このメンバーの下に', 'under_member'),
    ]:
        if p in text: return z
    if 'その中から' in text and ('加え' in text or '手札' in text): return 'hand'

def extract_count(text: str) -> Optional[int]:
    m = re.search(r'(\d+)枚', text)
    if m: return int(m.group(1))
    m = re.search(r'(\d+)人', text)
    if m: return int(m.group(1))
    m = re.search(r'(\d+)つ', text)
    if m: return int(m.group(1))

def extract_card_type(text: str) -> Optional[str]:
    for p, ct in [('メンバーカード', 'member_card'), ('メンバー', 'member_card'),
                   ('ライブカード', 'live_card'), ('エネルギーカード', 'energy_card')]:
        if p in text: return ct
    if 'カード' in text: return 'card'

def extract_target(text: str) -> Optional[str]:
    if '自分と相手' in text: return 'both'
    if '自分か相手' in text: return 'either'
    if '相手' in text: return 'opponent'
    if '自分' in text: return 'self'

def extract_optional(text: str) -> bool:
    return 'もよい' in text or 'てもよい' in text

def extract_state_change(text: str) -> Optional[str]:
    if 'ウェイト' in text: return 'wait'
    if 'アクティブ' in text: return 'active'

def extract_group(text: str) -> Optional[Dict]:
    m = re.search(r'『([^』]+)』', text)
    if m and len(m.group(1)) <= 15: return {'name': m.group(1)}

def extract_duration(text: str) -> Optional[str]:
    for p, c in [('ライブ終了時まで', 'live_end'), ('このターンの間', 'this_turn'),
                  ('このライブの間', 'this_live')]:
        if p in text: return c

def extract_resource(text: str) -> Optional[str]:
    if '{{icon_blade' in text or 'ブレード' in text: return 'blade'
    if '{{heart' in text or 'ハート' in text: return 'heart'
    if '{{icon_energy' in text: return 'energy'

def extract_cost_limit(text: str) -> Optional[int]:
    m = re.search(r'コスト(\d+)', text)
    if m: return int(m.group(1))

def extract_cost_limit_op(text: str) -> Optional[str]:
    for op in ['以下', '以上', '未満', '超']:
        if op in text: return {'以下': '<=', '以上': '>=', '未満': '<', '超': '>'}[op]

def extract_placement_order(text: str) -> Optional[str]:
    if '好きな順番で' in text: return 'any_order'

# =========================================================================
# STRUCTURE DETECTION
# =========================================================================

def structure_type(text: str) -> str:
    if not text.strip(): return 'empty'
    # Sequential (さらに) must be checked BEFORE conditional,
    # because sentences like "if X, A。if Y, further B" are 3-branch sequential
    if 'さらに' in text:
        parts = [p.strip() for p in text.split('。') if p.strip()]
        if len(parts) >= 2:
            return 'sequential'
    if '：' in text and not any(m in text for m in ['場合、', 'とき、', 'なら、']):
        return 'cost_action'
    if any(m in text for m in ['場合、', 'とき、', 'なら、']):
        return 'conditional'
    if '以下から1つを選ぶ' in text: return 'choice'
    if 'その中から' in text: return 'look_select'
    if 'かぎり' in text: return 'duration'
    if 'につき' in text or 'ごとに' in text: return 'per_unit'
    if '。' in text:
        parts = [p.strip() for p in text.split('。') if p.strip()]
        if len(parts) >= 2: return 'sequential'
    if '、' in text:
        first = text.split('、')[0].strip()
        if any(first.endswith(e) for e in ['き','ぎ','し','じ','ち','び','み','り','い','え']):
            return 'sequential'
    return 'simple'

def split_structure(text: str, stype: str) -> List[str]:
    if stype == 'cost_action':
        parts = text.split('：', 1)
        return [p.strip() for p in parts]
    if stype == 'conditional':
        for kw in ['場合', 'とき', 'なら']:
            p = kw + '、'
            if p in text:
                idx = text.find(kw)
                return [text[:idx+len(kw)].strip(), text[idx+len(kw)+1:].strip()]
        return [text]
    if stype in ('sequential',):
        if 'さらに' in text:
            parts = [p.strip() for p in text.split('。') if p.strip()]
            result = []
            for p in parts:
                p = p.replace('さらに', '', 1).strip() if 'さらに' in p else p
                result.append(p)
            return result
        if '。' in text:
            return [p.strip() for p in text.split('。') if p.strip()]
        if '、' in text:
            return [p.strip() for p in text.split('、') if p.strip()]
        return [text]
    if stype == 'duration':
        parts = text.split('かぎり', 1)
        return [parts[0].strip() + 'かぎり', parts[1].strip().lstrip('、')]
    if stype == 'per_unit':
        m = re.search(r'(.+?)(につき|ごとに)', text)
        if m: return [m.group(1).strip(), text[m.end():].strip().lstrip('、')]
    if stype == 'look_select':
        m = re.search(r'(.+?)その中から(.+)', text)
        if m: return [m.group(1).strip(), m.group(2).strip()]
    if stype == 'choice':
        return text.split('以下から1つを選ぶ', 1)
    return [text]

# =========================================================================
# ACTION DETECTION
# =========================================================================

ACTION_TRIGGERS = [
    ('シャッフル', 'shuffle'), ('入れ替える', 'swap'), ('入れ替えて', 'swap'),
    ('無効にする', 'invalidate_ability'), ('何もしない', 'do_nothing'),
    ('引いてもよい', 'draw_card'), ('引く', 'draw_card'), ('引き', 'draw_card'),
    ('置く', 'move_cards'), ('置いて', 'move_cards'), ('加える', 'move_cards'),
    ('加えて', 'move_cards'), ('送る', 'move_cards'), ('戻す', 'move_cards'),
    ('公開する', 'reveal'), ('公開し', 'reveal'), ('見る', 'look_at'), ('見て', 'look_at'),
    ('選ぶ', 'select'), ('選ん', 'select'),
    ('得る', 'gain_resource'), ('得て', 'gain_resource'),
    ('アクティブにする', 'change_state'), ('ウェイトにする', 'change_state'),
    ('ポジションチェンジ', 'position_change'),
    ('スコアを', 'modify_score'),
]

def detect_action(text: str) -> str:
    for trigger, action in ACTION_TRIGGERS:
        if trigger in text: return action
    return 'custom'

# =========================================================================
# CONDITION PARSING
# =========================================================================

def parse_condition(text: str, compound_ctx: Dict = None) -> Dict:
    text = re.sub(r'[（）()]', '', text).strip()
    r = {'text': text}

    if 'かつ' in text:
        parts = [p.strip() for p in text.split('かつ') if p.strip()]
        if len(parts) >= 2:
            ctx = {}
            if '人' in text: ctx = {'location': 'stage', 'card_type': 'member_card'}
            if compound_ctx: ctx.update(compound_ctx)
            parsed = [parse_condition(p, ctx) for p in parts]
            for sub in parsed:
                for k, v in ctx.items():
                    if k not in sub: sub[k] = v
            return {'type': 'compound', 'operator': 'and', 'conditions': parsed, 'text': text}

    m = re.search(r'(\d+)人以上いる', text)
    if not m: m = re.search(r'(\d+)人以上', text)
    if m: return {'type': 'card_count_condition', 'count': int(m.group(1)),
                  'operator': '>=', 'unit': '人', 'card_type': 'member_card', 'text': text}

    m = re.search(r'(\d+)枚以上', text)
    if m:
        r2 = {'type': 'card_count_condition', 'count': int(m.group(1)), 'operator': '>=', 'text': text}
        ct = extract_card_type(text)
        if ct: r2['card_type'] = ct
        return r2

    if '名前が異なる' in text:
        r2 = {'type': 'location_condition', 'location': 'stage', 'target': 'self', 'distinct': True, 'text': text}
        m = re.search(r'(\d+)(人|枚)', text)
        if m: r2['count'] = int(m.group(1)); r2['operator'] = '>='
        return r2

    loc = extract_source(text) or extract_destination(text)
    if loc:
        r2 = {'type': 'location_condition', 'location': loc, 'target': extract_target(text) or 'self', 'text': text}
        ct = extract_card_type(text)
        if ct: r2['card_type'] = ct
        cl = extract_cost_limit(text)
        if cl: r2['cost_limit'] = cl
        return r2

    if 'このターン' in text:
        return {'type': 'temporal_condition', 'temporal': 'this_turn', 'text': text}

    r.setdefault('type', 'comparison_condition')
    r.setdefault('target', 'self')
    r.setdefault('count', 1)
    r.setdefault('operator', '>=')
    if compound_ctx:
        for k, v in compound_ctx.items():
            if k not in r: r[k] = v
    return r

# =========================================================================
# COST PARSING
# =========================================================================

def parse_cost(text: str) -> Dict:
    if not text.strip(): return {'text': text, 'type': 'custom'}
    r = {'text': text, 'type': 'custom'}

    en = text.count('{{icon_energy.png|E}}')
    if en and text.strip().startswith('{{icon_energy.png|E}}'):
        r.update({'type': 'pay_energy', 'energy': en})
        if extract_optional(text): r['optional'] = True
        return r

    sc = extract_state_change(text)
    if sc and 'このメンバー' in text:
        r.update({'type': 'change_state', 'action': 'change_state', 'state_change': sc, 'card_type': 'member_card', 'self_cost': True})
        if extract_optional(text): r['optional'] = True
        return r

    src = extract_source(text)
    dst = extract_destination(text)
    if src or dst:
        r.update({'type': 'move_cards', 'action': 'move_cards'})
        if src: r['source'] = src
        if dst: r['destination'] = dst
        c = extract_count(text)
        if c: r['count'] = c
        ct = extract_card_type(text)
        if ct: r['card_type'] = ct
        if extract_optional(text): r['optional'] = True
        if 'このメンバー' in text: r['self_cost'] = True
        return r

    return r

# =========================================================================
# EFFECT ASSEMBLER
# =========================================================================

def assemble_action(text: str) -> Dict:
    slots = {'text': text}
    action = detect_action(text)
    slots['action'] = action

    if action == 'move_cards':
        slots.setdefault('source', extract_source(text) or '')
        slots.setdefault('destination', extract_destination(text) or '')
        c = extract_count(text)
        if c is not None: slots['count'] = c
        ct = extract_card_type(text)
        if ct: slots['card_type'] = ct
        if extract_optional(text): slots['optional'] = True
        sc = extract_state_change(text)
        if sc: slots['state_change'] = sc
        tgt = extract_target(text)
        if tgt: slots['target'] = tgt
        po = extract_placement_order(text)
        if po: slots['placement_order'] = po
        cl = extract_cost_limit(text)
        if cl: slots['cost_limit'] = cl
        g = extract_group(text)
        if g: slots['group'] = g
        slots.setdefault('count', 1)

    elif action == 'draw_card':
        slots.setdefault('source', 'deck')
        slots.setdefault('destination', 'hand')
        slots.setdefault('count', 1)
        tgt = extract_target(text)
        if tgt: slots['target'] = tgt

    elif action == 'gain_resource':
        slots['resource'] = extract_resource(text) or 'generic'
        slots.setdefault('count', len(re.findall(r'{{heart_\d+\.png\|heart\d+}}', text)) or 1)
        dur = extract_duration(text)
        if dur: slots['duration'] = dur
        tgt = extract_target(text)
        if tgt: slots['target'] = tgt
        ct = extract_card_type(text)
        if ct: slots['card_type'] = ct
        g = extract_group(text)
        if g: slots['group'] = g
        if 'このカード' in text: slots['self_target'] = True

    elif action == 'change_state':
        sc = extract_state_change(text)
        if sc: slots['state_change'] = sc

    elif action == 'modify_score':
        slots['operation'] = 'add'
        vm = re.search(r'[＋+](\d+)', text)
        slots['value'] = int(vm.group(1)) if vm else 1
        if 'このカード' in text: slots['self_target'] = True

    return slots

def parse_effect(text: str, depth: int = 0) -> Dict:
    if depth > 20: return assemble_action(text)
    text = normalize(text).strip()
    text = re.sub(r'[。.]$', '', text).strip()
    if not text: return {'action': 'do_nothing', 'text': ''}

    stype = structure_type(text)

    # Sequential (さらに or period-split) must be checked FIRST
    # to catch multi-sentence texts before individual sentences
    if stype == 'sequential':
        parts = split_structure(text, stype)
        if len(parts) >= 2:
            actions = [parse_effect(p, depth + 1) for p in parts if p.strip()]
            valid = [a for a in actions if a.get('action') not in ('custom', None)]
            if len(valid) >= 2:
                return {'text': text, 'action': 'sequential', 'actions': valid}
            if valid:
                return valid[0]
        return assemble_action(text)

    if stype == 'cost_action':
        parts = split_structure(text, stype)
        if len(parts) >= 2:
            result = {'text': text}
            result['cost'] = parse_cost(parts[0])
            sub = parse_effect(parts[1], depth + 1)
            result.update(sub)
            return result

    if stype == 'conditional':
        parts = split_structure(text, stype)
        if len(parts) >= 2:
            cond = parse_condition(parts[0])
            sub = parse_effect(parts[1], depth + 1)
            sub['condition'] = cond
            sub['text'] = text
            return sub

    if stype == 'duration':
        parts = split_structure(text, stype)
        if len(parts) >= 2:
            cond = parse_condition(parts[0])
            sub = parse_effect(parts[1], depth + 1)
            sub['condition'] = cond
            sub['duration'] = 'as_long_as'
            sub['text'] = text
            return sub

    if stype == 'per_unit':
        parts = split_structure(text, stype)
        result = {'text': text, 'per_unit': True}
        if len(parts) >= 2:
            pm = re.search(r'(\d+)(人|枚|つ)', parts[0])
            if pm:
                result['per_unit_count'] = int(pm.group(1))
                result['per_unit_type'] = pm.group(2)
            for kw, t in [('メンバー', 'member'), ('人', 'member'), ('カード', 'card')]:
                if kw in parts[0]: result['per_unit_type'] = t; break
            sub = parse_effect(parts[1], depth + 1)
            result.update(sub)
            return result

    if stype == 'look_select':
        parts = split_structure(text, stype)
        if len(parts) >= 2:
            look = assemble_action(parts[0])
            look.setdefault('action', 'look_at')
            look.setdefault('source', 'deck_top')
            sel = _build_select_action(parts[1])
            return {'text': text, 'action': 'look_and_select',
                    'look_action': look, 'select_action': sel}

    if stype == 'choice':
        parts = split_structure(text, stype)
        result = {'text': text, 'action': 'choice'}
        if len(parts) > 1:
            lines = [l.strip() for l in parts[1].split('\n') if l.strip() and l.startswith('・')]
            options = [parse_effect(l[1:].strip(), depth + 1) for l in lines]
            if options: result['options'] = options
        return result

    return assemble_action(text)

def _build_select_action(text: str) -> Dict:
    if '手札に加え' in text and '残りを控え室に置く' in text:
        parts = re.split(r'[、。]', text)
        if len(parts) >= 2:
            return {'action': 'sequential', 'text': text, 'actions': [
                {'action': 'move_cards', 'destination': 'hand', 'source': 'looked_at', 'text': parts[0].strip()},
                {'action': 'move_cards', 'destination': 'discard', 'source': 'looked_at_remaining',
                 'dynamic_count': {'type': 'remaining_looked_at', 'reference': 'previous_look'}, 'text': parts[1].strip()},
            ]}
    if '好きな枚数を好きな順番でデッキの上に置き' in text and '残りを控え室に置く' in text:
        parts = text.split('、', 1)
        if len(parts) >= 2:
            return {'action': 'sequential', 'text': text, 'actions': [
                {'action': 'move_cards', 'destination': 'deck_top', 'any_number': True, 'source': 'looked_at', 'text': parts[0].strip()},
                {'action': 'move_cards', 'destination': 'discard', 'source': 'looked_at_remaining',
                 'dynamic_count': {'type': 'remaining_looked_at', 'reference': 'previous_look'}, 'text': parts[1].strip()},
            ]}
    slots = assemble_action(text)
    slots.setdefault('action', 'move_cards')
    return slots

# =========================================================================
# TOP-LEVEL API
# =========================================================================

def parse_ability(triggerless_text: str) -> Dict:
    result = {'triggerless_text': triggerless_text}
    text = triggerless_text.strip()
    if not text: return result

    cost_text, effect_text = '', text
    if '：' in text:
        idx = text.find('：')
        cost_text = text[:idx].strip()
        effect_text = text[idx+1:].strip()

    if cost_text:
        result['cost'] = parse_cost(cost_text)
    if effect_text:
        effect = parse_effect(effect_text)
        result['effect'] = effect
        if isinstance(effect, dict) and 'cost' in effect:
            result['cost'] = effect.pop('cost')
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
    abilities_file = Path(__file__).resolve().parent.parent / 'cards' / 'abilities.json'
    with open(abilities_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
    result = process_abilities(data)
    out = Path(__file__).parent / 'abilities_v2.json'
    with open(out, 'w', encoding='utf-8') as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    # Show Sunny Day Song
    entry = result['unique_abilities'][523]
    print("SUNNY DAY SONG:")
    print(json.dumps(entry.get('effect', {}), indent=2, ensure_ascii=False))

"""Full ability parser using template-based approach.

Pipeline:
1. Split cost/effect on '：'
2. Parse cost (template-based)
3. Parse effect (composition → atomic templates)
4. Parse conditions (template-based)

The output is a tree of template IDs + slots, which IS invertible.
"""

from __future__ import annotations
import re
from typing import Any, Dict, List, Optional

from templates import match_atomic, AtomicTemplate, ATOMICS

# ------------------------------------------------------------------
# Cost patterns
# ------------------------------------------------------------------

COST_TEMPLATES: List[AtomicTemplate] = [
    AtomicTemplate('cost_energy', 'pay_energy',
        r'\{\{icon_energy\.png\|E\}\}+(?P<optional>もよい)?',
        aliases={'__energy_count': 'energy'}),
    # Actually energy count is the number of icon repetitions
]

def count_energy_icons(text: str) -> int:
    """Count energy icon occurrences for pay_energy cost."""
    return text.count('{{icon_energy.png|E}}')

def parse_cost(text: str) -> Optional[Dict]:
    """Parse a cost text. Returns cost dict or None."""
    if '{{icon_energy.png|E}}' in text:
        energy = count_energy_icons(text)
        optional = 'もよい' in text or 'してもよい' in text
        return {
            'type': 'pay_energy',
            'energy': energy,
            'optional': optional,
        }
    # Sequential cost: energy + action
    if '{{icon_energy.png|E}}' in text:
        rest = re.sub(r'^\{\{icon_energy\.png\|E\}\}+', '', text).strip()
        if rest:
            return {
                'type': 'sequential_cost',
                'costs': [
                    {'type': 'pay_energy', 'energy': count_energy_icons(text),
                     'optional': 'もよい' in rest},
                    parse_cost(rest) or {'type': 'custom', 'text': rest},
                ]
            }
    # change_state cost (wait)
    if 'ウェイトにする' in text or 'ウェイトにし' in text:
        optional = 'もよい' in text
        result = {'type': 'change_state', 'state_change': 'wait',
                  'card_type': 'member_card', 'self_cost': True}
        if optional:
            result['optional'] = True
        return result
    # move_cards cost (discard from stage or hand)
    src = None
    dst = None
    if 'ステージから' in text and '控え室に置く' in text:
        src, dst = 'stage', 'discard'
    elif '手札を' in text and '控え室に置く' in text:
        src, dst = 'hand', 'discard'
    elif '手札を' in text and '控え室に置いて' in text:
        src, dst = 'hand', 'discard'
    if src and dst:
        result = {'type': 'move_cards', 'source': src, 'destination': dst,
                  'card_type': 'member_card', 'self_cost': True}
        cnt_match = re.search(r'(\d+)枚', text)
        if cnt_match:
            result['count'] = int(cnt_match.group(1))
        if 'もよい' in text or 'てもよい' in text:
            result['optional'] = True
        return result
    return None

# ------------------------------------------------------------------
# Condition patterns
# ------------------------------------------------------------------

CONDITION_PATTERNS: List[AtomicTemplate] = [
    # Baton touch + ability negation (MUST be before plain baton touch)
    AtomicTemplate('cond_baton_touch_no_ability', 'location_condition',
        r'(?P<no_ability>能力を持たない)メンバーから(?P<baton_touch>バトンタッチして登場した)場合',
        defaults={'location': 'stage', 'target': 'self',
                  'baton_touch_trigger': True,
                  'ability_negation': True,
                  'type': 'location_condition'}),

    # Baton touch
    AtomicTemplate('cond_baton_touch', 'location_condition',
        r'(?P<baton_touch>バトンタッチして登場した)場合',
        defaults={'location': 'stage', 'target': 'self',
                  'baton_touch_trigger': True,
                  'type': 'location_condition'}),

    # Card count (N人以上)
    AtomicTemplate('cond_card_count', 'card_count_condition',
        r'(?P<count>\d+)(?P<unit>人|枚)(?:以上|いる)',
        defaults={'operator': '>=', 'type': 'card_count_condition'}),

    # Appeared
    AtomicTemplate('cond_appeared', 'appearance_condition',
        r'(?P<location>\w+)?に(?P<appearance>登場している|登場した)場合',
        defaults={'type': 'appearance_condition'}),

    # Temporal (this turn)
    AtomicTemplate('cond_this_turn', 'temporal_condition',
        r'このターン',
        defaults={'temporal': 'this_turn', 'type': 'temporal_condition'}),

    # Temporal (during live)
    AtomicTemplate('cond_during_live', 'temporal_condition',
        r'ライブ中',
        defaults={'temporal': 'during_live', 'type': 'temporal_condition'}),

    # Compound (かつ)
    AtomicTemplate('cond_compound', 'compound',
        r'かつ',
        defaults={'operator': 'and', 'type': 'compound'}),

    # Exclude self
    AtomicTemplate('cond_exclude_self', 'location_condition',
        r'このメンバー以外の(?P<card_type>\w+)?が(?P<count>\d+)(?:人|枚)',
        defaults={'exclude_self': True, 'type': 'location_condition'}),
]

def parse_condition(text: str) -> Optional[Dict]:
    """Parse a condition prefix (before 場合、/とき、/なら、)."""
    for template in CONDITION_PATTERNS:
        result = template.match(text)
        if result:
            return result
    # Generic fallback: detect location
    loc = None
    if 'ステージ' in text: loc = 'stage'
    elif '控え室' in text: loc = 'discard'
    elif '手札' in text: loc = 'hand'
    if loc:
        return {'type': 'location_condition', 'location': loc, 'text': text}
    return {'type': 'custom', 'text': text}

# ------------------------------------------------------------------
# Composition: split effect text into atomic parts
# ------------------------------------------------------------------

def split_cost_effect(text: str):
    """Split on '：' to get cost and effect parts."""
    if '：' not in text:
        return '', text
    paren_depth = 0
    for i, ch in enumerate(text):
        if ch in '（(': paren_depth += 1
        elif ch in '）)': paren_depth -= 1
        elif ch == '：' and paren_depth == 0:
            return text[:i].strip(), text[i+1:].strip()
    return '', text

# ------------------------------------------------------------------
# Full ability parser
# ------------------------------------------------------------------

def _strip_icons(text: str) -> str:
    return re.sub(r'\{\{[^}]+\}\}', '', text).strip()

def parse_ability_text(triggerless_text: str) -> Dict:
    """Parse a complete ability text (no trigger icons)."""
    result = {'triggerless_text': triggerless_text}
    
    # 0. Strip icons but keep a copy for icon-based detection
    text_clean = _strip_icons(triggerless_text)
    text_icon_preserved = triggerless_text  # keep original for icon matching
    
    # 1. Split cost/effect (use icon-preserved text so energy icons are found)
    cost_text, effect_text = split_cost_effect(triggerless_text)
    
    # 2. Parse cost
    if cost_text:
        cost = parse_cost(cost_text)
        if cost:
            result['cost'] = cost
    
    # 3. Parse effect
    if effect_text:
        condition = parse_effect_with_conditions(effect_text, result)
    
    return result


def _parse_single_action(text: str) -> Optional[Dict]:
    """Parse a single action text, stripping duration prefix first."""
    clean = text.rstrip('。').strip()
    # Strip duration prefix
    for prefix in ['ライブ終了時まで', 'ライブ終了まで', 'このターンの間', 'このライブの間']:
        if clean.startswith(prefix):
            clean = clean[len(prefix):].lstrip('、').strip()
            break
    return match_atomic(clean)


def _try_conditional_sequential(effect_text: str) -> Optional[Dict]:
    """Try そうした場合 pattern."""
    if 'そうした場合' not in effect_text:
        return None
    parts = effect_text.split('そうした場合', 1)
    fa_text = parts[0].strip().rstrip('。').strip()
    sa_text = parts[1].strip().lstrip('、').strip()
    fa = _parse_single_action(fa_text)
    if not fa:
        # If first part doesn't match a template, use as custom
        fa = {'action': 'custom', 'text': fa_text}
    sa = _parse_single_action(sa_text)
    if not sa:
        sa = {'action': 'custom', 'text': sa_text}
    return {'action': 'sequential', 'actions': [fa, sa], 'conditional': True}


def parse_effect_with_conditions(effect_text: str, result: Dict) -> Optional[Dict]:
    """Parse effect text, handling conditions, sequential, and atomic matches."""
    # Check for condition prefix: X場合、/Xとき、/Xなら、
    # BUT exclude そうした場合、 which is a sequential marker, not a condition
    condition = None
    for marker in ['場合、', 'とき、', 'なら、']:
        idx = effect_text.find(marker)
        if idx == -1:
            continue
        # Check this is not そうした場合、
        if marker == '場合、' and idx >= 4 and effect_text[idx-4:idx+len(marker)] == 'そうした場合、':
            continue  # skip this occurrence, look for next
        cond_text = effect_text[:idx + len(marker)]
        effect_text = effect_text[idx + len(marker):].strip()
        condition = parse_condition(cond_text)
        break
    
    clean = effect_text.rstrip('。').strip()
    
    # Try conditional sequential (そうした場合) first
    seq_result = _try_conditional_sequential(clean)
    if seq_result:
        result['effect'] = seq_result
        if condition:
            result['effect']['condition'] = condition
        return condition
    
    # Try split on 。first (period separates multi-sentence effects like "look. select. discard")
    if '。' in clean:
        period_parts = [p.strip() for p in clean.split('。') if p.strip()]
        if len(period_parts) >= 2:
            actions = []
            for pp in period_parts:
                a = _parse_single_action(pp)
                if not a and '、' in pp:
                    # Second-level split for remaining comma-separated actions
                    sub_parts = [p.strip() for p in pp.split('、') if p.strip()]
                    for sp in sub_parts:
                        sa = _parse_single_action(sp)
                        if sa:
                            actions.append(sa)
                elif a:
                    actions.append(a)
            if len(actions) >= 2:
                result['effect'] = {'action': 'sequential', 'actions': actions, 'text': clean}
                if condition:
                    result['effect']['condition'] = condition
                return condition
    
    # Try sequential split on 、 FIRST (before full match, since full text
    # may contain multiple actions like "draw、discard")
    if '、' in clean:
        # Check for その後, which indicates a temporal boundary
        if 'その後' in clean:
            after_parts = clean.split('その後', 1)
            first_block = after_parts[0].strip()
            second_block = after_parts[1].strip().lstrip('、')
            parts = [p.strip() for p in first_block.split('、') if p.strip()]
            parts.append(second_block)
        else:
            parts = [p.strip() for p in clean.split('、') if p.strip()]
        actions = []
        for p in parts:
            a = _parse_single_action(p)
            if a:
                actions.append(a)
        if len(actions) >= 2:
            result['effect'] = {
                'action': 'sequential',
                'actions': actions,
                'text': clean,
            }
            if condition:
                result['effect']['condition'] = condition
            return condition
    
    # Try parsing the full text as one action
    action = _parse_single_action(clean)
    if action:
        result['effect'] = action
        if condition:
            result['effect']['condition'] = condition
        return condition
    
    # Last resort
    result['effect'] = _parse_single_action(clean) or {'action': 'custom', 'text': clean}
    if condition and result['effect'].get('action') != 'custom':
        result['effect']['condition'] = condition
    return condition

"""Mapper - converts annotated clauses to engine JSON using template matching."""
from .annotator import Clause, classify


def build(clauses):
    ability = {}
    role_map = {}
    for c in clauses:
        role_map.setdefault(c.role, []).append(c)

    if 'cost' in role_map:
        c = _build_cost(role_map['cost'][0])
        if c: ability['cost'] = c

    effect = _build_effect(role_map, clauses)
    if effect: ability['effect'] = effect
    return ability


def _build_cost(clause):
    classify(clause)
    t = clause.text

    # Sequential cost (te-form chain or 、-separated)
    if '、' in t:
        parts = [p.strip() for p in t.split('、') if p.strip()]
        if len(parts) >= 2:
            costs = []
            for p in parts:
                sc = Clause(role='cost', text=p.rstrip('し').strip() if p.endswith('し') else p)
                classify(sc)
                costs.append(_build_cost_item(sc))
            costs = [c for c in costs if c]
            if len(costs) >= 2:
                return {'text': t, 'type': 'sequential_cost', 'costs': costs}

    # Energy + action combined: "{{icon_energy.png|E}}{{icon_energy.png|E}}手札を1枚控え室に置く"
    if '{{icon_energy.png|E}}' in t and t.strip().startswith('{{icon_energy.png|E}}'):
        energy_end = t.rfind('}}', t.rfind('{{icon_energy.png|E}}')) + 2
        et = t[:energy_end].strip()
        ot = t[energy_end:].strip()
        if ot and ot != et:
            ec = Clause(role='cost', text=et)
            oc = Clause(role='cost', text=ot)
            classify(ec); classify(oc)
            return {'text': t, 'type': 'sequential_cost', 'costs': [
                _build_cost_item(ec), _build_cost_item(oc)]}

    return _build_cost_item(clause)


def _build_cost_item(clause):
    p = clause.params
    t = clause.text
    c = clause

    if '{{icon_energy.png|E}}' in t and ('支払う' in t or 'E}}' in t):
        return {'type': 'pay_energy', 'text': t, 'energy': p.get('energy_count', 1),
                'optional': True if p.get('optional') else None}

    if '公開する' in t or '公開し' in t:
        return {'type': 'reveal', 'text': t, 'source': c.source or 'hand',
                'count': c.count or 1, 'card_type': c.card_type}

    if '下に置く' in t and 'エネルギー' in t:
        return {'type': 'place_energy_under_member', 'text': t}

    # Move cards cost
    if c.source or c.destination:
        result = {'type': 'move_cards', 'text': t,
                  'source': c.source or 'hand', 'destination': c.destination or 'discard',
                  'count': c.count or 1}
        for k in ['card_type', 'self_cost', 'exclude_self', 'optional',
                   'cost_limit', 'cost_limit_operator', 'group_names', 'characters']:
            if p.get(k): result[k] = p[k] if k != 'optional' else True
        if p.get('state_change') and 'メンバー' in t:
            result['state_change'] = p['state_change']
        return result

    # Change state cost
    if p.get('state_change') and ('メンバー' in t or 'このメンバー' in t):
        return {'type': 'change_state', 'text': t, 'state_change': p['state_change'],
                'card_type': 'member_card', 'optional': True if p.get('optional') else None}

    # Choice cost
    if 'か' in t and '、' in t:
        parts = t.split('か、', 1)
        if len(parts) == 2:
            return {'text': t, 'type': 'choice_condition',
                    'options': [_build_cost_item(Clause(role='cost', text=parts[0].strip())),
                                _build_cost_item(Clause(role='cost', text=parts[1].strip()))]}

    return {'text': t}


def _build_effect(role_map, clauses):
    has_cause = 'cause_result' in role_map and 'primary' in role_map
    has_look = 'look' in role_map and 'select' in role_map
    has_opt = any(r in role_map for r in ['cond_affirmation', 'cond_negation'])
    has_dur = 'duration_effect' in role_map
    has_per = 'per_unit_action' in role_map
    has_choice = 'choice_option' in role_map
    has_condition = 'condition' in role_map
    has_seq = any(r in role_map for r in ['further', 'sentence', 'sequential'])

    # 1. Conditional-on-result
    if has_cause:
        primary = build_action(role_map['primary'][0])
        cc = role_map.get('cause_condition', [None])[0]
        cr = role_map.get('cause_result', [None])[0]
        result = {'action': 'conditional_on_result'}
        if primary: result['primary_effect'] = primary
        if cc:
            classify(cc)
            result['result_condition'] = {'text': cc.text}
        if cr:
            f = build_action(cr)
            if f: result['followup_action'] = f
        return result

    # 2. Look-and-select
    if has_look:
        lc = role_map['look'][0]
        classify(lc)
        look_act = _build_simple_action(lc) or {'action': 'look_at', 'source': 'deck_top', 'count': 1}
        sel = build_action(role_map['select'][0])
        return {'action': 'look_and_select', 'look_action': look_act, 'select_action': sel}

    # 3. Conditional-on-optional
    if has_opt:
        opt = build_action(role_map.get('optional_action', [None])[0]) if 'optional_action' in role_map else None
        neg = 'cond_negation' in role_map
        r = 'cond_negation' if neg else 'cond_affirmation'
        ca = build_action(role_map.get(r, [None])[0]) if r in role_map else None
        return {'action': 'conditional_on_optional', 'optional_action': opt,
                'conditional_action': ca, 'conditional_negation': neg}

    # 4. Duration (かぎり)
    if has_dur:
        cc = role_map.get('condition', role_map.get('duration_cond', [None]))[0]
        ec = role_map['duration_effect'][0]
        if cc: classify(cc)
        eff = build_action(ec)
        result = {'duration': 'as_long_as'}
        if cc: result['condition'] = {'text': cc.text}
        if eff: result.update(eff)
        return result

    # 5. Per-unit
    if has_per:
        uc = role_map.get('per_unit_ref', [None])[0]
        ac = role_map['per_unit_action'][0]
        pu = _per_unit_params(uc)
        act = build_action(ac)
        if isinstance(act, dict):
            act['per_unit'] = True
            act['per_unit_count'] = pu.get('per_unit_count', 1)
            if pu.get('per_unit_type'): act['per_unit_type'] = pu['per_unit_type']
        return act

    # 6. Choice
    if has_choice:
        opts = [build_action(c) for c in role_map.get('choice_option', [])]
        return {'action': 'choice', 'options': [o for o in opts if o]}

    # 7. Conditional (場合、/とき、)
    if has_condition:
        cc = role_map['condition'][0]
        classify(cc)
        acs = [c for c in clauses if c.role not in ('condition', 'cost', 'duration_cond')]
        if acs:
            result = {'condition': {'text': cc.text}}
            acts = [build_action(c) for c in acs]
            acts = [a for a in acts if a]
            if len(acts) == 1:
                if acts[0].get('action') == 'sequential':
                    result['action'] = 'sequential'
                    result['actions'] = acts[0].get('actions', [])
                else:
                    result.update(acts[0])
            else:
                result['action'] = 'sequential'
                result['actions'] = acts
            return result

    # 8. Sequential (sentence / further / te-form chain)
    if has_seq:
        cls_list = role_map.get('further', []) or role_map.get('sentence', []) or role_map.get('sequential', [])
        acts = [build_action(c) for c in cls_list]
        acts = [a for a in acts if a]
        if len(acts) == 1:
            return acts[0]
        if acts:
            return {'action': 'sequential', 'actions': acts}

    # 9. Simple action
    for c in clauses:
        if c.role == 'action':
            return build_action(c)

    return None


def _per_unit_params(unit_clause):
    params = {'per_unit_count': 1}
    if not unit_clause: return params
    t = unit_clause.text
    import re
    m = re.search(r'(\d+)(人|枚|つ)', t)
    if m:
        params['per_unit_count'] = int(m.group(1))
        u = m.group(2)
        if u == '人': params['per_unit_type'] = 'member'
        elif u == '枚':
            params['per_unit_type'] = 'live_card_zone' if 'ライブ中のカード' in t else 'card'
    elif 'メンバー' in t: params['per_unit_type'] = 'member'
    elif 'カード' in t: params['per_unit_type'] = 'card'
    if 'ライブ中のカード' in t: params['per_unit_type'] = 'live_card_zone'
    return params


def build_action(clause):
    if isinstance(clause, str):
        clause = Clause(role='action', text=clause)
    if not clause.verb:
        classify(clause)
    return _build_simple_action(clause)


def _build_simple_action(clause):
    p = clause.params
    v = clause.verb
    t = clause.text
    c = clause

    if v == 'draw':
        return _compact('draw_card', t, count=c.count or 1)

    if v == 'discard':
        return _compact('move_cards', t, source=c.source or 'hand', destination='discard',
                        count=c.count or 1, card_type=c.card_type, optional=c.optional or None)

    if v == 'recover':
        return _compact('move_cards', t, source=c.source or 'discard', destination='hand',
                        count=c.count or 1, target=c.target, card_type=c.card_type,
                        group_names=p.get('group_names'), cost_limit=p.get('cost_limit'),
                        cost_limit_operator=p.get('cost_limit_operator'),
                        characters=p.get('characters'))

    if v == 'gain_blade':
        cnt = p.get('blade_icons') or c.count or 1
        return _compact('gain_resource', t, resource='blade', count=cnt, target=c.target)

    if v == 'gain_heart':
        return _compact('gain_resource', t, resource='heart', count=c.count or 1,
                        target=c.target, heart_color=p.get('heart_colors', [None])[0] or None)

    if v == 'gain':
        res = 'blade' if 'ブレード' in t else 'heart' if 'ハート' in t else 'generic'
        return _compact('gain_resource', t, resource=res, count=c.count or 1, target=c.target)

    if v == 'modify_score':
        return _compact('modify_score', t, operation=p.get('operation', 'add'), value=c.count or 1)

    if v == 'modify_hearts':
        return _compact('modify_required_hearts', t, operation=p.get('operation', 'decrease'),
                        heart_color=p.get('heart_color', 'heart00'), count=c.count or 1)

    if v == 'change_wait':
        r = _compact('change_state', t, state_change='wait', card_type='member_card', count=1, target=c.target)
        if c.optional: r['optional'] = True
        return r

    if v == 'change_active':
        return _compact('change_state', t, state_change='active', card_type='member_card',
                        count=c.count or None, target=c.target)

    if v == 'select':
        return _compact('select', t, source=c.source or 'stage', count=c.count or 1,
                        card_type=c.card_type or 'member_card')

    if v == 'look':
        return _compact('look_at', t, source='deck_top', count=c.count or 1)

    if v == 'reveal':
        return _compact('reveal', t, source=c.source or 'hand', count=c.count or 1)

    if v == 'appear':
        return _compact('appear', t, source=c.source or 'hand', destination=c.destination or 'stage')

    if v == 'pos_change' or v == 'swap':
        return _compact('position_change', t, target=c.target)

    if v == 'formation_change':
        return _compact('formation_change', t)

    if v == 'restrict':
        rt = 'cannot_live' if 'ライブできない' in t else (
            'cannot_activate' if ('アクティブにならない' in t or 'アクティブにしない' in t) else (
            'cannot_place' if ('置くことができない' in t or '置けない' in t) else (
            'cannot_appear' if '登場できない' in t else (
            'cannot_move' if '移動できない' in t else (
            'cannot_baton_touch' if 'バトンタッチ' in t else 'generic')))))
        return _compact('restriction', t, restriction_type=rt)

    if v == 'mod_cost':
        return _compact('modify_cost', t, operation=p.get('operation', 'subtract'),
                        value=c.count, group_names=p.get('group_names'))

    if v == 'gain_ability':
        return {'action': 'gain_ability', 'text': t}

    if v == 'activate_ability':
        return _compact('activate_ability', t, count=c.count or 1)

    if v == 'invalidate':
        return _compact('invalidate_ability', t)

    if v == 'baton':
        return _compact('play_baton_touch', t, count=c.count or 1)

    if v == 're_yell':
        return _compact('re_yell', t, lose_blade_hearts=True)

    if v == 'set_blade_count':
        return _compact('set_blade_count', t, count=c.count)

    if v == 'set_blade_type':
        return _compact('set_blade_type', t, blade_type=p.get('blade_type'))

    if v == 'set_identity':
        return _compact('set_card_identity', t)

    if v == 'do_nothing':
        return {'action': 'do_nothing', 'text': t}

    if v == 'shuffle':
        return _compact('shuffle', t, target='deck')

    if v == 'draw_until':
        return _compact('draw_until_count', t, target_count=c.count or 1)

    if v == 'move' or v == 'place':
        return _compact('move_cards', t, source=c.source, destination=c.destination,
                        count=c.count, card_type=c.card_type,
                        placement_order=p.get('placement_order'),
                        any_number=p.get('any_number') or None)

    # Generic fallback for source+dest present
    if c.source or c.destination:
        return _compact('move_cards', t, source=c.source, destination=c.destination,
                        count=c.count, card_type=c.card_type,
                        group_names=p.get('group_names'), characters=p.get('characters'))

    return None


def _compact(action, text, **kwargs):
    import re as _re
    clean = _re.sub(r'\{\{[^}]+\}\}', '', text).strip().rstrip('。') if text else ''
    d = {'action': action}
    if clean: d['text'] = clean
    for k, v in kwargs.items():
        if v is not None and v is not False and v != 0:
            d[k] = v
    return d

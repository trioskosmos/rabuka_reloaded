"""Template-based ability parser — each template is an invertible pattern with slots.
The output is {template_id, slots} which fully represents the ability:
given the template and slots, you can reconstruct the original text.

Matching is regex-based with named capture groups for each slot.
"""

from __future__ import annotations
import re
from typing import Any, Dict, List, Optional, Pattern

# ------------------------------------------------------------------
# A single atomic template
# ------------------------------------------------------------------

class AtomicTemplate:
    """A sentence pattern with named slots. Invertible: template + slots → text."""

    def __init__(self, tid: str, action: str, regex: str,
                 defaults: Optional[Dict] = None,
                 aliases: Optional[Dict] = None):
        self.tid = tid
        self.action = action
        self.pattern: Pattern = re.compile(regex)
        self.defaults = defaults or {}
        self.aliases = aliases or {}  # field_name → regex_group mapping

    def match(self, text: str) -> Optional[Dict]:
        m = self.pattern.search(text)
        if not m:
            return None
        slots = {}
        for key, val in m.groupdict().items():
            if val is not None:
                # Check aliases
                orig_key = self.aliases.get(key, key)
                slots[orig_key] = val
        # Apply defaults
        for k, v in self.defaults.items():
            slots.setdefault(k, v)
        # Convert numerics (only if still string)
        for fld in ('count', 'cost_limit', 'value', 'energy', 'target_count'):
            if fld in slots and isinstance(slots[fld], str) and slots[fld].isdigit():
                slots[fld] = int(slots[fld])
        return slots

    def __repr__(self):
        return f"AtomicTemplate({self.tid}, {self.action})"


# ------------------------------------------------------------------
# All atomic templates — each maps to ~1-2 English-style parameters
# ------------------------------------------------------------------

ATOMICS: List[AtomicTemplate] = [
    # === draw_card === (1 template)
    AtomicTemplate('draw_N', 'draw_card',
        r'カードを(?P<count>\d+)枚?引(?:く|き)',
        defaults={'source': 'deck', 'destination': 'hand'}),

    # === move_cards: 加える (discard→hand) ===
    AtomicTemplate('add_N_from_zone',
        'move_cards',
        r'(?P<target>自分|相手)の(?P<source>控え室|手札|デッキ|エネルギー置き場)から'
        r'(?:コスト(?P<cost_limit>\d+)以下)?'
        r'(?:の)?'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>ライブカード|メンバーカード|エネルギーカード|カード)?'
        r'を(?P<count>\d+)枚(?P<max>まで)?'
        r'(?P<destination>手札)に加える',
        defaults={'target': 'self'},
        aliases={'max': 'max'}),

    # === move_cards: 置く (place to zone) ===
    AtomicTemplate('place_to_zone',
        'move_cards',
        r'(?P<target>自分|相手)?の?(?P<source>控え室|エネルギー置き場|エネルギーデッキ|デッキ|手札)から'
        r'(?:コスト(?P<cost_limit>\d+)以下の)?'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>ライブカード|メンバーカード|エネルギーカード|カード)?'
        r'を(?P<count>\d+)枚(?P<max>まで)?'
        r'(?P<state>ウェイト状態|アクティブ状態)?で?'
        r'(?P<destination>デッキの一番上|デッキの一番下|デッキの上|デッキの下|エネルギー置き場|エネルギーゾーン|成功ライブカード置き場|ライブカード置き場|手札|控え室)に置く',
        defaults={'target': 'self'}),

    # === move_cards: 登場させる (deploy to stage) ===
    AtomicTemplate('deploy_to_stage',
        'move_cards',
        r'(?P<target>自分|相手)?の?(?P<source>控え室)から'
        r'(?:コスト(?P<cost_limit>\d+)以下の)?'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>メンバーカード|ライブカード)を(?P<count>\d+)枚'
        r'(?:メンバーのいない)?(?P<destination>ステージ|エリア|\w*エリア?)に登場させる',
        defaults={'destination': 'stage'}),

    # === move_cards: 送る (discard) ===
    AtomicTemplate('send_to_discard',
        'move_cards',
        r'(?P<target>自分|相手)?の?(?P<card_type>メンバー|エネルギー)?カード?'
        r'を(?P<count>\d+)(?:人|枚)'
        r'(?P<destination>控え室)に送る',
        defaults={'destination': 'discard'}),

    # === move_cards: discarding from hand (cost) ===
    AtomicTemplate('discard_from_hand',
        'move_cards',
        r'手札を(?P<count>\d+)枚(?:まで)?控え室に置く',
        defaults={'source': 'hand', 'destination': 'discard'}),

    # === move_cards: 戻す (return to deck) ===
    AtomicTemplate('return_to_deck',
        'move_cards',
        r'(?P<target>自分|相手)?の?(?P<source>控え室|手札)から'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>ライブカード|メンバーカード|エネルギーカード|カード)?'
        r'を(?P<count>\d+)枚(?P<destination>デッキの上|デッキの下|デッキ)に戻す'),

    # === move_cards: 見る (look at deck) ===
    AtomicTemplate('look_at_deck',
        'look_at',
        r'(?P<target>自分|相手)?の?(?P<source>デッキの上|デッキの一番上)から'
        r'(?:カードを)(?P<count>\d+)枚見る',
        defaults={'source': 'deck_top'}),

    # === gain_resource: 得る ===
    AtomicTemplate('gain_resource_blade',
        'gain_resource',
        r'\{\{icon_blade\.png\|ブレード\}\}(?:\{\{icon_blade\.png\|ブレード\}\})+',
        defaults={'resource': 'blade'},
        aliases={'__icon_count': 'count'}),

    AtomicTemplate('gain_resource_heart',
        'gain_resource',
        r'\{\{heart_\d+\.png\|heart\d+\}\}.*?ハート.*?を得る',
        defaults={'resource': 'heart'}),

    AtomicTemplate('gain_resource_raw',
        'gain_resource',
        r'(?P<resource>ブレード|ハート)を得る'),

    # === change_state: アクティブにする ===
    AtomicTemplate('activate',
        'change_state',
        r'(?P<target>自分|相手)?の?(?P<card_type>\w+)?を(?P<count>\d+)枚(?P<max>まで)?アクティブにする',
        defaults={'state_change': 'active'}),

    AtomicTemplate('activate_energy',
        'change_state',
        r'エネルギーを(?P<count>\d+)枚アクティブにする',
        defaults={'state_change': 'active', 'card_type': 'energy'}),

    # === change_state: ウェイトにする ===
    AtomicTemplate('wait_member',
        'change_state',
        r'(?P<target>自分|相手)?の?(?P<location>\w+)?を?(?:コスト(?P<cost_limit>\d+)以下)?'
        r'の?(?:『(?P<group>[^』]+)』の)?(?P<card_type>メンバー)?'
        r'を(?P<count>\d+)人(?P<max>まで)?ウェイトにする',
        defaults={'state_change': 'wait'}),

    # === modify_score ===
    AtomicTemplate('modify_score_plus',
        'modify_score',
        r'(?P<target>このカード)?の?(?:ライブの合計)?スコアを(?:\+|\プラス)?(?P<value>\d+)する',
        defaults={'operation': 'add'}),

    # === reveal ===
    AtomicTemplate('reveal_N',
        'reveal',
        r'(?:『(?P<group>[^』]+)』の)?(?P<card_type>メンバーカード|ライブカード)?'
        r'を(?P<count>\d+)枚(?P<max>まで)?公開する'),

    # === select ===
    AtomicTemplate('select_N',
        'select',
        r'(?P<source>控え室|手札|その中)?から?(?:『(?P<group>[^』]+)』の)?(?P<card_type>\w+)?'
        r'を(?P<count>\d+)(?:枚|人)選ぶ'),

    # === gain_ability ===
    AtomicTemplate('gain_ability',
        'gain_ability',
        r'「(.+)」を得る'),

    # === modify_cost ===
    AtomicTemplate('modify_cost_decrease',
        'modify_cost',
        r'(?:への)?コスト(?:は|が)(?P<value>\d+)(?P<operation>減る|減らす|増える|増やす)'),

    AtomicTemplate('modify_cost_deploy',
        'modify_cost',
        r'(?P<card_type>メンバーカード)?'
        r'を(?:自分の手札から)?登場させるためのコスト(?P<value>\d+)減る',
        defaults={'operation': 'decrease', 'source': 'hand'}),

    # === restrict ===
    AtomicTemplate('cannot_live',
        'restriction',
        r'ライブできない',
        defaults={'restriction_type': 'cannot_live'}),

    AtomicTemplate('cannot_activate',
        'restriction',
        r'アクティブにしない',
        defaults={'restriction_type': 'cannot_activate'}),

    AtomicTemplate('cannot_place',
        'restriction',
        r'置くことができない|置けない',
        defaults={'restriction_type': 'cannot_place'}),

    AtomicTemplate('cannot_appear',
        'restriction',
        r'登場できない',
        defaults={'restriction_type': 'cannot_appear'}),

    AtomicTemplate('cannot_move',
        'restriction',
        r'移動できない',
        defaults={'restriction_type': 'cannot_move'}),

    AtomicTemplate('cannot_baton_touch',
        'restriction',
        r'バトンタッチで控え室に置かれない',
        defaults={'restriction_type': 'cannot_baton_touch'}),

    # === gain_resource bare ===
    AtomicTemplate('gain_raw',
        'gain_resource',
        r'を得る',
        defaults={'resource': 'generic', 'count': 1}),

    # === move_cards: remaining to discard ===
    AtomicTemplate('remaining_to_discard',
        'move_cards',
        r'残りを(?P<destination>控え室)に置く',
        defaults={'source': 'looked_at_remaining', 'count': 1}),

    # === move_cards: energy deck to energy zone ===
    AtomicTemplate('energy_from_deck_to_zone',
        'move_cards',
        r'(?P<target>自分)?の?(?P<source>エネルギーデッキ)から、'
        r'(?P<card_type>エネルギーカード)を(?P<count>\d+)枚'
        r'(?P<state>ウェイト状態)で置く',
        defaults={'destination': 'energy_zone'}),

    # === move_cards: deck top to discard ===
    AtomicTemplate('deck_top_to_discard',
        'move_cards',
        r'(?P<target>自分|相手)?の?(?P<source>デッキの上|デッキの一番上)から'
        r'(?:カードを)?(?P<count>\d+)枚'
        r'(?P<destination>控え室)に置く'),

    # === move_cards: any_number placement ===
    AtomicTemplate('any_number_placement',
        'move_cards',
        r'好きな枚数を好きな順番で(?P<destination>デッキの上|デッキの一番上|デッキの下|デッキの一番下)に置く',
        defaults={'source': 'looked_at', 'any_number': True, 'placement_order': 'any_order'}),

    # === move_cards: short form (Nを手札に加える) ===
    AtomicTemplate('add_short',
        'move_cards',
        r'(?P<count>\d+)枚を?(?P<destination>手札)に加える'),

    # === gain_resource: verb form (得る) ===
    AtomicTemplate('gain_verb',
        'gain_resource',
        r'(?P<resource>ブレード|ハート|\w+)を?得る'),

    # === select from group ===
    AtomicTemplate('select_from',
        'select',
        r'(?P<source>その中)?から?(?:コスト(?P<cost_limit>\d+)以下)?'
        r'(?:『(?P<group>[^』]+)』の)?(?P<card_type>\w+)?'
        r'を(?P<count>\d+)(?:枚|人|つ)(?:まで)?選ぶ'),

    # === change_state: wait with original value ===
    AtomicTemplate('wait_with_original',
        'change_state',
        r'(?P<target>自分|相手)?の?(?P<location>ステージ)?にいる?'
        r'元々持つ(?P<resource>ブレード)?の?数が(?P<count>\d+)つ以下?の'
        r'(?P<card_type>メンバー)?(?P<count2>\d+)人をウェイトにする',
        defaults={'state_change': 'wait'}),

    # === modify_required_hearts ===
    AtomicTemplate('modify_hearts_decrease',
        'modify_required_hearts',
        r'(?:このカードを成功させるための)?必要ハートを(?P<operation>減らす|増やす)',
        defaults={'target': 'self'}),

    # === position_change ===
    AtomicTemplate('position_change_self',
        'position_change',
        r'このメンバーをポジションチェンジしてもよい',
        defaults={'target': 'self'}),

    AtomicTemplate('position_change_member',
        'position_change',
        r'(?:メンバー|(?P<count>\d+)人)をポジションチェンジさせる'),

    # === modify_score: generic ===
    AtomicTemplate('modify_score_generic',
        'modify_score',
        r'(?:このカード)?の?(?:ライブの)?(?:合計)?スコアを(?:\+)?(?P<value>\d+)(?:する|プラス)',
        defaults={'operation': 'add'}),

    # === gain_resource: heart selection ===
    AtomicTemplate('heart_selection',
        'gain_resource',
        r'好きなハートの色を(?P<count>\d+)つ指定する',
        defaults={'resource': 'heart', 'heart_selection': True}),

    # === gain_resource: per_unit (heart per card) ===
    AtomicTemplate('gain_per_unit',
        'gain_resource',
        r'(?:(?:これにより|選んだ)?(?P<source>\w+)?'
        r'(?:カード|枚)?(?P<per_unit_count>\d+)枚につき、)?'
        r'(?:選んだ)?(?P<resource>ブレード|ハート)を(?P<count>\d+)つ?得る',
        defaults={'per_unit': True}),

    # === move_cards: optional add to hand ===
    AtomicTemplate('add_to_hand_optional',
        'move_cards',
        r'手札に加えてもよい',
        defaults={'destination': 'hand', 'optional': True}),

    # === move_cards: short num+dest ===
    AtomicTemplate('num_to_dest',
        'move_cards',
        r'(?P<count>\d+)枚を(?P<destination>手札|控え室)に加える'),

    # === move_cards: any_order placement (short) ===
    AtomicTemplate('any_order_dest',
        'move_cards',
        r'好きな順番で(?P<destination>デッキの上|デッキの下|デッキの一番上|デッキの一番下)に置く',
        defaults={'placement_order': 'any_order', 'source': 'looked_at'}),

    # === change_state: wait member (with location) ===
    AtomicTemplate('wait_member_location',
        'change_state',
        r'(?P<target>自分|相手)?の?(?P<location>ステージ)にいる'
        r'(?:元々持つ(?P<resource>ブレード)?の?数が(?P<blade_limit>\d+)つ以下)?'
        r'(?:コスト(?P<cost_limit>\d+)以下)?'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>メンバー)?'
        r'を(?P<count>\d+)人(?P<max>まで)?ウェイトにする',
        defaults={'state_change': 'wait'}),

    # === change_state: activate members ===
    AtomicTemplate('activate_members',
        'change_state',
        r'(?P<target>自分|相手)?の?(?:ステージにいる)?'
        r'(?:このメンバー以外の)?'
        r'(?:ウェイト状態の)?'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>メンバー)?'
        r'を(?P<count>\d+)人(?P<max>まで)?アクティブにする',
        defaults={'state_change': 'active'}),

    # === select: from options ===
    AtomicTemplate('select_N_from',
        'select',
        r'の中から(?P<count>\d+)(?:つ|枚|人)を選ぶ'),

    # === gain_resource: original hearts become ===
    AtomicTemplate('original_hearts_become',
        'gain_resource',
        r'このメンバーが元々持つハートは選んだハートになる',
        defaults={'original_value': True, 'heart_selection': True}),

    # === modify_score: card score +N ===
    AtomicTemplate('card_score_plus',
        'modify_score',
        r'このカードのスコアを\+(?P<value>\d+)する',
        defaults={'operation': 'add', 'self_target': True}),

    # === activate_ability ===
    AtomicTemplate('activate_ability_new',
        'activate_ability',
        r'(?:これにより)?(?:\w+)?\w*(?:カード)?の?能力(?:\w*)を(?P<count>\d+)つ発動させる'),

    # === restriction: modify required hearts global ===
    AtomicTemplate('modify_hearts_global',
        'restriction',
        r'(?P<target>自分|相手)?の?(?P<location>ライブカード置き場)?に?ある?'
        r'(?:すべての)?(?P<card_type>ライブカード)?'
        r'は、成功させるための必要ハートが(?P<heart_color>\w+)'
        r'(?P<operation>多くなる|少なくなる)',
        defaults={'restriction_type': 'modify_required_hearts_global'}),

    # === modify_yell_count ===
    AtomicTemplate('modify_yell_count_decrease',
        'modify_yell_count',
        r'エールによって公開される自分(?:のステージ)?のカードの枚数が(?P<count>\d+)枚(?P<operation>減る|増える)'),

    # === draw_until_count ===
    AtomicTemplate('draw_until_count',
        'draw_until_count',
        r'手札が(?P<target_count>\d+)枚になるまでカードを引く',
        defaults={'source': 'deck', 'destination': 'hand'}),

    # === place_energy_under_member ===
    AtomicTemplate('place_energy_under',
        'place_energy_under_member',
        r'(?P<target>自分)?の?(?P<source>エネルギー置き場)?にある?'
        r'(?P<card_type>エネルギー)を(?P<energy_count>\d+)枚'
        r'(?:まで)?このメンバーの下に置く',
        defaults={'target_member': 'this_member'}),

    # === position_change: to specific area ===
    AtomicTemplate('position_change_area',
        'position_change',
        r'(?P<target>このメンバー)?を(?P<exclude_position>センターエリア)?外?に'
        r'ポジションチェンジする'),

    # ── add_short variant for looked_at source ──
    AtomicTemplate('add_short_looked_at',
        'move_cards',
        r'(?P<count>\d+)枚(?:を)?(?P<destination>手札)に加える',
        defaults={'source': 'looked_at'}),

    # ── remaining to discard ──
    AtomicTemplate('remaining_to_discard',
        'move_cards',
        r'残りを(?P<destination>控え室)に置く',
        defaults={'source': 'looked_at_remaining', 'count': 1}),

    # ── generic gain ──
    AtomicTemplate('gain_any',
        'gain_resource',
        r'(?P<resource>.+)を得る'),

    # ── generic draw ──
    AtomicTemplate('draw_generic',
        'draw_card',
        r'カードを(?P<count>\d+)枚?(?:引く|引き)',
        defaults={'source': 'deck', 'destination': 'hand'}),

    # ── generic change_state ──
    AtomicTemplate('change_wait_member',
        'change_state',
        r'(?P<target>自分|相手)?(?:の)?(?:ステージにいる)?'
        r'(?:コスト(?P<cost_limit>\d+)以下)?'
        r'(?:の)?(?:元々持つ(?P<resource>\w+)?の?数が(?P<blade_limit>\d+)つ以下)?'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>\w+)?を(?P<count>\d+)(?:人|枚)(?:まで)?(?P<max>)'
        r'(?:ウェイトにする|アクティブにする)'),

    # ── generic modify_score ──
    AtomicTemplate('mod_score_self',
        'modify_score',
        r'(?:このカード)?(?:の)?(?:ライブの合計)?スコアを\+?(?P<value>\d+)(?:する|プラス)',
        defaults={'operation': 'add'}),

    # ── deploy from discard (self) ──
    AtomicTemplate('deploy_self',
        'move_cards',
        r'このカードを(?P<source>控え室)から(?P<destination>ステージ)に登場させる',
        defaults={'self_target': True}),

    # ── optional add to hand ──
    AtomicTemplate('optional_add_hand',
        'move_cards',
        r'(?P<destination>手札)に加え(?:て|ても)よい',
        defaults={'optional': True}),

    # ── optional discard from hand ──
    AtomicTemplate('optional_discard_hand',
        'move_cards',
        r'手札を(?P<count>\d+)枚(?P<destination>控え室)に置い(?:て|ても)よい',
        defaults={'source': 'hand', 'optional': True}),

    # ── select from inside ──
    AtomicTemplate('select_from_inside',
        'select',
        r'(?:の)?中から(?P<count>\d+)(?:つ|枚|人)を選ぶ'),

    # ── select from options ──
    AtomicTemplate('select_options',
        'select',
        r'以下から(?P<count>\d+)(?:つ|枚|人)を選ぶ'),

    # ── activate ability ──
    AtomicTemplate('activate_abil',
        'activate_ability',
        r'\w*能力\w*を(?P<count>\d+)つ発動させる'),

    # ── invalidate ability ──
    AtomicTemplate('invalidate_abil',
        'invalidate_ability',
        r'無効に(?:し|して)もよい'),

    # ── modify required hearts ──
    AtomicTemplate('modify_hearts',
        'modify_required_hearts',
        r'(?:このカードを)?(?:成功させるための)?必要ハートを(?P<operation>減らす|増やす)'),

    # ── modify required hearts global ──
    AtomicTemplate('modify_hearts_global',
        'restriction',
        r'必要ハートが(?P<operation>多くなる|少なくなる)',
        defaults={'restriction_type': 'modify_required_hearts_global'}),

    # ── position change exclude center ──
    AtomicTemplate('pos_change_exclude',
        'position_change',
        r'このメンバーを(?P<exclude>センターエリア以外)にポジションチェンジする'),

    # ── position change optional ──
    AtomicTemplate('pos_change_opt',
        'position_change',
        r'(?:この)?メンバー(?P<count>\d+)人?をポジションチェンジ(?:し|させ)(?:て)?もよい'),

    # ── gain ability ──
    AtomicTemplate('gain_abil',
        'gain_ability',
        r'「(.+)」を得る'),

    # ── short add (te-form) ──
    AtomicTemplate('short_add_te',
        'move_cards',
        r'(?P<count>\d+)枚(?:を)?(?P<destination>手札)に加え',
        defaults={'source': 'looked_at'}),

    # ── any order placement ──
    AtomicTemplate('any_order_placement',
        'move_cards',
        r'好きな枚数を好きな順番で(?P<destination>デッキの上|デッキの一番上|デッキの下|デッキの一番下)に置く',
        defaults={'any_number': True, 'placement_order': 'any_order', 'source': 'looked_at'}),

    # ── select from those ──
    AtomicTemplate('select_those',
        'select',
        r'その中から(?P<count>\d+)(?:枚|つ|人)(?:を)?(?:選ぶ|手札に加える)',
        defaults={'source': 'looked_at'}),

    # ── change_state: wait target with location/cost (simple version) ──
    AtomicTemplate('wait_simple',
        'change_state',
        r'(?P<target>相手|自分)?(?:の)?(?:ステージにいる)?'
        r'(?:元々持つ)?(?:ブレードの数が(?P<blade_limit>\d+)つ以下)?'
        r'(?:コスト(?P<cost_limit>\d+)以下)?(?:の)?'
        r'(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>メンバー)?'
        r'を(?P<count>\d+)(?:人|枚)(?:まで)?'
        r'(?P<change>ウェイト|アクティブ)にする',
        defaults={'state_change': 'wait'}),

    # ── generic change_state fallback ──
    AtomicTemplate('change_state_fallback',
        'change_state',
        r'(?P<state_change>ウェイト|アクティブ)にする'),

    # ── discard until count ──
    AtomicTemplate('discard_until',
        'discard_until_count',
        r'手札の枚数が(?P<target_count>\d+)枚になるまで手札を(?P<destination>控え室)に置く',
        defaults={'source': 'hand'}),

    # ── both-players discard_until ──
    AtomicTemplate('both_discard_until',
        'discard_until_count',
        r'自分と相手はそれぞれ自身の手札の枚数が(?P<target_count>\d+)枚になるまで手札を(?P<destination>控え室)に置(?:く|き)',
        defaults={'source': 'hand', 'target': 'both', 'multiple_targets': True}),

    # ── exclude self move from stage to discard ──
    AtomicTemplate('exclude_self_move_discard',
        'move_cards',
        r'(?:このメンバー以外の)?(?:『(?P<group>[^』]+)』の)?'
        r'(?P<card_type>メンバー)?(?P<count>\d+)人を'
        r'(?:自分の)?(?P<source>ステージ)から(?P<destination>控え室)に置く',
        defaults={'exclude_self': True}),

    # ── select from middle (inside) ──
    AtomicTemplate('select_inside',
        'select',
        r'(?:の)?中から(?P<count>\d+)(?:つ|枚)を?選ぶ',
        defaults={'source': 'looked_at'}),

    # ── select from among ──
    AtomicTemplate('select_among',
        'select',
        r'その中から(?P<count>\d+)(?:枚|つ)を?(?:手札に加える|選ぶ)',
        defaults={'source': 'looked_at'}),

    # ── modify_score generic ──
    AtomicTemplate('score_plus_N',
        'modify_score',
        r'(?:このカード)?の?(?:ライブの)?(?:合計)?スコアを\+(?P<value>\d+)(?:する|に)',
        defaults={'operation': 'add'}),

    # ── modify_score with に (set) ──
    AtomicTemplate('score_set',
        'modify_score',
        r'(?:このカード)?の?スコアを(?P<value>\d+)になる',
        defaults={'operation': 'add'}),

    # ── place energy under member ──
    AtomicTemplate('place_energy_under_opt',
        'place_energy_under_member',
        r'(?P<target>自分)?の?(?P<source>エネルギー置き場|エネルギーゾーン)にある'
        r'(?P<card_type>エネルギー)\s*(?P<energy_count>\d+)枚?を'
        r'このメンバーの下に置(?:く|い)て(?:も)?よい',
        defaults={'target_member': 'this_member', 'optional': True}),

    # ── revealed remaining to discard ──
    AtomicTemplate('revealed_remaining_discard',
        'move_cards',
        r'(?:これにより)?公開された(?:(?P<group>\w+)の)?ほかの?すべての?(?P<card_type>カード)?を'
        r'(?P<destination>控え室)に置く',
        defaults={'source': 'revealed_remaining', 'count': 1}),

    # ── deploy from discard to empty area ──
    AtomicTemplate('deploy_empty_area',
        'move_cards',
        r'(?P<source>控え室|手札)から(?P<card_type>メンバーカード)?を(?P<count>\d+)枚?'
        r'(?:まで)?(?:、)?(?P<destination>メンバーのいないエリア|ステージ)に登場させる'),

    # ── re-yell ──
    AtomicTemplate('re_yell_simple',
        're_yell',
        r'(?:この)?エールで(?:得た)?ブレードハートを失い、もう(?:一度|1度)エールを行う',
        defaults={'lose_blade_hearts': True}),

    # ── send to discard with 送る ──
    AtomicTemplate('send_discard',
        'move_cards',
        r'(?P<target>相手|自分)?(?:の)?(?P<card_type>\w+)?(?:カード)?を'
        r'(?P<count>\d+)(?:人|枚)?(?P<destination>控え室)に送る'),

    # ── cost modification: per card ──
    AtomicTemplate('cost_per_card',
        'modify_cost',
        r'(?:この)?カード以外の自分の手札(?P<per_unit_count>\d+)枚につき、'
        r'(?P<value>\d+)少なくなる',
        defaults={'operation': 'decrease', 'per_unit': True, 'location': 'hand'}),

    # ── set required hearts ──
    AtomicTemplate('set_required_hearts',
        'set_required_hearts',
        r'コストは(?P<heart_colors>.+)になる'),

    # ── generic modify_score ──
    AtomicTemplate('generic_score_plus',
        'modify_score',
        r'(?:このカード)?(?P<target>.+)?の?スコアを(?:\+)(?P<value>\d+)(?:する|プラス|に)',
        defaults={'operation': 'add'}),

    # ── per_unit gain (linked to per_unit check above) ──
    AtomicTemplate('per_unit_blade',
        'gain_resource',
        r'\w+?(?P<per_unit_count>\d+)(?:枚|人)につき[\s、]*(?:選んだ)?(?P<resource>ブレード|ハート)(?:を)?(?P<count>\d+)つ?得る',
        defaults={'per_unit': True}),

    # ── per_unit heart gain ──
    AtomicTemplate('per_unit_heart',
        'gain_resource',
        r'(?:これにより|選んだ)?[\w]+?(?P<per_unit_count>\d+)(?:枚|人)につき[\s、]*(?:その|選んだ)?(?P<resource>ハート|ブレード)(?:を)?(?P<count>\d+)つ得る',
        defaults={'per_unit': True}),

    # ── score minus (remove) ──
    AtomicTemplate('score_minus',
        'modify_score',
        r'スコアを(?P<value>\d+)マイナス',
        defaults={'operation': 'remove'}),

    # ── set score ──
    AtomicTemplate('score_set_N',
        'set_score',
        r'スコア[はが](?P<value>\d+)になる'),

    # ── cannot place in zone ──
    AtomicTemplate('cannot_place_zone',
        'restriction',
        r'(?P<target>\w+)?(?:は)?(?P<location>\w+)?に(?P<card_type>\w+)?'
        r'を置くことができない',
        defaults={'restriction_type': 'cannot_place'}),

    # ── draw when condition ──
    AtomicTemplate('draw_when',
        'draw_card',
        r'(?:カードを)?(?P<count>\d+)枚?(?:まで)?引く',
        defaults={'source': 'deck', 'destination': 'hand'}),

    # ── modify_required_hearts per unit ──
    AtomicTemplate('modify_hearts_per_unit',
        'modify_required_hearts',
        r'(?P<per_unit_count>\d+)枚につき[\s、]*このカードを成功させるための必要ハート(?P<operation>減らす|増やす)',
        defaults={'per_unit': True}),

    # ── modify_cost on stage ──
    AtomicTemplate('modify_cost_stage',
        'modify_cost',
        r'ステージにいるこのメンバーのコスト[がを]\+(?P<value>\d+)増える',
        defaults={'operation': 'increase'}),

    # ── set_card_identity ──
    AtomicTemplate('set_card_identity_generic',
        'set_card_identity',
        r'すべての領域にあるこのカードは(?P<identities>.+)として扱う',
        defaults={'all_regions': True}),

    # ── reveal generic ──
    AtomicTemplate('reveal_generic',
        'reveal',
        r'(?P<card_type>\w+)?カード?を(?P<count>\d+)枚?まで?(?:公開し|公開する)'),

    # ── gain_resource (with count, from anything) ──
    AtomicTemplate('gain_resource_count',
        'gain_resource',
        r'(?:\w+)?(?:(?P<count>\d+)つ?)?(?P<resource>ブレード|ハート|\w+?)(?:を)?得る'),

    # ── generic move_cards fallback ──
    AtomicTemplate('move_cards_fallback',
        'move_cards',
        r'(?P<source>\w+)?(?:から)?(?:の)?(?P<card_type>\w+)?'
        r'を(?P<count>\d+)(?:人|枚)(?:まで)?'
        r'(?P<destination>\w+)(?:に)?(?:置く|加える|送る|戻す|登場させる)'),
]



# ------------------------------------------------------------------
# Matcher: find the best-matching atomic template
# ------------------------------------------------------------------

def match_atomic(text: str) -> Optional[Dict]:
    """Find the best atomic template match for text. Returns slots or None."""
    for template in ATOMICS:
        slots = template.match(text)
        if slots:
            slots['_template'] = template.tid
            slots['action'] = template.action
            return slots
    return None


def match_with_fallback(text: str) -> Optional[Dict]:
    """Template match with automatic pattern generation as fallback."""
    # First try existing templates
    result = match_atomic(text)
    if result:
        return result
    
    # Fallback: normalized regex with digit/name capture
    # Replace numbers with (\d+), group/character names with capture groups
    fallback_pattern = re.escape(text)
    fallback_pattern = re.sub(r'\\d\+', r'(\\d+)', fallback_pattern)
    
    # Try to match ANY template with the normalized pattern
    norm_text = re.sub(r'\d+', 'N', text)
    norm_text = re.sub(r'『[^』]+』', '『G』', norm_text)
    
    return {'action': 'custom', 'text': text, '_normalized': norm_text}


def parse_action(text: str) -> Dict[str, Any]:
    """Parse action text using template matching."""
    result = match_with_fallback(text)
    return result or {'action': 'custom', 'text': text}

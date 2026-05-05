"""Schema-driven parser — fields are extracted independently, 
action is inferred from field SETS, not from a priority cascade.

Every field here maps 1:1 to a field in the Rust AbilityEffect struct.
"""

from __future__ import annotations
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Pattern, Tuple
import re

# ------------------------------------------------------------------
# A single extractable field
# ------------------------------------------------------------------

@dataclass
class Field:
    """A field that can be extracted from ability text.
    Maps directly to a field in Rust's AbilityEffect struct."""
    name: str                     # matches the Rust struct field name
    rust_type: str                # e.g. "Option<String>", "Option<u32>", "Option<bool>"
    patterns: List[Tuple[str, Any]]  # [(regex_pattern, value), ...] first match wins
    transform: Optional[Callable] = None  # post-processing if needed
    depends_on: List[str] = None  # fields that must be present first

# ------------------------------------------------------------------
# All field definitions — one per Rust AbilityEffect field
# Each field is extracted independently, no priority
# ------------------------------------------------------------------

FIELDS: List[Field] = [
    # -- Source (locations) --
    Field('source', 'Option<String>', [
        (r'控え室から|控え室にある', 'discard'),
        (r'手札から|手札を(?=控え室)', 'hand'),
        (r'デッキから|山札から', 'deck'),
        (r'デッキの上から|デッキの一番上から', 'deck_top'),
        (r'ステージから', 'stage'),
        (r'エネルギー置き場から|エネルギーゾーンから', 'energy_zone'),
        (r'成功ライブカード置き場から', 'success_live_zone'),
        (r'ライブカード置き場から', 'live_card_zone'),
        (r'デッキの一番下から', 'deck_bottom'),
        (r'これにより公開された|これにより公開した|公開したカードを', 'revealed_cards'),
        (r'下に置かれている', 'under_member'),
        (r'このカードを手札に加えてもよい', 'revealed_card'),
    ]),

    # -- Destination --
    Field('destination', 'Option<String>', [
        (r'控え室に置く|控え室に置いて|控え室に送る|枚控え室に置く', 'discard'),
        (r'手札に加える|手札に加えて|手札に置く', 'hand'),
        (r'エネルギー置き場に置く|エネルギーゾーンに置く', 'energy_zone'),
        (r'デッキの一番上に置く|デッキの上に置く|山札の上に置く', 'deck_top'),
        (r'デッキの一番下に置く|デッキの下に置く|山札の下に置く', 'deck_bottom'),
        (r'ステージに登場させる|登場させる', 'stage'),
        (r'成功ライブカード置き場に置く', 'success_live_zone'),
        (r'ライブカード置き場に置く', 'live_card_zone'),
        (r'メンバーのいないエリア', 'empty_area'),
        (r'いたエリアに|置かれていたエリアに', 'same_area'),
        (r'このメンバーの下に置く', 'under_member'),
        (r'デッキに戻す|デッキに置く', 'deck'),
        (r'エネルギーデッキに置く', 'energy_deck'),
    ]),

    # -- Count --
    Field('count', 'Option<u32>', [
        (r'(\d+)枚', lambda m: int(m.group(1))),
        (r'(\d+)人', lambda m: int(m.group(1))),
        (r'(\d+)つ', lambda m: int(m.group(1))),
    ]),

    # -- Target count --
    Field('target_count', 'Option<u32>', [
        (r'(\d+)枚になるまで', lambda m: int(m.group(1))),
    ]),

    # -- Card type --
    Field('card_type', 'Option<String>', [
        (r'メンバーカード', 'member_card'),
        (r'ライブカード', 'live_card'),
        (r'エネルギーカード', 'energy_card'),
    ]),
    # Note: infer_card_type also checks "メンバー" and "エネルギー" broadly

    # -- Target --
    Field('target', 'Option<String>', [
        (r'(?=.*自分の)(?=.*相手の)自分と相手|自分と相手', 'both'),
        (r'自分か相手', 'either'),
        (r'相手の', 'opponent'),
        (r'自分の', 'self'),
    ]),

    # -- Duration --
    Field('duration', 'Option<String>', [
        (r'ライブ終了時まで', 'live_end'),
        (r'ライブ終了まで', 'live_end'),
        (r'このターンの間', 'this_turn'),
        (r'このライブの間', 'this_live'),
        (r'ターン終了時まで', 'turn_end'),
    ]),

    # -- Resource --
    Field('resource', 'Option<String>', [
        (r'{{icon_blade\.png\|ブレード}}', 'blade'),
        (r'{{heart_\d+\.png\|heart\d+}}', 'heart'),
        (r'{{icon_all\.png\|ハート}}', 'heart'),
        (r'ブレード', 'blade'),
        (r'ハート(?=を得る)', 'heart'),
    ], depends_on=['得る']),

    # -- State change --
    Field('state_change', 'Option<String>', [
        (r'ウェイトにする|ウェイト状態|ウェイトにし|ウェイトに', 'wait'),
        (r'アクティブにする|アクティブにし|アクティブに', 'active'),
    ]),

    # -- Operation (for modify_cost, modify_score, etc.) --
    Field('operation', 'Option<String>', [
        (r'減る|減らす', 'decrease'),
        (r'増える|増やす', 'increase'),
        (r'プラス|\+', 'add'),
        (r'マイナス|\-', 'remove'),
    ]),

    # -- Value --
    Field('value', 'Option<u32>', [
        (r'(\d+)(?=プラス|\+価|ポイント)', lambda m: int(m.group(1))),
    ]),

    # -- Position --
    Field('position', 'Option<String>', [
        (r'センターエリア|センター', 'center'),
        (r'左サイドエリア|左サイド', 'left_side'),
        (r'右サイドエリア|右サイド', 'right_side'),
    ]),

    # -- Activation position --
    Field('activation_position', 'Option<String>', [
        (r'この能力は.*センターエリア', 'center'),
        (r'この能力は.*左サイドエリア', 'left_side'),
        (r'この能力は.*右サイドエリア', 'right_side'),
        (r'この能力は.*左サイドか.*右サイド|この能力は.*右サイドか.*左サイド', 'left_side|right_side'),
    ]),

    # -- Boolean flags --
    Field('optional', 'Option<bool>', [
        (r'もよい|てもよい', True),
    ]),
    Field('exclude_self', 'Option<bool>', [
        (r'このメンバー以外|ほかのメンバー', True),
    ]),
    Field('all', 'Option<bool>', [
        (r'すべての|全ての|全部の|カードをすべて', True),
    ]),
    Field('multiple_targets', 'Option<bool>', [
        (r'それぞれ|ずつ', True),
    ]),
    Field('self_target', 'Option<bool>', [
        (r'このカード', True),
    ]),
    Field('max', 'Option<bool>', [
        (r'人まで|枚まで', True),
    ]),
    Field('per_unit', 'Option<bool>', [
        (r'につき|ごとに|たび', True),
    ]),
    Field('original_value', 'Option<bool>', [
        (r'元々持つ|元々', True),
    ]),
    Field('ability_negation', 'Option<bool>', [
        (r'能力を持たない|能力も持たない', True),
    ]),
    Field('shuffle', 'Option<bool>', [
        (r'シャッフル', True),
    ]),
    Field('conditional', 'Option<bool>', [
        (r'そうした場合', True),
    ]),
    Field('lose_blade_hearts', 'Option<bool>', [
        (r'ブレードハートを失い|ブレードハートを失う', True),
    ]),

    # -- Per-unit fields --
    Field('per_unit_count', 'Option<u32>', [
        (r'(\d+)(?=枚につき|人につき|つにつき)', lambda m: int(m.group(1))),
    ]),
    Field('per_unit_type', 'Option<String>', [
        (r'メンバー\d*(?=人|枚)', 'member'),
        (r'カード\d*(?=枚)', 'card'),
        (r'枚(?=につき)', 'card'),
    ]),

    # -- Cost limit --
    Field('cost_limit', 'Option<u32>', [
        (r'コスト(\d+)(?=以上|以下|未満|超)', lambda m: int(m.group(1))),
    ]),

    # -- Placement --
    Field('placement_order', 'Option<String>', [
        (r'好きな順番', 'any_order'),
    ]),

    # -- Misc --
    Field('restriction_type', 'Option<String>', [
        (r'ライブできない', 'cannot_live'),
        (r'アクティブにしない', 'cannot_activate'),
        (r'バトンタッチで控え室に置けない', 'cannot_baton_touch'),
        (r'置くことができない|置けない', 'cannot_place'),
        (r'登場できない', 'cannot_appear'),
        (r'移動できない', 'cannot_move'),
    ]),
    Field('blade_type', 'Option<String>', [
        (r'すべて\[([^\]]+)\]になる', lambda m: m.group(1)),
    ]),
    Field('energy_count', 'Option<u32>', [
        (r'エネルギー(\d+)枚', lambda m: int(m.group(1))),
        (r'エネルギー(\d+)つ', lambda m: int(m.group(1))),
    ]),
]


# ------------------------------------------------------------------
# Action signatures — keyed by field SETS, not priority
# ------------------------------------------------------------------

# Each action type is identified by the SET of required fields
# that must be present. The inference picks the best match
# (most required fields satisfied).
ACTION_FIELD_SIGNATURES: Dict[str, Dict] = {
    'move_cards': {
        'required': ['source', 'destination'],
        'optional': ['count', 'card_type', 'target', 'exclude_self', 'self_target',
                     'max', 'shuffle', 'placement_order', 'cost_limit', 'all'],
        'defaults': {},
    },
    'draw_card': {
        'required': [],
        'optional': ['count'],
        'defaults': {'source': 'deck', 'destination': 'hand'},
        'keywords': ['引く', '引き'],
    },
    'change_state': {
        'required': ['state_change'],
        'optional': ['count', 'card_type', 'optional', 'target'],
        'defaults': {},
    },
    'gain_resource': {
        'required': ['resource'],
        'optional': ['count', 'duration', 'per_unit', 'per_unit_count', 'per_unit_type'],
        'defaults': {},
    },
    'gain_ability': {
        'required': [],
        'optional': ['duration', 'quoted_text', 'count'],
        'keywords': ['能力.*を得る', '「.+」を得る'],
    },
    'modify_cost': {
        'required': ['operation'],
        'optional': ['value', 'per_unit', 'count'],
        'defaults': {},
    },
    'modify_score': {
        'required': ['operation'],
        'optional': ['value'],
        'defaults': {},
        'keywords': ['スコア'],
    },
    'pay_energy': {
        'required': [],
        'optional': ['optional', 'count'],
        'keywords': ['{{icon_energy.png|E}}'],
        'defaults': {},
    },
    'shuffle': {
        'required': ['shuffle'],
        'optional': ['target'],
        'defaults': {},
    },
    'position_change': {
        'required': [],
        'optional': ['position', 'target', 'optional'],
        'keywords': ['ポジションチェンジ', '入れ替える', 'フォーメーションチェンジ'],
    },
    'reveal': {
        'required': [],
        'optional': ['source', 'count'],
        'keywords': ['公開する', '公開し'],
    },
    'select': {
        'required': [],
        'optional': ['count'],
        'keywords': ['選ぶ', '選ん'],
    },
    'look_at': {
        'required': [],
        'optional': ['source', 'count', 'dynamic_count'],
        'keywords': ['見る', '見て'],
    },
    'appear': {
        'required': [],
        'optional': ['destination', 'card_type', 'state_change'],
        'keywords': ['登場させ'],
    },
    'invalidate_ability': {
        'required': [],
        'optional': ['optional'],
        'keywords': ['無効に'],
    },
    'activate_ability': {
        'required': [],
        'optional': ['count'],
        'keywords': ['発動させる', '起動でき'],
    },
    'modify_required_hearts': {
        'required': [],
        'optional': ['count', 'duration'],
        'keywords': ['必要ハート'],
    },
    'restriction': {
        'required': ['restriction_type'],
        'optional': ['target', 'duration', 'card_type'],
        'defaults': {},
    },
    'choice': {
        'required': [],
        'optional': ['options', 'choice_type'],
        'keywords': ['以下から1つを選ぶ'],
    },
    'set_blade_type': {
        'required': ['blade_type'],
        'optional': ['duration'],
        'defaults': {},
    },
    'set_blade_count': {
        'required': [],
        'optional': ['count'],
        'keywords': ['ブレードの数は'],
    },
    'set_card_identity': {
        'required': [],
        'optional': ['identities', 'all_regions'],
        'keywords': ['として扱う'],
    },
    'do_nothing': {
        'required': [],
        'optional': [],
        'keywords': ['何もしない'],
    },
    're_yell': {
        'required': ['lose_blade_hearts'],
        'optional': [],
        'defaults': {},
    },
}


# ------------------------------------------------------------------
# Public API
# ------------------------------------------------------------------

def extract_field(field: Field, text: str) -> Optional[Any]:
    """Extract a single field from text. Returns extracted value or None."""
    for pattern, value in field.patterns:
        m = re.search(pattern, text)
        if m:
            if callable(value):
                return value(m)
            if isinstance(value, str):
                return value
            if value is True or isinstance(value, int):
                return value
            return m.group(1) if m.lastindex else value
    return None


def extract_all(text: str) -> Dict[str, Any]:
    """Extract ALL fields from text. Each field is independent."""
    result = {}
    for field in FIELDS:
        value = extract_field(field, text)
        if value is not None:
            result[field.name] = value
    return result

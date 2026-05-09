"""Annotator - grammar-first clause decomposition + particle-aware extraction."""
import re
from typing import Any, Dict, List, Optional, Tuple
from dataclasses import dataclass, field


@dataclass
class Clause:
    role: str = ''
    verb: str = ''
    text: str = ''
    params: dict = field(default_factory=dict)
    source: Optional[str] = None
    destination: Optional[str] = None
    card_type: Optional[str] = None
    count: Optional[int] = None
    target: str = 'self'
    optional: bool = False
    duration: Optional[str] = None


# === Complete source/destination lists (matching parser.py) ===
SOURCE_PATTERNS = [
    ('相手の控え室', 'discard'), ('相手の控え室', 'discard'),
    ('控え室', 'discard'), ('手札', 'hand'),
    ('デッキの上から', 'deck_top'), ('デッキの一番上から', 'deck_top'),
    ('デッキの一番下から', 'deck_bottom'),
    ('デッキ', 'deck'), ('山札', 'deck'),
    ('ステージ', 'stage'),
    ('エネルギー置き場', 'energy_zone'),
    ('エネルギーデッキ', 'energy_deck'),
    ('ライブカード置き場', 'live_card_zone'),
    ('成功ライブカード置き場', 'success_live_zone'),
    ('エールにより公開された', 'revealed_cards'),
    ('下に置かれている', 'under_member'),
]

DEST_PATTERNS = [
    ('控え室に置く', 'discard'), ('控え室に送る', 'discard'),
    ('手札に加える', 'hand'), ('手札に加え', 'hand'), ('手札に戻す', 'hand'), ('手札に置く', 'hand'),
    ('デッキの一番上に', 'deck_top'), ('デッキの上に', 'deck_top'),
    ('デッキの一番下に', 'deck_bottom'), ('デッキの下に', 'deck_bottom'),
    ('デッキに戻す', 'deck'), ('デッキに置く', 'deck'),
    ('ステージに登場させる', 'stage'), ('ステージに置く', 'stage'),
    ('エネルギー置き場に', 'energy_zone'), ('エネルギーゾーンに', 'energy_zone'),
    ('ライブカード置き場に', 'live_card_zone'),
    ('成功ライブカード置き場に', 'success_live_zone'),
    ('メンバーのいないエリア', 'empty_area'),
    ('そのメンバーがいたエリア', 'same_area'),
    ('このメンバーの下に', 'under_member'),
]

CARD_TYPES = {
    'メンバーカード': 'member_card', 'メンバー': 'member_card',
    'ライブカード': 'live_card', 'エネルギーカード': 'energy_card',
}


def extract_source(text: str) -> Optional[str]:
    """Extract source location, preferring explicit から marker."""
    # First check for から with a specific noun before it
    m = re.search(r'(.+?)(?:から)', text)
    if m:
        before = m.group(1)
        for noun, code in SOURCE_PATTERNS:
            if noun in before:
                return code
    # Also check にある (in/at location)
    m = re.search(r'(.+?)(?:にある)', text)
    if m:
        before = m.group(1)
        for noun, code in SOURCE_PATTERNS:
            if noun in before:
                return code
    # Fallback: check for source nouns anywhere
    for noun, code in SOURCE_PATTERNS:
        if noun in text:
            # Avoid matching when noun is in destination context (に / へ)
            if 'に' in text and text.find(noun) < text.find('に'):
                return code
            if 'に' not in text:
                return code
    # Final fallback: check for '手札を' (hand as object) or '手札の' (hand as possession)
    if '手札を' in text or '手札の' in text:
        return 'hand'
    return None


def extract_destination(text: str) -> Optional[str]:
    """Extract destination location from text."""
    for phrase, code in DEST_PATTERNS:
        if phrase in text:
            return code
    return None


def extract_card_type(text: str) -> Optional[str]:
    for phrase, code in CARD_TYPES.items():
        if phrase in text:
            return code
    return None


def extract_count(text: str) -> Optional[int]:
    m = re.search(r'(\d+)(枚|人|つ|個)', text)
    if m: return int(m.group(1))
    return None


def extract_target(text: str) -> str:
    if '自分の' in text and '相手の' in text: return 'both'
    if '相手の' in text or '相手は' in text: return 'opponent'
    if '自分か相手' in text: return 'either'
    return 'self'


def extract_optional(text: str) -> bool:
    return 'もよい' in text or 'てもよい' in text


def extract_duration(text: str) -> Optional[str]:
    for ph, code in [('ライブ終了時まで', 'live_end'), ('ライブ終了まで', 'live_end'),
                    ('このターンの間', 'this_turn'), ('そのターンの間', 'turn_end'),
                    ('ターン終了時まで', 'turn_end'), ('このライブの間', 'this_live')]:
        if ph in text: return code
    return None


# === Verb Detection ===
def detect_verb(text: str) -> str:
    pairs = [
        ('カードを1枚引いてもよい', 'draw'),
        ('引く', 'draw'), ('引き', 'draw'),
        ('控え室に置く', 'discard'), ('控え室に置いて', 'discard'), ('控え室に置き', 'discard'),
        ('手札に加える', 'recover'), ('手札に加え', 'recover'), ('手札に戻す', 'recover'),
        ('ブレードを得る', 'gain_blade'), ('選んだブレード', 'gain_blade'),
        ('ハートを得る', 'gain_heart'), ('選んだハート', 'gain_heart'),
        ('得る', 'gain'),
        ('スコアを', 'modify_score'),
        ('必要ハート', 'modify_hearts'), ('ハートを増やす', 'modify_hearts'), ('ハートを減らす', 'modify_hearts'),
        ('ウェイトにする', 'change_wait'), ('ウェイト状態', 'change_wait'),
        ('アクティブにする', 'change_active'), ('アクティブにしてもよい', 'change_active'),
        ('登場させる', 'appear'), ('登場させ', 'appear'),
        ('公開する', 'reveal'), ('公開し', 'reveal'),
        ('見る', 'look'), ('見て', 'look'),
        ('選ぶ', 'select'), ('選ん', 'select'), ('選び', 'select'),
        ('ポジションチェンジ', 'pos_change'), ('入れ替える', 'swap'),
        ('フォーメーションチェンジ', 'formation_change'),
        ('できない', 'restrict'), ('置けない', 'restrict'),
        ('アクティブにならない', 'restrict'), ('アクティブにしない', 'restrict'),
        ('登場できない', 'restrict'), ('移動できない', 'restrict'),
        ('バトンタッチ', 'baton'),
        ('もう一度エール', 're_yell'), ('もう1度エール', 're_yell'),
        ('能力を発動させ', 'activate_ability'),
        ('能力を得る', 'gain_ability'), ('能力をすべて得る', 'gain_ability'),
        ('無効に', 'invalidate'),
        ('減る', 'mod_cost'), ('減らす', 'mod_cost'), ('増える', 'mod_cost'),
        ('増やす', 'mod_cost'), ('少なくなる', 'mod_cost'),
        ('コストを', 'mod_cost'),
        ('ブレードの数は', 'set_blade_count'),
        ('ブレードの色を', 'set_blade_type'), ('すべて[', 'set_blade_type'),
        ('として扱う', 'set_identity'), ('セット', 'set_identity'),
        ('何もしない', 'do_nothing'),
        ('シャッフル', 'shuffle'),
        ('枚になるまで', 'draw_until'),
        ('置く', 'place'), ('置いて', 'place'), ('置き', 'place'),
    ]
    for phrase, verb in pairs:
        if phrase in text:
            return verb
    return 'unknown'


# === Clause Decomposition ===
def decompose(text: str) -> List[Clause]:
    """Split ability text into clauses using structural markers."""
    clauses = []

    if '：' in text:
        parts = text.split('：', 1)
        clauses.append(Clause(role='cost', text=parts[0].strip()))
        text = parts[1]

    # Try structural handlers in priority order
    handler = _find_structure(text)
    if handler:
        result = handler(text)
        if result:
            clauses.extend(result)
            return clauses

    # Sequential markers: 。 and 、
    if '。' in text:
        parts = [p.strip() for p in text.split('。') if p.strip()]
        if len(parts) >= 2:
            for p in parts:
                clauses.append(Clause(role='sentence', text=p))
            return clauses

    if '、' in text:
        parts = [p.strip().rstrip('、') for p in text.split('、') if p.strip()]
        if len(parts) >= 2:
            for p in parts:
                clauses.append(Clause(role='sequential', text=p))
            return clauses

    clauses.append(Clause(role='action', text=text))
    return clauses


def _find_structure(text):
    if 'その中から' in text:
        return _handle_look_select
    if 'これにより' in text and '場合' in text:
        return _handle_cause_effect
    if 'そうしなかった場合' in text or 'そうした場合' in text:
        return _handle_conditional_optional
    if 'かぎり' in text:
        return _handle_duration
    if ('につき' in text or 'ごとに' in text) and 'この能力を起動するためのコストは' not in text:
        return _handle_per_unit
    if any(m in text for m in ['場合、', 'とき、']):
        return _handle_conditional
    if 'さらに' in text:
        return _handle_furthermore
    if '以下から1つを選ぶ' in text:
        return _handle_choice
    return None


def _handle_look_select(text):
    parts = text.split('その中から', 1)
    return [Clause(role='look', text=parts[0].strip()),
            Clause(role='select', text=parts[1].strip())]


def _handle_cause_effect(text):
    parts = text.split('これにより', 1)
    rest = 'これにより' + parts[1]
    cond_parts = rest.split('場合', 1)
    result = [Clause(role='primary', text=parts[0].strip()),
              Clause(role='cause_condition', text=cond_parts[0].strip() + '場合')]
    if len(cond_parts) > 1 and cond_parts[1].strip():
        result.append(Clause(role='cause_result', text=cond_parts[1].strip().lstrip('、，').strip()))
    return result


def _handle_conditional_optional(text):
    for marker, role in [('そうしなかった場合', 'cond_negation'), ('そうした場合', 'cond_affirmation')]:
        if marker in text:
            parts = text.split(marker, 1)
            return [Clause(role='optional_action', text=parts[0].strip()),
                    Clause(role=role, text=parts[1].strip().lstrip('、，').strip())]
    return None


def _handle_duration(text):
    parts = text.split('かぎり', 1)
    return [Clause(role='duration_cond', text=parts[0].strip() + 'かぎり'),
            Clause(role='duration_effect', text=parts[1].strip().lstrip('、，').strip())]


def _handle_per_unit(text):
    m = re.search(r'(.+?)(につき|ごとに)', text)
    if not m: return None
    unit_text = m.group(1).strip()
    remaining = text[m.end():].strip().lstrip('、，').strip()
    for marker in ['場合、', 'とき、']:
        if marker in unit_text:
            parts = unit_text.split(marker, 1)
            return [Clause(role='condition', text=parts[0].strip() + marker.rstrip('、')),
                    Clause(role='per_unit_ref', text=parts[1].strip()),
                    Clause(role='per_unit_action', text=remaining)]
    return [Clause(role='per_unit_ref', text=unit_text),
            Clause(role='per_unit_action', text=remaining)]


def _handle_conditional(text):
    for marker in ['場合、', 'とき、']:
        if marker in text:
            parts = text.split(marker, 1)
            result = [Clause(role='condition', text=parts[0].strip() + marker.rstrip('、'))]
            # Recursively decompose the action part to handle sequential actions
            action_clauses = decompose(parts[1].strip())
            for c in action_clauses:
                if c.role == 'cost': c.role = 'action'
                result.append(c)
            return result
    return None


def _handle_furthermore(text):
    parts = [p.strip() for p in text.split('。') if p.strip()]
    result = []
    for p in parts:
        clean = p.replace('さらに', '', 1).strip() if 'さらに' in p else p.strip()
        result.append(Clause(role='further', text=clean))
    return result


def _handle_choice(text):
    parts = text.split('以下から1つを選ぶ', 1)
    if len(parts) < 2: return None
    opts = [l.strip()[1:].strip() for l in parts[1].split('\n') if l.strip().startswith('・')]
    result = [Clause(role='choice_preamble', text=parts[0].strip())]
    for opt in opts:
        result.append(Clause(role='choice_option', text=opt))
    return result


# === Clause Classification ===
def classify(clause: Clause) -> Clause:
    if not clause.text:
        return clause
    clause.verb = detect_verb(clause.text)
    clause.source = extract_source(clause.text)
    clause.destination = extract_destination(clause.text)
    clause.card_type = extract_card_type(clause.text)
    clause.count = extract_count(clause.text)
    clause.target = extract_target(clause.text)
    clause.optional = extract_optional(clause.text)
    clause.duration = extract_duration(clause.text)
    clause.params = _extract_params(clause)
    return clause


def _extract_params(clause: Clause) -> dict:
    t = clause.text
    p = {}
    m = re.search(r'コスト(\d+)', t)
    if m: p['cost_limit'] = int(m.group(1))
    if '以下' in t: p['cost_limit_operator'] = '<='
    elif '以上' in t: p['cost_limit_operator'] = '>='
    groups = re.findall(r'『([^』]+)』', t)
    if groups: p['group_names'] = groups
    chars = re.findall(r'「([^」]+)」', t)
    if chars: p['characters'] = chars
    energy = t.count('{{icon_energy.png|E}}')
    if energy > 0: p['energy_count'] = energy
    blade = t.count('{{icon_blade.png|ブレード}}')
    if blade > 0: p['blade_icons'] = blade
    hearts = list(set('heart' + m for m in re.findall(r'heart_(\d+)', t)))
    if hearts: p['heart_colors'] = hearts
    if 'ウェイト' in t: p['state_change'] = 'wait'
    elif 'アクティブ' in t: p['state_change'] = 'active'

    if clause.verb == 'modify_score':
        p['operation'] = 'add' if ('プラス' in t or '+' in t or '＋' in t) else 'remove'
        if p.get('count') is None:
            m = re.search(r'[+＋](\d+)', t)
            if m: p['count'] = int(m.group(1))
    if clause.verb == 'mod_cost':
        p['operation'] = 'subtract' if ('減る' in t or '減らす' in t or '少なくなる' in t) else 'increase'
    if clause.verb == 'modify_hearts':
        p['operation'] = 'decrease' if ('減らす' in t or '減る' in t) else 'increase'
        heart = re.search(r'{{heart_(\d+)\.png\|heart\d+}}', t)
        if heart: p['heart_color'] = 'heart' + heart.group(1).zfill(2)
    if clause.verb == 'change_wait':
        p['state_change'] = 'wait'
    elif clause.verb == 'change_active':
        p['state_change'] = 'active'
    if clause.verb == 'set_blade_type':
        m = re.search(r'すべて\[([^\]]+)\]', t)
        if m: p['blade_type'] = m.group(1)
    if clause.verb == 'place' and (clause.source or clause.destination):
        clause.verb = 'move'
        p['placement_order'] = 'any_order' if '好きな順番で' in t else None
    if 'このメンバーを' in t and ('ウェイトに' in t or 'ステージから' in t):
        p['self_cost'] = True
    if 'このメンバー以外' in t or 'ほかのメンバー' in t or 'このカード以外' in t:
        p['exclude_self'] = True
    if '好きな枚数' in t:
        p['any_number'] = True
    if '枚まで' in t or '人まで' in t:
        p['max'] = True

    return p

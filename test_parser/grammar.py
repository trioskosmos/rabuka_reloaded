"""Grammar-based ability parser.

Japanese ability text follows a grammatical structure where particles (から、を、に、の)
determine the SEMANTIC ROLE of each noun phrase, regardless of specific words.

A MOVE action always has the pattern:
    {source}から {object}を {destination}に {verb}

The grammar extracts roles from particles, then infers action from the verb.
This naturally handles ALL variants without per-pattern templates.
"""

from __future__ import annotations
import re
from typing import Any, Dict, List, Optional, Tuple

# ------------------------------------------------------------------
# Grammar rules for Japanese ability text
# ------------------------------------------------------------------

# Particle-to-role mapping
PARTICLE_ROLES = {
    'から': 'source',      # Xから → source location
    'を': 'object',        # Xを → direct object
    'に': 'destination',   # Xに → destination location/target
    'の': 'owner',         # Xの → owner/possessor
    'が': 'subject',       # Xが → grammatical subject
    'は': 'topic',         # Xは → topic
}

# Verb-to-action mapping
VERB_ACTIONS: Dict[str, str] = {
    # Movement verbs
    '加える': 'move_cards',
    '加え': 'move_cards',
    '置く': 'move_cards',
    '置い': 'move_cards',
    '置き': 'move_cards',
    '送る': 'move_cards',
    '戻す': 'move_cards',
    '登場させる': 'move_cards',
    '引く': 'draw_card',
    '引き': 'draw_card',
    '見る': 'look_at',
    '見て': 'look_at',
    # State change verbs
    'アクティブにする': 'change_state',
    'ウェイトにする': 'change_state',
    # Gain verbs
    '得る': 'gain_resource',
        # Selection verbs
    '選ぶ': 'select',
    '選ん': 'select',
    '選択する': 'select',
    '公開する': 'reveal',
    '公開し': 'reveal',
    '指定する': 'gain_resource',
    '指定し': 'gain_resource',
    '失い': 're_yell',
    'エールを行う': 're_yell',
    # Position
    'ポジションチェンジする': 'position_change',
    'ポジションチェンジし': 'position_change',
    '入れ替える': 'position_change',
    '移動させる': 'position_change',
    '移動させ': 'position_change',
    # Ability
    '発動させる': 'activate_ability',
    '無効にする': 'invalidate_ability',
    '無効にし': 'invalidate_ability',
    # Cost
    '支払う': 'pay_energy',
    '支払って': 'pay_energy',
    # Restriction
    'できない': 'restriction',
    '置けない': 'restriction',
    '登場できない': 'restriction',
}

# State detection
STATE_PATTERNS = {
    'ウェイト': 'wait',
    'アクティブ': 'active',
}

# Resource detection
RESOURCE_PATTERNS = {
    'ブレード': 'blade',
    'ハート': 'heart',
    'エネルギー': 'energy',
}

# Card type detection
CARD_TYPE_PATTERNS = {
    'メンバーカード': 'member_card',
    'メンバー': 'member_card',
    'ライブカード': 'live_card',
    'エネルギーカード': 'energy_card',
    'カード': 'card',
}


# ------------------------------------------------------------------
# Tokenizer: extract particle-phrase pairs from text
# ------------------------------------------------------------------

def tokenize(text: str) -> List[Tuple[str, str, int]]:
    """Split text into (phrase, particle, position) triples.
    Uses particles as delimiters to find phrase-particle pairs."""
    tokens = []
    # Find all particle positions
    positions = []
    for particle in ['から', 'を', 'に', 'の', 'が', 'は']:
        pos = 0
        while True:
            idx = text.find(particle, pos)
            if idx == -1:
                break
            # Find the start of this phrase (word boundary)
            positions.append((idx, particle, idx))
            pos = idx + len(particle)
    
    # Sort by position
    positions.sort()
    
    # Extract phrase for each particle
    for i, (end_pos, particle, _) in enumerate(positions):
        if i == 0:
            start = 0
        else:
            prev_end = positions[i-1][0] + len(positions[i-1][1])
            # Phrase starts after previous particle + any connector
            start = prev_end
        
        phrase = text[start:end_pos].strip().strip('、、')
        # Clean up leading/trailing markers
        phrase = re.sub(r'^[\s、，]+|[\s、，]+$', '', phrase)
        if phrase:
            tokens.append((phrase, particle, end_pos))
    
    return tokens


def extract_noun(text: str, start: int) -> Tuple[str, int]:
    """Extract a noun phrase starting from position start.
    Returns (noun_phrase, end_position)."""
    # A noun phrase ends at the next particle or punctuation
    end = start
    while end < len(text):
        ch = text[end]
        if ch in 'からをにのがは、。；\n':
            break
        end += 1
    return text[start:end].strip(), end


# ------------------------------------------------------------------
# Role extractor: from tokens to semantic roles
# ------------------------------------------------------------------

def extract_roles(tokens: List[Tuple[str, str, int]]) -> Dict[str, str]:
    """Convert particle-phrase tokens to semantic role dict."""
    roles: Dict[str, str] = {}
    
    for phrase, particle, _ in tokens:
        role = PARTICLE_ROLES.get(particle)
        if not role:
            continue
        
        # Detect card type in the phrase
        for keyword, ct in CARD_TYPE_PATTERNS.items():
            if keyword in phrase:
                roles['card_type'] = ct
                break
        
        # Detect group name in 『』
        gm = re.search(r'『([^』]+)』', phrase)
        if gm:
            roles['group'] = gm.group(1)
        
        # Detect count in phrase
        cm = re.search(r'(\d+)', phrase)
        if cm and role in ('destination', 'object'):
            roles['count'] = int(cm.group(1))
        
        # Set the role value
        if role == 'source':
            # Clean up: remove "自分の" prefix, extract location
            for loc in ['控え室', '手札', 'デッキ', 'ステージ', 
                        'エネルギー置き場', 'エネルギーゾーン',
                        'ライブカード置き場', '成功ライブカード置き場',
                        '山札', 'デッキの上', 'デッキの一番上',
                        'デッキの下', 'デッキの一番下']:
                if loc in phrase:
                    roles[role] = loc
                    break
            if role not in roles:
                roles[role] = phrase
        
        elif role == 'destination':
            for loc in ['手札', '控え室', 'ステージ', 'デッキ',
                        'エネルギー置き場', 'エネルギーゾーン',
                        'ライブカード置き場', '成功ライブカード置き場',
                        'デッキの上', 'デッキの一番上',
                        'デッキの下', 'デッキの一番下',
                        'メンバーのいないエリア',
                        '同じエリア', 'そのメンバーがいたエリア',
                        'このメンバーの下']:
                if loc in phrase:
                    roles[role] = loc
                    break
            if role not in roles:
                roles[role] = phrase
    
    return roles


# ------------------------------------------------------------------
# Verb extractor and action inference
# ------------------------------------------------------------------

def extract_verb(text: str) -> Tuple[Optional[str], Optional[str]]:
    """Extract the verb from text and determine action type.
    Returns (verb_text, action_type)."""
    for verb, action in sorted(VERB_ACTIONS.items(), key=lambda x: -len(x[0])):
        if verb in text:
            return verb, action
    return None, None


def extract_state(text: str) -> Optional[str]:
    """Extract state change (wait/active) from text."""
    for keyword, state in STATE_PATTERNS.items():
        if keyword in text and 'にする' in text:
            return state
    return None


def extract_resource(text: str) -> Optional[str]:
    """Extract resource type from text."""
    for keyword, resource in RESOURCE_PATTERNS.items():
        if keyword in text:
            return resource
    return None


def extract_target(text: str) -> Optional[str]:
    """Extract target (self/opponent/both) from text."""
    if '相手の' in text and '自分の' in text:
        return 'both'
    if '相手の' in text or '相手は' in text:
        return 'opponent'
    if '自分の' in text or '自分は' in text:
        return 'self'
    return None


def extract_optional(text: str) -> bool:
    """Check if action is optional."""
    return 'もよい' in text or 'てもよい' in text


# ------------------------------------------------------------------
# Full grammar-based parse
# ------------------------------------------------------------------

def parse_action(text: str) -> Dict[str, Any]:
    """Parse an action text using Japanese grammar rules."""
    result: Dict[str, Any] = {'text': text}
    
    # 1. Extract verb
    verb, action = extract_verb(text)
    if not verb:
        # Fallback: check known substrings
        if '無効に' in text:
            verb, action = '無効にする', 'invalidate_ability'
        elif '起動' in text:
            verb, action = '起動する', 'activate_ability'
        elif '発動' in text:
            verb, action = '発動させる', 'activate_ability'
        elif 'ポジションチェンジ' in text:
            verb, action = 'ポジションチェンジする', 'position_change'
        elif 'シャッフル' in text:
            verb, action = 'シャッフルする', 'shuffle'
        elif 'できない' in text or '置けない' in text or '登場できない' in text:
            verb, action = 'できない', 'restriction'
        elif '必要ハート' in text and ('減らす' in text or '増やす' in text):
            verb, action = '変更する', 'modify_required_hearts'
        elif '必要ハート' in text and ('なる' in text):
            verb, action = '変わる', 'modify_required_hearts'
        elif 'スコアを' in text and ('+' in text or 'プラス' in text or '-' in text or 'マイナス' in text):
            verb, action = '変更する', 'modify_score'
        elif 'エール' in text and '枚数' in text:
            verb, action = '変更する', 'modify_yell_count'
        elif '公開' in text:
            verb, action = '公開する', 'reveal'
        elif '選ぶ' in text:
            verb, action = '選ぶ', 'select'
        elif '得る' in text:
            verb, action = '得る', 'gain_resource'
        else:
            result['action'] = 'custom'
            return result
    
    result['action'] = action
    result['verb'] = verb
    
    # 2. Tokenize and extract roles
    tokens = tokenize(text)
    roles = extract_roles(tokens)
    
    # 3. Apply semantic roles
    for role in ('source', 'destination', 'card_type', 'count', 'group'):
        if role in roles:
            result[role] = roles[role]
    
    # 4. Extract additional fields
    state = extract_state(text)
    if state:
        result['state_change'] = state
    
    resource = extract_resource(text)
    if resource:
        result['resource'] = resource
    
    target = extract_target(text)
    if target:
        result['target'] = target
    
    result['optional'] = extract_optional(text)
    
    # 5. Apply defaults per action type
    if action == 'draw_card':
        result.setdefault('source', 'deck')
        result.setdefault('destination', 'hand')
    
    # 6. Extract count from text if not found by tokenizer
    if 'count' not in result:
        cm = re.search(r'(\d+)(?:枚|人|つ|回)', text)
        if cm:
            result['count'] = int(cm.group(1))
    
    # 7. Detect duration prefix
    for prefix, code in [('ライブ終了時まで', 'live_end'), ('このターンの間', 'this_turn')]:
        if text.startswith(prefix):
            result['duration'] = code
            break
    
    return result


def parse_cost(text: str) -> Optional[Dict]:
    """Parse cost text."""
    if '{{icon_energy.png|E}}' in text:
        energy = text.count('{{icon_energy.png|E}}')
        return {
            'type': 'pay_energy',
            'energy': energy,
            'optional': extract_optional(text),
        }
    # Try move_cards cost
    if ('ステージから' in text or '手札を' in text) and '控え室に置く' in text:
        src = 'stage' if 'ステージから' in text else 'hand'
        return {
            'type': 'move_cards',
            'source': src, 'destination': 'discard',
            'card_type': 'member_card', 'self_cost': True,
            'optional': extract_optional(text),
        }
    # Try change_state cost
    if 'ウェイトにする' in text or 'アクティブにする' in text:
        return {
            'type': 'change_state',
            'state_change': 'wait' if 'ウェイト' in text else 'active',
            'card_type': 'member_card', 'self_cost': True,
            'optional': extract_optional(text),
        }
    return None


def split_cost_effect(text: str) -> Tuple[str, str]:
    if '：' not in text:
        return '', text
    paren_depth = 0
    for i, ch in enumerate(text):
        if ch in '（(':
            paren_depth += 1
        elif ch in '）)':
            paren_depth -= 1
        elif ch == '：' and paren_depth == 0:
            return text[:i].strip(), text[i+1:].strip()
    return '', text


def parse_ability_text(triggerless_text: str) -> Dict:
    """Full ability parse using grammar-based approach."""
    result = {'triggerless_text': triggerless_text}
    cost_text, effect_text = split_cost_effect(triggerless_text)
    
    if cost_text:
        cost = parse_cost(cost_text)
        if cost:
            result['cost'] = cost
    
    if effect_text:
        result['effect'] = parse_action(effect_text)
    
    return result

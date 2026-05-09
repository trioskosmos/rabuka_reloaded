"""Generate test scenarios from abilities.json.

For each unique ability, determines:
- Which card has it
- What game setup is needed (cards in zones, energy, phase)
- What the expected effect/cost structure should be
- Key fields to verify (source, destination, card_type, count, etc.)

Output: test_runner/scenarios.json
"""
import json, re, sys
from pathlib import Path
from collections import defaultdict

ABILITIES = Path(__file__).parent / 'abilities.json'
OUTPUT = Path(__file__).parent / 'scenarios.json'

data = json.load(open(ABILITIES, encoding='utf-8'))
entries = data['unique_abilities']
FILLER = "PL!-sd1-010-SD"

scenarios = []

for i, entry in enumerate(entries):
    t = entry.get('triggerless_text', '')
    cards = entry.get('cards', [])
    triggers = entry.get('triggers') or ''
    is_null = entry.get('is_null', False)
    if not t or is_null or not cards:
        continue

    eff = entry.get('effect') or {}
    cost = entry.get('cost') or {}
    action = eff.get('action', '')
    
    # Get first card this ability belongs to
    first_card = cards[0].split(' | ')[0] if ' | ' in cards[0] else cards[0]
    
    # Determine setup requirements
    need_stage = bool(re.search(r'登場|起動|常時', triggers))
    need_live_phase = bool(re.search(r'ライブ開始時|ライブ成功時', triggers))
    need_hand_cards = 3  # Always give some cards
    need_energy = 10  # Always give energy
    need_discard_cards = 3 if '控え室' in t and ('から' in t or '加える' in t) else 1
    need_deck_cards = 5 if 'デッキ' in t or '山札' in t else 3
    
    # Determine expected effect structure
    expected_source = eff.get('source')
    expected_destination = eff.get('destination')
    expected_card_type = eff.get('card_type')
    expected_count = eff.get('count')
    expected_target = eff.get('target', 'self')
    
    # Determine if selection should happen
    has_selection = '選ぶ' in t or '選ん' in t or '選び' in t
    has_cost_selection = '選ぶ' in cost.get('text', '') or '選ん' in cost.get('text', '')
    is_optional = 'もよい' in t
    has_conditional_on_optional = 'そうした場合' in t or 'そうしなかった場合' in t
    
    # Determine which zone to check after execution
    check_zones = []
    dest = expected_destination
    src = expected_source
    if dest == 'hand': check_zones.append('hand')
    elif dest == 'discard': check_zones.append('discard')
    elif dest == 'deck_top': check_zones.append('deck_top')
    elif dest == 'stage': check_zones.append('stage')
    elif dest == 'energy_zone': check_zones.append('energy_zone')
    
    if src == 'discard': check_zones.append('discard')
    
    scenario = {
        'index': i,
        'card_no': first_card,
        'triggers': triggers,
        'action': action,
        'text': t[:80],
        'setup': {
            'stage': need_stage,
            'live_phase': need_live_phase,
            'hand_cards': need_hand_cards,
            'energy': need_energy,
            'discard_cards': need_discard_cards,
            'deck_cards': need_deck_cards,
        },
        'expected': {
            'action': action,
            'source': expected_source,
            'destination': expected_destination,
            'card_type': expected_card_type,
            'count': expected_count,
            'target': expected_target,
            'has_selection': has_selection or has_cost_selection,
            'optional': is_optional,
            'conditional_on_optional': has_conditional_on_optional,
            'cost_type': cost.get('type', ''),
        },
        'checks': {
            'has_energy_in_text': 'エネルギー' in t,
            'has_member_in_text': 'メンバー' in t,
            'has_cost_limit': bool(re.search(r'コスト\d+', t)),
            'has_color_filter': 'heart_' in t or 'ハート' in t,
        },
    }
    scenarios.append(scenario)

OUTPUT.parent.mkdir(parents=True, exist_ok=True)
with open(OUTPUT, 'w', encoding='utf-8') as f:
    json.dump(scenarios, f, ensure_ascii=False, indent=2)

print(f"Generated {len(scenarios)} scenarios -> {OUTPUT}")

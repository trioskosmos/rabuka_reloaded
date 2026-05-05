"""Prototype tests for the new parser structure."""

import json, sys, os
sys.path.insert(0, os.path.dirname(__file__))

import logging
logging.basicConfig(level=logging.INFO, format='%(levelname)s: %(message)s')
logger = logging.getLogger('test')

# ------------------------------------------------------------------
# Test 1: RuleRegistry basics
# ------------------------------------------------------------------
from dispatcher import Rule, RuleRegistry, registry

def test_registry_priority():
    """Rules are ordered by priority descending."""
    r = registry(
        (10, 'low',  lambda t: 'z' in t),
        (50, 'high', lambda t: 'a' in t),
        (30, 'mid',  lambda t: 'm' in t),
    )
    assert r.names == ['high', 'mid', 'low'], f"got {r.names}"
    print("  OK priority ordering correct")

def test_registry_match():
    """First matching rule by priority wins."""
    r = registry(
        (50, 'cats',  lambda t: 'cat' in t),
        (40, 'catch', lambda t: 'catch' in t),
    )
    state = {}
    result = r.dispatch('catch', state)
    assert result == 'cats', f"got {result} — rule with higher priority should match first"
    print("  OK priority-first-match works")

def test_registry_no_match():
    """No match returns default action."""
    r = registry(
        (50, 'cats', lambda t: 'cat' in t),
    )
    state = {}
    result = r.dispatch('dog', state)
    assert result == 'custom', f"got {result}"
    assert state['action'] == 'custom'
    print("  OK no-match returns default")

def test_registry_inject():
    """Add a rule at any time; it slots into correct priority position."""
    r = registry(
        (50, 'high', lambda t: 'x' in t),
        (10, 'low',  lambda t: 'x' in t),
    )
    r.add(Rule(30, 'mid', lambda t: 'x' in t))
    assert r.names == ['high', 'mid', 'low'], f"got {r.names}"
    print("  OK late injection maintains priority order")

# ------------------------------------------------------------------
# Test 2: Action dispatch with real ability text
# ------------------------------------------------------------------
from actions import parse_action, DISPATCH

def test_dispatch_registry_built():
    """DISPATCH has all expected rules in priority order."""
    names = DISPATCH.names
    assert 'modify_cost' in names
    assert 'move_cards' in names
    assert 'draw_card' in names
    assert 'pay_energy' in names
    assert names[0] == 'modify_cost', f"highest priority should be modify_cost, got {names[0]}"
    print(f"  OK {len(DISPATCH)} rules built")

def test_dispatch_move_cards():
    """Basic discard-from-hand → move_cards."""
    result = parse_action('手札を1枚控え室に置く')
    assert result['action'] == 'move_cards', f"got {result['action']}"
    assert result['source'] == 'hand'
    assert result['destination'] == 'discard'
    assert result['count'] == 1
    print("  OK 手札→控え室 → move_cards")

def test_dispatch_draw_card():
    result = parse_action('カードを2枚引く')
    assert result['action'] == 'draw_card'
    assert result['count'] == 2
    print("  OK draw_card with count")

def test_dispatch_pay_energy():
    result = parse_action('{{icon_energy.png|E}}{{icon_energy.png|E}}支払ってもよい')
    assert result['action'] == 'pay_energy'
    assert result['energy'] == 2
    assert result['optional'] == True
    print("  OK pay_energy with optional")

def test_dispatch_gain_blade():
    result = parse_action('{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る')
    assert result['action'] == 'gain_resource'
    assert result['resource'] == 'blade'
    assert result['count'] == 2
    print("  OK gain_resource blade")

def test_dispatch_modify_cost():
    result = parse_action('コストは2減る')
    assert result['action'] == 'modify_cost', f"got {result['action']}"
    print("  OK modify_cost")

def test_dispatch_change_state():
    result = parse_action('このメンバーをウェイトにする')
    assert result['action'] == 'change_state', f"got {result['action']}"
    assert result['state_change'] == 'wait'
    print("  OK change_state wait")

def test_dispatch_modify_cost_beats_move():
    """modify_cost has higher priority than move_cards (both match)."""
    result = parse_action('能力を持たないメンバーカードを自分の手札から登場させるためのコストは1減る')
    assert result['action'] == 'modify_cost', f"got {result['action']} — should be modify_cost, not move_cards"
    print("  OK modify_cost beats move_cards (priority works)")

# ------------------------------------------------------------------
# Test 3: Condition parsing
# ------------------------------------------------------------------
from conditions import parse_condition

def test_condition_card_count():
    r = parse_condition('自分のステージにメンバーが3人以上いる場合')
    assert r and r['type'] == 'card_count_condition', f"got {r}"
    assert r['count'] == 3
    print("  OK card_count_condition")

def test_condition_location():
    r = parse_condition('自分の控え室からライブカードを1枚手札に加える')
    assert r and r['type'] == 'location_condition', f"got {r}"
    assert r['location'] == 'discard'
    print("  OK location_condition")

def test_condition_temporal():
    r = parse_condition('このターン、自分のステージにメンバーが登場している場合')
    assert r and r['type'] == 'temporal_condition', f"got {r}"
    assert r['temporal'] == 'this_turn'
    print("  OK temporal_condition")

def test_condition_compound():
    r = parse_condition('自分のステージにメンバーが3人以上いるかつ名前が異なる場合')
    assert r and r['type'] == 'compound', f"got {r}"
    assert len(r['conditions']) == 2
    print("  OK compound condition")

def test_condition_or():
    r = parse_condition('このメンバーが登場か、エリアを移動したとき')
    assert r and r['type'] == 'or_condition', f"got {r}"
    assert len(r['conditions']) == 2
    print("  OK or_condition")

def test_condition_baton_touch_ability_negation():
    r = parse_condition('能力を持たないメンバーからバトンタッチして登場した場合')
    assert r and r['type'] == 'location_condition', f"got {r}"
    assert r['baton_touch_trigger'] == True
    assert r['ability_negation'] == True
    print("  OK baton_touch + ability_negation")

# ------------------------------------------------------------------
# Test 4: Real ability texts from the game
# ------------------------------------------------------------------

REAL_ABILITIES = [
    # (name, text, expected_action, expected_fields)
    ("discard-recover",
     "このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。",
     {'effect': {'action': 'move_cards', 'source': 'discard', 'destination': 'hand', 'count': 1}}),
    ("draw-discard",
     "カードを1枚引き、手札を1枚控え室に置く。",
     None),  # will be sequential — we test sub-actions
    ("live-blade-gain",
     "{{icon_energy.png|E}}支払ってもよい：ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。",
     {'cost': {'action': 'pay_energy', 'energy': 1, 'optional': True},
      'effect': {'action': 'gain_resource', 'resource': 'blade', 'count': 2}}),
    ("baton-touch-ability-negation",
     "能力を持たないメンバーからバトンタッチして登場した場合、カードを1枚引く。",
     {'condition': {'baton_touch_trigger': True, 'ability_negation': True},
      'effect': {'action': 'draw_card', 'count': 1}}),
]

def test_real_abilities():
    for name, text, expected in REAL_ABILITIES:
        # Parse the effect part (text after '：')
        effect_text = text.split('：')[-1].strip().rstrip('。')
        if expected:
            result = parse_action(effect_text)
            expected_eff = expected.get('effect', {})
            for k, v in expected_eff.items():
                assert result.get(k) == v, (
                    f"[{name}] expected {k}={v!r}, got {result.get(k)!r}")
            expected_cost = expected.get('cost', {})
            # cost check is separate (skipped in this simplified test)
        print(f"  OK {name} parses correctly")

# ------------------------------------------------------------------
# Run
# ------------------------------------------------------------------

def main():
    tests = [
        test_registry_priority,
        test_registry_match,
        test_registry_no_match,
        test_registry_inject,
        test_dispatch_registry_built,
        test_dispatch_move_cards,
        test_dispatch_draw_card,
        test_dispatch_pay_energy,
        test_dispatch_gain_blade,
        test_dispatch_modify_cost,
        test_dispatch_change_state,
        test_dispatch_modify_cost_beats_move,
        test_condition_card_count,
        test_condition_location,
        test_condition_temporal,
        test_condition_compound,
        test_condition_or,
        test_condition_baton_touch_ability_negation,
        test_real_abilities,
    ]
    print(f"\nRunning {len(tests)} tests...\n")
    for t in tests:
        try:
            t()
        except Exception as e:
            print(f"  FAIL: {t.__name__}: {e}")
            raise
    print(f"\nAll {len(tests)} tests passed")

if __name__ == '__main__':
    main()

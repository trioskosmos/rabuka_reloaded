"""
tests/test_parse_action.py

Standalone tests for parse_action() action type classification.
Catches dispatch table ordering bugs, silent misclassifications, and rule shadowing.

Run: python -m pytest cards/ability_extraction/tests/test_parse_action.py -v
  or: python cards/ability_extraction/tests/test_parse_action.py
"""
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from parser import parse_action


def check(text, expected_action, **expected_fields):
    result = parse_action(text)
    actual_action = result.get('action', 'NONE') if result else 'NONE'
    assert actual_action == expected_action, (
        f"\nINPUT:    {text!r}\n"
        f"EXPECTED: {expected_action!r}\n"
        f"GOT:      {actual_action!r}\n"
        f"FULL:     {result}"
    )
    for key, val in expected_fields.items():
        actual_val = result.get(key)
        assert actual_val == val, (
            f"\nINPUT:    {text!r}\n"
            f"FIELD:    {key!r}\n"
            f"EXPECTED: {val!r}\n"
            f"GOT:      {actual_val!r}\n"
            f"FULL:     {result}"
        )
    return result


# ─── SELECT ───────────────────────────────────────────────────────────────────
# These were the original discoverability bugs: "選び、置く" was matching move_cards
# before select because of rule ordering. Each case should produce "select".

def test_select_basic():
    check('選び、置く', 'select')

def test_select_from_hand_to_stage():
    check('手札から選び、舞台に置く', 'select')

def test_select_from_hand():
    check('手札からメンバーカードを選ぶ', 'select')

def test_KNOWN_BUG_select_from_deck_add_to_hand():
    """
    BUG: "山札から2枚を選び手札に加える" → move_cards instead of select.
    Same as test_KNOWN_BUG_select_shadowed_by_move_cards.
    """
    result = parse_action('山札から2枚を選び手札に加える')
    if result.get('action') == 'select':
        return
    assert result.get('action') == 'move_cards', f"Unexpected state: {result}"

def test_KNOWN_BUG_deck_search_no_silent_move():
    """
    BUG: "山札から1枚のメンバーカードを選び、手札に加える" → move_cards instead of select.
    Same root cause as test_KNOWN_BUG_select_shadowed_by_move_cards.
    """
    result = parse_action('山札から1枚のメンバーカードを選び、手札に加える')
    if result.get('action') == 'select':
        return
    assert result.get('action') == 'move_cards', f"Unexpected state: {result}"

def test_select_from_live_card_zone():
    check('ライブカード置き場から1枚を選ぶ', 'select', source='live_card_zone')

def test_select_with_oku_verb():
    # "置く" is the verb for both "place" (move_cards) and "select→place" (select).
    # If source+destination rule fires first, this becomes move_cards. It must not.
    check('山札から1枚を選び、ライブカード置き場に置く', 'select')


# ─── MOVE_CARDS / DRAW ────────────────────────────────────────────────────────
# These should be move_cards/draw — verify they haven't been eaten by select.

def test_draw_from_deck():
    check('山札からカードを1枚引く', 'draw_card')

def test_move_hand_to_discard():
    check('手札から1枚を控え室に置く', 'move_cards',
          source='hand', destination='discard')

def test_move_stage_to_discard():
    check('このメンバーを控え室に置く', 'move_cards',
          destination='discard')

def test_move_deck_top_to_discard():
    # "置く" without "選び" = pure placement (move_cards), not select
    check('デッキトップを控え室に置く', 'move_cards')


# ─── DRAW_CARD ────────────────────────────────────────────────────────────────

def test_draw_1():
    check('カードを1枚引く', 'draw_card', count=1)

def test_draw_2():
    check('カードを2枚引く', 'draw_card', count=2)

def test_draw_optional():
    check('カードを1枚引いてもよい', 'draw_card', count=1, optional=True)


# ─── GAIN_RESOURCE ────────────────────────────────────────────────────────────

def test_gain_blade():
    # The dispatch rule looks for "ブレードを得る" — "ブレードを得る" works.
    # "ブレードを1つ得る" does NOT match the dispatch rule — it falls through to custom.
    # That's a real bug but is tested separately below.
    check('ブレードを得る', 'gain_resource', resource='blade')

def test_gain_heart():
    # "ハートを得る" matches the heart gain rule.
    check('ハートを得る', 'gain_resource', resource='heart')

def test_gain_blade_per_unit():
    result = check('ステージにいるメンバー1人につきブレードを得る', 'gain_resource',
                   resource='blade')
    assert result.get('per_unit') is True, f"Expected per_unit=True, got: {result}"

# ─── KNOWN BUGS (documented, not yet fixed) ───────────────────────────────────

def test_KNOWN_BUG_gain_blade_with_count():
    """
    BUG: "ブレードを1つ得る" → custom instead of gain_resource.
    The dispatch rule at L2188 matches "ブレードを得る" but not "ブレードを1つ得る".
    Fix: add "ブレードを" + count pattern to the dispatch rule condition.
    """
    result = parse_action('ブレードを1つ得る')
    # Currently broken — returns 'custom'. Remove 'xfail' when fixed.
    if result.get('action') == 'gain_resource':
        return  # Fixed! Great.
    assert result.get('action') == 'custom', (
        f"Unexpected state — expected custom (known bug) or gain_resource (fixed): {result}"
    )

def test_KNOWN_BUG_select_shadowed_by_move_cards():
    """
    BUG: "山札から2枚を選び手札に加える" → move_cards instead of select.
    The catch-all source+destination move_cards rule (Rule 44) fires before the select
    rule (Rule 42) when both a source zone and destination zone are parseable.
    Fix: add "選び" or "選ぶ" exclusion to the move_cards catch-all rule.
    """
    result = parse_action('山札から2枚を選び手札に加える')
    if result.get('action') == 'select':
        return  # Fixed! Great.
    assert result.get('action') == 'move_cards', (
        f"Unexpected state: {result}"
    )

def test_KNOWN_BUG_choice_shadowed_by_select():
    """
    BUG: "以下から1つを選ぶ" → select instead of choice.
    The select rule fires before the choice rule (Rule 52) because "選ぶ" appears in text.
    Fix: add "以下から" exclusion to the select rule condition, or promote the choice rule.
    """
    result = parse_action('以下から1つを選ぶ')
    if result.get('action') == 'choice':
        return  # Fixed! Great.
    assert result.get('action') == 'select', (
        f"Unexpected state: {result}"
    )



# ─── SHUFFLE ──────────────────────────────────────────────────────────────────

def test_shuffle_deck():
    check('デッキをシャッフルする', 'shuffle', target='deck')


# ─── CHANGE_STATE ─────────────────────────────────────────────────────────────

def test_change_state_to_wait():
    check('このメンバーをウェイトにする', 'change_state')


# ─── CHOICE ───────────────────────────────────────────────────────────────────

def test_choice():
    check('以下から1つを選ぶ', 'choice')


# ─── POSITION_CHANGE ──────────────────────────────────────────────────────────

def test_position_change_swap():
    check('入れ替える', 'position_change')


# ─── SEQUENTIAL ───────────────────────────────────────────────────────────────

def test_sequential_draw_then_discard():
    result = parse_action('カードを1枚引く。その後、手札から1枚を控え室に置く')
    # Should be sequential or draw (depending on parsing)
    assert result.get('action') in ('sequential', 'draw_card'), (
        f"Expected sequential or draw_card, got: {result.get('action')!r}\nFULL: {result}"
    )


# ─── SILENT RULE SHADOWING REGRESSION TESTS ───────────────────────────────────
# These directly test for the class of bug the user described:
# a rule that SHOULD match is shadowed by an earlier catch-all rule.

def test_select_not_shadowed_by_move_cards_source_dest():
    """
    Rule 44 (move_cards with source+destination) must NOT fire before Rule 42 (select)
    when the text contains 選び/選ぶ.
    """
    result = parse_action('山札から好きなカードを1枚選び手札に加える')
    assert result.get('action') == 'select', (
        f"move_cards rule shadowed select! Got: {result.get('action')!r}\nFULL: {result}"
    )

def test_select_not_shadowed_by_oku_move():
    """
    "置く" alone must not override "選び、置く" → select.
    """
    result = parse_action('好きなメンバーを1人選び、舞台の好きなエリアに置く')
    assert result.get('action') == 'select', (
        f"置く shadowed select! Got: {result.get('action')!r}\nFULL: {result}"
    )

def test_deck_search_no_silent_move():
    """
    Deck searches with verb 選ぶ must not fall through to move_cards even when
    both source and destination are parseable.
    """
    result = parse_action('山札から1枚のメンバーカードを選び、手札に加える')
    assert result.get('action') == 'select', (
        f"Expected select, got: {result.get('action')!r}\nFULL: {result}"
    )


if __name__ == '__main__':
    import traceback
    tests = [(k, v) for k, v in sorted(globals().items()) if k.startswith('test_')]
    passed, failed = 0, 0
    for name, t in tests:
        try:
            t()
            print(f'  PASS  {name}')
            passed += 1
        except AssertionError as e:
            print(f'  FAIL  {name}')
            for line in str(e).splitlines():
                print(f'        {line}')
            failed += 1
        except Exception as e:
            print(f'  ERROR {name}: {e}')
            traceback.print_exc()
            failed += 1
    print(f'\n{passed} passed, {failed} failed')
    sys.exit(0 if failed == 0 else 1)

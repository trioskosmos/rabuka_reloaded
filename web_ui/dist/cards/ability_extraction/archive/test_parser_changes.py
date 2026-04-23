"""Test the parser changes for ability gain and character name extraction"""
import json
import sys
import os

# Add the ability_extraction directory to the path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from parser import parse_action, parse_condition

# Test cases for ability gain
ability_gain_tests = [
    {
        'text': '「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。',
        'expected_action': 'gain_ability',
        'expected_has_ability_text': True,
        'description': 'Ability gain with icon syntax (should be parsed as ability)'
    },
    {
        'text': '「中須かすみ」から能力を得る。',
        'expected_action': 'gain_ability',
        'expected_has_ability_source': True,
        'description': 'Ability gain with character name (should be parsed as character)'
    },
    {
        'text': '「中須かすみ」からバトンタッチして登場した',
        'expected_type': 'baton_touch_condition',
        'expected_has_source_member': True,
        'description': 'Baton touch with character name'
    },
    {
        'text': '「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」からバトンタッチして登場した',
        'expected_type': 'baton_touch_condition',
        'expected_has_source_ability': True,
        'description': 'Baton touch with ability text'
    },
    {
        'text': '「中須かすみ」以外のメンバー',
        'expected_has_except_quoted': True,
        'description': 'Except condition with character name'
    },
    {
        'text': '「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」以外の能力',
        'expected_has_except_abilities': True,
        'description': 'Except condition with ability text'
    },
]

print("Testing parser changes...")
print("=" * 80)

passed = 0
failed = 0

for test in ability_gain_tests:
    text = test['text']
    print(f"\nTest: {test['description']}")
    print(f"Text: {text}")
    
    # Parse as action or condition depending on test
    if 'expected_action' in test or 'expected_has_ability_text' in test or 'expected_has_ability_source' in test:
        result = parse_action(text)
    else:
        result = parse_condition(text)
    
    print(f"Result: {json.dumps(result, ensure_ascii=False, indent=2)}")
    
    # Check expected values
    test_passed = True
    if 'expected_action' in test:
        if result.get('action') != test['expected_action']:
            print(f"  ❌ Expected action: {test['expected_action']}, got: {result.get('action')}")
            test_passed = False
    if 'expected_type' in test:
        if result.get('type') != test['expected_type']:
            print(f"  ❌ Expected type: {test['expected_type']}, got: {result.get('type')}")
            test_passed = False
    if 'expected_has_ability_text' in test:
        if 'ability_text' not in result:
            print(f"  ❌ Expected ability_text, but not found")
            test_passed = False
    if 'expected_has_ability_source' in test:
        if 'ability_source' not in result:
            print(f"  ❌ Expected ability_source, but not found")
            test_passed = False
    if 'expected_has_source_member' in test:
        if 'source_member' not in result:
            print(f"  ❌ Expected source_member, but not found")
            test_passed = False
    if 'expected_has_source_ability' in test:
        if 'source_ability' not in result:
            print(f"  ❌ Expected source_ability, but not found")
            test_passed = False
    if 'expected_has_except_quoted' in test:
        if 'except_quoted' not in result:
            print(f"  ❌ Expected except_quoted, but not found")
            test_passed = False
    if 'expected_has_except_abilities' in test:
        if 'except_abilities' not in result:
            print(f"  ❌ Expected except_abilities, but not found")
            test_passed = False
    
    if test_passed:
        print(f"  ✅ PASSED")
        passed += 1
    else:
        print(f"  ❌ FAILED")
        failed += 1

print("\n" + "=" * 80)
print(f"Results: {passed} passed, {failed} failed")

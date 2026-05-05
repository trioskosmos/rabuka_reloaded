"""Debug the parser to find where do_nothing comes from."""
import sys
import json
sys.path.insert(0, '.')
from parser import parse_effect, parse_ability, _EFFECT_HANDLERS, _try_duration_prefix, _try_furthermore, _try_implicit_sequential, _try_conditional

# Monkey-patch parse_effect to add debug
original_parse_effect = parse_effect

def debug_parse_effect(text, depth=0):
    indent = "  " * depth
    # Check if this text would produce a do_nothing
    result = original_parse_effect(text)
    if isinstance(result, dict):
        if result.get('action') == 'do_nothing' or (result.get('actions') and any(a.get('action') == 'do_nothing' for a in result.get('actions', []))):
            print(f"{indent}FOUND do_nothing in: '{text[:60]}'")
            print(f"{indent}Result: {json.dumps(result, indent=2, ensure_ascii=False)[:200]}")
    # Recursive check for sequential
    if isinstance(result, dict) and result.get('actions'):
        for i, a in enumerate(result.get('actions', [])):
            if isinstance(a, dict) and a.get('text') and a.get('text') != text:
                pass  # skip recursive for now
    return result

import parser
parser.parse_effect = debug_parse_effect

# Test the full triggerless text for branch 2 specifically
text2 = "2人以上いる場合、自分のステージにいる'μ's'のメンバー1人は、ライブ終了まで、{{heart_03.png|heart03}}を得る"
print("=== Testing with condition ===")
result = parse_effect(text2)
print("\nFinal result:")
print(json.dumps(result, indent=2, ensure_ascii=False)[:500])

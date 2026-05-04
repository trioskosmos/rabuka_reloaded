"""Debug _try_implicit_sequential on branch 2 text."""
import sys
import json
sys.path.insert(0, '.')
import parser

# Monkey-patch _try_implicit_sequential to add debug
orig = parser._try_implicit_sequential
def debug_try_implicit_sequential(text):
    result = orig(text)
    if result and text.startswith("自分のステージにいる"):
        print("=== _try_implicit_sequential called with ===")
        print(f"  text: {text[:80]}")
        if result.get('actions'):
            for i, a in enumerate(result['actions']):
                print(f"  action[{i}]: {a.get('action')}, text='{a.get('text','')[:40]}'")
    return result
parser._try_implicit_sequential = debug_try_implicit_sequential

# Also monkey-patch the parse_action within parser to debug
orig_parse_action = parser.parse_action
def debug_parse_action(text):
    result = orig_parse_action(text)
    if text == "ライブ終了まで":
        print(f"=== parse_action for 'ライブ終了まで' ===")
        print(f"  result: {json.dumps(result, indent=2, ensure_ascii=False)}")
    return result
parser.parse_action = debug_parse_action

# Run process_abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

result = parser.process_abilities(data)
entry = result['unique_abilities'][523]
effect = entry.get('effect', {})

# Check branch 2
actions = effect.get('actions', [])
if len(actions) >= 2:
    branch2 = actions[1]
    print("\n=== Branch 2 final ===")
    print(json.dumps(branch2, indent=2, ensure_ascii=False))

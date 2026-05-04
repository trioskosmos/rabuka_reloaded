"""Debug all effect handlers on branch 2 text."""
import sys
import json
sys.path.insert(0, '.')
import parser
from parser import _EFFECT_HANDLERS

# The exact text that goes into _try_furthermore's parse_effect for branch 2
text = "2人以上いる場合、自分のステージにいる'μ's'のメンバー1人は、ライブ終了まで、{{heart_03.png|heart03}}を得る"

print(f"Testing text: {text[:80]}...")
print()

for i, handler in enumerate(_EFFECT_HANDLERS):
    try:
        result = handler(text)
        if result is not None:
            print(f"Handler #{i} ({handler.__name__}) MATCHED!")
            print(f"  {json.dumps(result, indent=2, ensure_ascii=False)[:300]}")
            break
    except Exception as e:
        print(f"Handler #{i} ({handler.__name__}) error: {e}")
else:
    print("No handler matched!")
    
    # Fallback
    from parser import parse_action
    action = parse_action(text)
    print(f"parse_action result: {json.dumps(action, indent=2, ensure_ascii=False)[:200]}")

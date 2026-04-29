"""Test script for DOLLCHESTRA ability parsing."""
import sys
sys.path.insert(0, 'c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\ability_extraction')

from parser import parse_ability

# Test the DOLLCHESTRA ability with cost comparison
test_text = "このメンバーよりコストが低い『DOLLCHESTRA』のメンバーからバトンタッチして登場した場合、ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。"

print("Testing DOLLCHESTRA ability parsing...")
print(f"Input text: {test_text}")
print("\n" + "="*80 + "\n")

result = parse_ability(test_text)

import json
print("Parsed result:")
print(json.dumps(result, indent=2, ensure_ascii=False))

print("\n" + "="*80 + "\n")
print("Key checks:")
print(f"- Cost parsed: {'cost' in result}")
print(f"- Effect parsed: {'effect' in result}")
if 'effect' in result:
    effect = result['effect']
    print(f"- Condition parsed: {'condition' in effect}")
    print(f"- Action parsed: {'action' in effect}")
    if 'condition' in effect:
        cond = effect['condition']
        print(f"- Condition type: {cond.get('type', 'N/A')}")
        print(f"- Has group: {'group' in cond}")
        print(f"- Has comparison_type: {'comparison_type' in cond}")
        print(f"- Has operator: {'operator' in cond}")
        print(f"- Has baton_touch_trigger: {'baton_touch_trigger' in cond}")
        print(f"- Has baton_touch_group: {'baton_touch_group' in cond}")
        
        # Check for cost comparison details
        print(f"\nCondition details:")
        print(f"- Location: {cond.get('location', 'N/A')}")
        print(f"- Target: {cond.get('target', 'N/A')}")
        print(f"- Comparison type: {cond.get('comparison_type', 'N/A')}")
        print(f"- Operator: {cond.get('operator', 'N/A')}")
        if 'group' in cond:
            print(f"- Group name: {cond['group'].get('name', 'N/A')}")
    
    if 'action' in effect:
        action = effect['action']
        print(f"\nAction details:")
        if isinstance(action, dict):
            print(f"- Action type: {action.get('action', 'N/A')}")
            print(f"- Has duration: {'duration' in action}")
            print(f"- Duration: {action.get('duration', 'N/A')}")
        else:
            print(f"- Action type: {action}")
            print(f"- Has duration: {'duration' in effect}")
            print(f"- Duration: {effect.get('duration', 'N/A')}")
            print(f"- Count: {effect.get('count', 'N/A')}")

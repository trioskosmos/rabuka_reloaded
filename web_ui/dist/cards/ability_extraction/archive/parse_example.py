"""Parse example: {{toujyou.png|登場}}自分の成功ライブカード置き場にカードが2枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。"""

text = "{{toujyou.png|登場}}自分の成功ライブカード置き場にカードが2枚以上ある場合、自分の控え室からライブカードを1枚手札に加える。"

print("=== PARSING EXAMPLE ===")
print(f"Original text: {text}\n")

# Step 1: Remove trigger
triggerless = text.replace('{{toujyou.png|登場}}', '').strip()
print(f"Step 1 - Remove trigger: {triggerless}\n")

# Step 2: Identify structure (condition + action)
if '場合' in triggerless:
    condition_part, action_part = triggerless.split('場合', 1)
    condition_part = condition_part.strip()
    action_part = action_part.strip().rstrip('。')
    print(f"Step 2 - Split structure:")
    print(f"  Condition: {condition_part}")
    print(f"  Action: {action_part}\n")

# Step 3: Parse condition
print("Step 3 - Parse condition:")
condition = {
    'text': condition_part + '場合',
}

# Extract target
if '自分の' in condition_part:
    condition['target'] = 'self'
    print(f"  target: self")

# Extract location
if '成功ライブカード置き場' in condition_part:
    condition['location'] = 'success_live_card_zone'
    print(f"  location: success_live_card_zone")

# Extract object
if 'カード' in condition_part:
    condition['object'] = 'card'
    print(f"  object: card")

# Extract count and operator
import re
count_match = re.search(r'(\d+)枚以上', condition_part)
if count_match:
    condition['count'] = int(count_match.group(1))
    condition['operator'] = '>='
    print(f"  count: {condition['count']}")
    print(f"  operator: >=")

condition['type'] = 'card_count_at_least'
print(f"  type: card_count_at_least\n")

# Step 4: Parse action
print("Step 4 - Parse action:")
action = {
    'text': action_part,
}

# Extract target
if '自分の' in action_part:
    action['target'] = 'self'
    print(f"  target: self")

# Extract source
if '控え室から' in action_part:
    action['source'] = 'discard'
    print(f"  source: discard")

# Extract card type
if 'ライブカード' in action_part:
    action['card_type'] = 'live_card'
    print(f"  card_type: live_card")

# Extract count
count_match = re.search(r'(\d+)枚', action_part)
if count_match:
    action['count'] = int(count_match.group(1))
    print(f"  count: {action['count']}")

# Extract destination
if '手札に加える' in action_part:
    action['destination'] = 'hand'
    print(f"  destination: hand")

action['action'] = 'move_cards'
print(f"  action: move_cards\n")

# Step 5: Assemble final structure
print("Step 5 - Final structure:")
result = {
    'trigger': 'deploy',
    'condition': condition,
    'effect': action,
}

import json
print(json.dumps(result, indent=2, ensure_ascii=False))

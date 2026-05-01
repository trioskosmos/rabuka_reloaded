"""Analyze what custom and null actions should actually be"""
import json

with open(r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== ABILITIES WITH 'custom' ACTION ===\n")
custom_count = 0
for idx, ability in enumerate(abilities):
    effect = ability.get('effect', {})
    if isinstance(effect, dict) and effect.get('action') == 'custom':
        custom_count += 1
        text = effect.get('text', 'N/A')[:80]
        print(f"Ability {idx}:")
        print(f"  Text: {text}...")
        # Check other fields to understand what it should be
        if 'per_unit' in effect:
            print(f"  -> Has per_unit=true, should be 'modify_cost' or 'modify_required_hearts'")
        if 'operation' in effect:
            print(f"  -> Has operation={effect['operation']}, likely 'modify_required_hearts'")
        if 'state_change' in effect:
            print(f"  -> Has state_change={effect['state_change']}, should be 'change_state'")
        if 'optional' in effect:
            print(f"  -> Has optional={effect['optional']}")
        print()
        if custom_count >= 15:
            print(f"... and more custom actions")
            break

print(f"\n=== ABILITIES WITH 'null' ACTION ===\n")
null_count = 0
for idx, ability in enumerate(abilities):
    effect = ability.get('effect', {})
    if isinstance(effect, dict) and effect.get('action') == 'null':
        null_count += 1
        text = ability.get('full_text', 'N/A')[:80]
        print(f"Ability {idx}:")
        print(f"  Full text: {text}...")
        print(f"  -> This is a passive/replacement effect, should be 'restriction' or 'replacement_effect'")
        print()

print(f"\n=== SUMMARY ===")
print(f"Custom actions: {sum(1 for a in abilities if a.get('effect', {}).get('action') == 'custom')}")
print(f"Null actions: {sum(1 for a in abilities if a.get('effect', {}).get('action') == 'null')}")

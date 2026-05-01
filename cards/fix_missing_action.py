"""Find and fix ability 277 missing action"""
import json

abilities_path = r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json'

with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# Find ability 277
ability = abilities[277]
print(f"Ability 277:")
print(f"  full_text: {ability.get('full_text', '')[:80]}...")
print(f"  effect: {json.dumps(ability.get('effect', {}), ensure_ascii=False, indent=2)[:500]}")

# Check what's missing
effect = ability.get('effect', {})
if not effect.get('action'):
    print("\n  -> MISSING action field!")
    # Try to infer action from text
    text = effect.get('text', '')
    if '得る' in text and 'ハート' in text:
        effect['action'] = 'gain_resource'
        effect['resource'] = 'heart'
        print("  -> Fixed: Set action to 'gain_resource'")
    elif '引く' in text or '引き' in text:
        effect['action'] = 'draw_card'
        print("  -> Fixed: Set action to 'draw_card'")
    elif '見る' in text:
        effect['action'] = 'look'
        print("  -> Fixed: Set action to 'look'")
    else:
        effect['action'] = 'custom'
        print("  -> Fixed: Set action to 'custom'")

# Save
with open(abilities_path, 'w', encoding='utf-8') as f:
    json.dump(data, f, ensure_ascii=False, indent=2)

print("\nFixed ability 277 and saved.")

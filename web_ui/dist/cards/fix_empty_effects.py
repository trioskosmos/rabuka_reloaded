"""Find and fix abilities with empty effects"""
import json

abilities_path = r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json'

with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

empty_effect_abilities = []

for idx, ability in enumerate(abilities):
    if not ability.get('effect'):
        empty_effect_abilities.append((idx, ability))
        print(f"Ability {idx}: EMPTY EFFECT")
        print(f"  full_text: {ability.get('full_text', 'N/A')[:80]}...")
        print(f"  triggerless_text: {ability.get('triggerless_text', 'N/A')[:80]}...")
        print()

print(f"\nFound {len(empty_effect_abilities)} abilities with empty effects")

# Check what these abilities should have
for idx, ability in empty_effect_abilities:
    text = ability.get('full_text', '')
    print(f"\nAbility {idx}:")
    print(f"  Text: {text}")
    
    # Try to infer what the effect should be
    if 'null' in text.lower() or ability.get('is_null'):
        print("  -> This is a null ability, setting effect to null type")
        ability['effect'] = {'action': 'null', 'type': 'null'}
    elif 'ハート' in text or '得る' in text:
        print("  -> Should gain resource")
        ability['effect'] = {'action': 'gain_resource', 'text': text}
    elif '見る' in text or 'look' in text.lower():
        print("  -> Should look at cards")
        ability['effect'] = {'action': 'look', 'text': text}
    else:
        print("  -> Setting to custom action")
        ability['effect'] = {'action': 'custom', 'text': text}

# Save if we made changes
if empty_effect_abilities:
    with open(abilities_path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print(f"\nFixed {len(empty_effect_abilities)} abilities and saved.")
else:
    print("\nNo empty effects to fix.")

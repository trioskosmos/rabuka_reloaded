"""Compare long abilities to their parsed output to identify missing patterns."""
import json

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

# Find long abilities (by character count)
long_abilities = [a for a in abilities if len(a.get('triggerless_text', '')) > 150]
long_abilities.sort(key=lambda x: len(x.get('triggerless_text', '')), reverse=True)

print(f"=== LONG ABILITIES ANALYSIS ({len(long_abilities)} abilities > 150 chars) ===\n")

for i, ability in enumerate(long_abilities[:20], 1):
    text = ability.get('triggerless_text', '')
    cost = ability.get('cost')
    effect = ability.get('effect')
    
    print(f"--- Ability {i} ({len(text)} chars) ---")
    print(f"Text: {text}")
    print()
    
    if cost:
        print(f"Cost type: {cost.get('type', 'N/A')}")
        if cost.get('type') == 'custom':
            print(f"  Cost is CUSTOM - needs pattern")
        print(f"  Cost: {cost}")
    else:
        print("No cost")
    
    print()
    
    if effect:
        print(f"Effect action: {effect.get('action', 'N/A')}")
        if effect.get('action') == 'custom':
            print(f"  Effect is CUSTOM - needs pattern")
        if effect.get('condition'):
            print(f"  Condition type: {effect['condition'].get('type', 'N/A')}")
            if effect['condition'].get('type') == 'custom':
                print(f"    Condition is CUSTOM - needs pattern")
        print(f"  Effect: {effect}")
    else:
        print("No effect")
    
    print()
    print("=" * 80)
    print()

print(f"\n=== SUMMARY ===")
print(f"Total abilities analyzed: {len(long_abilities[:20])}")
custom_costs = sum(1 for a in long_abilities[:20] if a.get('cost') and a.get('cost').get('type') == 'custom')
custom_effects = sum(1 for a in long_abilities[:20] if a.get('effect') and a.get('effect').get('action') == 'custom')
print(f"Custom costs: {custom_costs}")
print(f"Custom effects: {custom_effects}")

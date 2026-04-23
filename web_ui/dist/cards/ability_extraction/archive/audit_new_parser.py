"""Audit the new parser output to find missing patterns."""
import json

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== AUDITING NEW PARSER OUTPUT ===\n")

# Check for missing cost
missing_cost = [a for a in abilities if '：' in a['triggerless_text'] and not a.get('cost')]
print(f"Abilities with cost marker but no parsed cost: {len(missing_cost)}")
if missing_cost:
    print("Samples:")
    for a in missing_cost[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for missing effect
missing_effect = [a for a in abilities if a['triggerless_text'] and not a.get('effect')]
print(f"\nAbilities with text but no parsed effect: {len(missing_effect)}")
if missing_effect:
    print("Samples:")
    for a in missing_effect[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for custom actions
custom_actions = [a for a in abilities if a.get('effect', {}).get('action') == 'custom']
print(f"\nAbilities with custom action (unparsed): {len(custom_actions)}")
if custom_actions:
    print("Samples:")
    for a in custom_actions[:10]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for custom cost types
custom_costs = [a for a in abilities if a.get('cost') and a.get('cost', {}).get('type') == 'custom']
print(f"\nAbilities with custom cost type: {len(custom_costs)}")
if custom_costs:
    print("Samples:")
    for a in custom_costs[:10]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for conditions
with_condition = [a for a in abilities if a.get('effect') and a.get('effect', {}).get('condition')]
print(f"\nAbilities with parsed conditions: {len(with_condition)}")

# Check for compound conditions (かつ)
compound_abilities = [a for a in abilities if 'かつ' in a['triggerless_text']]
print(f"\nAbilities with compound marker (かつ): {len(compound_abilities)}")
compound_parsed = [a for a in compound_abilities if a.get('effect') and a.get('effect', {}).get('condition', {}).get('type') == 'compound']
print(f"  Parsed as compound: {len(compound_parsed)}")

# Check for sequential (その後)
sequential_abilities = [a for a in abilities if 'その後' in a['triggerless_text']]
print(f"\nAbilities with sequential marker (その後): {len(sequential_abilities)}")
sequential_parsed = [a for a in sequential_abilities if a.get('effect') and a.get('effect', {}).get('action') == 'sequential']
print(f"  Parsed as sequential: {len(sequential_parsed)}")

# Check for choice (以下から1つを選ぶ)
choice_abilities = [a for a in abilities if '以下から1つを選ぶ' in a['triggerless_text']]
print(f"\nAbilities with choice marker: {len(choice_abilities)}")
choice_parsed = [a for a in choice_abilities if a.get('effect') and a.get('effect', {}).get('action') == 'choice']
print(f"  Parsed as choice: {len(choice_parsed)}")

# Check for duration (かぎり)
duration_abilities = [a for a in abilities if 'かぎり' in a['triggerless_text']]
print(f"\nAbilities with duration marker (かぎり): {len(duration_abilities)}")
duration_parsed = [a for a in duration_abilities if a.get('effect') and a.get('effect', {}).get('duration')]
print(f"  Parsed with duration: {len(duration_parsed)}")

# Check for state changes (ウェイト)
wait_abilities = [a for a in abilities if 'ウェイト' in a['triggerless_text']]
print(f"\nAbilities with wait marker (ウェイト): {len(wait_abilities)}")
wait_parsed = [a for a in wait_abilities if (a.get('effect') and a.get('effect', {}).get('state_change')) or (a.get('cost') and a.get('cost', {}).get('state_change'))]
print(f"  Parsed with state_change: {len(wait_parsed)}")

# Check for specific patterns not handled
print("\n=== CHECKING SPECIFIC PATTERNS ===")

# Check for "～につき" (per unit)
per_unit = [a for a in abilities if 'につき' in a['triggerless_text']]
print(f"Abilities with per-unit marker (につき): {len(per_unit)}")
if per_unit:
    print("Samples:")
    for a in per_unit[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for "いずれかの場合" (either case)
either_case = [a for a in abilities if 'いずれかの場合' in a['triggerless_text']]
print(f"\nAbilities with either-case marker (いずれかの場合): {len(either_case)}")
if either_case:
    print("Samples:")
    for a in either_case[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for "～以外" (except)
except_pattern = [a for a in abilities if '以外' in a['triggerless_text']]
print(f"\nAbilities with except marker (以外): {len(except_pattern)}")
if except_pattern:
    print("Samples:")
    for a in except_pattern[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for "～ごと" (per)
per_pattern = [a for a in abilities if 'ごと' in a['triggerless_text']]
print(f"\nAbilities with per marker (ごと): {len(per_pattern)}")
if per_pattern:
    print("Samples:")
    for a in per_pattern[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for position requirements
position_abilities = [a for a in abilities if any(x in a['triggerless_text'] for x in ['センターエリア', '左サイド', '右サイド'])]
print(f"\nAbilities with position requirement: {len(position_abilities)}")
position_parsed = [a for a in position_abilities if (a.get('effect') and a.get('effect', {}).get('condition', {}).get('position')) or (a.get('cost') and a.get('cost', {}).get('position'))]
print(f"  Parsed with position: {len(position_parsed)}")

# Check for "～ごとに" (per each)
per_each = [a for a in abilities if 'ごとに' in a['triggerless_text']]
print(f"\nAbilities with per-each marker (ごとに): {len(per_each)}")
if per_each:
    print("Samples:")
    for a in per_each[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for "～たび" (each time)
each_time = [a for a in abilities if 'たび' in a['triggerless_text']]
print(f"\nAbilities with each-time marker (たび): {len(each_time)}")
if each_time:
    print("Samples:")
    for a in each_time[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for "～たびに" (each time)
each_time_with = [a for a in abilities if 'たびに' in a['triggerless_text']]
print(f"\nAbilities with each-time marker (たびに): {len(each_time_with)}")
if each_time_with:
    print("Samples:")
    for a in each_time_with[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for "～枚ぶん" (for N cards worth)
for_cards = [a for a in abilities if '枚ぶん' in a['triggerless_text']]
print(f"\nAbilities with for-cards marker (枚ぶん): {len(for_cards)}")
if for_cards:
    print("Samples:")
    for a in for_cards[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

# Check for "～枚分" (for N cards worth)
for_cards_alt = [a for a in abilities if '枚分' in a['triggerless_text']]
print(f"\nAbilities with for-cards marker (枚分): {len(for_cards_alt)}")
if for_cards_alt:
    print("Samples:")
    for a in for_cards_alt[:5]:
        print(f"  {a['triggerless_text'][:100]}...")

print("\n=== SUMMARY ===")
print(f"Total abilities: {len(abilities)}")
print(f"Custom actions (unparsed): {len(custom_actions)}")
print(f"Custom costs (unparsed): {len(custom_costs)}")
print(f"Missing effects: {len(missing_effect)}")

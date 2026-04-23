"""Analyze punctuation usage in ability texts for names."""
import json
import re

# Load abilities
with open('../abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']

print("=== PUNCTUATION ANALYSIS ===\n")

# Count 『』 usage (group names)
group_bracket_count = 0
group_bracket_examples = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if '『' in text and '』' in text:
        group_bracket_count += 1
        # Extract all group names
        groups = re.findall(r'『([^』]+)』', text)
        if groups:
            group_bracket_examples.extend(groups[:2])  # Limit examples

print(f"Abilities with 『』 (group names): {group_bracket_count}")
print(f"Unique group names found: {len(set(group_bracket_examples))}")
print(f"Sample groups: {list(set(group_bracket_examples))[:10]}")

# Count 「」 usage (ability names, quotes)
quote_bracket_count = 0
quote_bracket_examples = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if '「' in text and '」' in text:
        quote_bracket_count += 1
        # Extract all quoted text
        quotes = re.findall(r'「([^」]+)」', text)
        if quotes:
            quote_bracket_examples.extend(quotes[:2])

print(f"\nAbilities with 「」 (quotes/ability names): {quote_bracket_count}")
print(f"Unique quoted text found: {len(set(quote_bracket_examples))}")
print(f"Sample quotes: {list(set(quote_bracket_examples))[:10]}")

# Count （） usage (parentheses)
paren_count = 0
paren_examples = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if '（' in text and '）' in text:
        paren_count += 1
        # Extract all parenthetical text
        parens = re.findall(r'（([^）]+)）', text)
        if parens:
            paren_examples.extend(parens[:2])

print(f"\nAbilities with （） (parentheses): {paren_count}")
print(f"Sample parenthetical: {paren_examples[:10]}")

# Check for patterns where punctuation affects parsing
print("\n=== PUNCTUATION INTERFERENCE PATTERNS ===\n")

# Check: 「」 in conditions
quote_in_condition = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if '「' in text and ('場合' in text or 'とき' in text):
        quote_in_condition.append(text[:100])

print(f"Abilities with quotes in conditions: {len(quote_in_condition)}")
if quote_in_condition:
    print("Sample:")
    for i, text in enumerate(quote_in_condition[:3], 1):
        print(f"  {i}. {text}...")

# Check: 『』 in conditions
group_in_condition = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if '『' in text and ('場合' in text or 'とき' in text):
        group_in_condition.append(text[:100])

print(f"\nAbilities with group brackets in conditions: {len(group_in_condition)}")
if group_in_condition:
    print("Sample:")
    for i, text in enumerate(group_in_condition[:3], 1):
        print(f"  {i}. {text}...")

# Check: quotes in cost
quote_in_cost = []
for ability in abilities:
    text = ability.get('triggerless_text', '')
    if '：' in text and '「' in text:
        parts = text.split('：', 1)
        if '「' in parts[0]:
            quote_in_cost.append(parts[0][:100])

print(f"\nAbilities with quotes in cost: {len(quote_in_cost)}")
if quote_in_cost:
    print("Sample:")
    for i, text in enumerate(quote_in_cost[:3], 1):
        print(f"  {i}. {text}...")

print("\n=== RECOMMENDATIONS ===")
print("1. 『』 marks group names - should be extracted as group field")
print("2. 「」 marks ability names or quoted text - should be extracted as ability_name field")
print("3. （） marks parenthetical notes - should be handled separately or as modifier")
print("4. Punctuation should be treated as word boundaries for parsing")
print("5. When splitting by punctuation, preserve the quoted/bracketed content")

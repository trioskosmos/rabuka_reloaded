"""Extract and analyze all text inside 「」brackets from abilities.json"""
import json
import re
from collections import defaultdict

# Load abilities.json
import os
script_dir = os.path.dirname(os.path.abspath(__file__))
abilities_path = os.path.join(script_dir, '../abilities.json')
with open(abilities_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

# Extract all quoted text from abilities
quoted_texts = defaultdict(list)
ability_texts = []

for ability in data['unique_abilities']:
    full_text = ability['full_text']
    triggerless_text = ability['triggerless_text']
    
    # Extract from both full_text and triggerless_text
    for text in [full_text, triggerless_text]:
        # Find all 「」brackets
        matches = re.findall(r'「([^」]+)」', text)
        for match in matches:
            quoted_texts[match].append({
                'full_text': full_text,
                'triggerless_text': triggerless_text,
                'cards': ability['cards']
            })
        ability_texts.append(text)

print(f"Total abilities: {len(data['unique_abilities'])}")
print(f"Total unique quoted strings: {len(quoted_texts)}")
print(f"\n" + "="*80)
print("ALL UNIQUE QUOTED STRINGS:")
print("="*80)

# Sort and display all unique quoted strings
for i, quote in enumerate(sorted(quoted_texts.keys()), 1):
    count = len(quoted_texts[quote])
    print(f"{i}. 「{quote}」 (appears {count} times)")

print(f"\n" + "="*80)
print("CATEGORIZATION:")
print("="*80)

# Common ability-related keywords
ability_keywords = ['能力', '効果', 'テキスト', 'アクション', '効果を得る', 'を持つ', 'と同じ']

character_names = []
ability_references = []
uncertain = []

for quote in sorted(quoted_texts.keys()):
    # Check if it looks like an ability reference
    is_ability = any(kw in quote for kw in ability_keywords)
    
    # Check if it looks like a character name (typically Japanese names)
    # Character names usually don't contain ability-related keywords
    if is_ability:
        ability_references.append(quote)
    elif len(quote) <= 20 and '能力' not in quote and '効果' not in quote:
        # Likely a character name
        character_names.append(quote)
    else:
        uncertain.append(quote)

print(f"\n--- CHARACTER NAMES ({len(character_names)}) ---")
for i, name in enumerate(character_names, 1):
    print(f"{i}. 「{name}」")

print(f"\n--- ABILITY REFERENCES ({len(ability_references)}) ---")
for i, ref in enumerate(ability_references, 1):
    print(f"{i}. 「{ref}」")

print(f"\n--- UNCERTAIN ({len(uncertain)}) ---")
for i, ref in enumerate(uncertain, 1):
    print(f"{i}. 「{ref}」")

print(f"\n" + "="*80)
print("DETAILED EXAMPLES:")
print("="*80)

# Show examples for each category with context
if character_names:
    print("\n--- CHARACTER NAME EXAMPLES ---")
    sample_name = character_names[0]
    print(f"\nQuote: 「{sample_name}」")
    for example in quoted_texts[sample_name][:3]:  # Show first 3 examples
        print(f"  - {example['triggerless_text'][:100]}...")

if ability_references:
    print("\n--- ABILITY REFERENCE EXAMPLES ---")
    sample_ref = ability_references[0]
    print(f"\nQuote: 「{sample_ref}」")
    for example in quoted_texts[sample_ref][:3]:  # Show first 3 examples
        print(f"  - {example['triggerless_text'][:100]}...")

if uncertain:
    print("\n--- UNCERTAIN EXAMPLES ---")
    sample_ref = uncertain[0]
    print(f"\nQuote: 「{sample_ref}」")
    for example in quoted_texts[sample_ref][:3]:  # Show first 3 examples
        print(f"  - {example['triggerless_text'][:100]}...")

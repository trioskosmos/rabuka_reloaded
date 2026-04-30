#!/usr/bin/env python3

# Read the file
with open('ability_resolver.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix broken PendingAbilityExecution assignments by removing orphaned field assignments
import re

# Pattern to match the broken assignments after the commented line
pattern = r'// self\.game_state\.pending_ability = Some\(crate::game_state::PendingAbilityExecution \{ \(REMOVED\)\s+card_no: "[^"]+",.*?\}\);'

# Replace with just a comment
content = re.sub(pattern, '// Queue system handles pending state', content, flags=re.DOTALL)

# Also fix the missing fields in AbilityEffect default
content = re.sub(r'AbilityEffect \{', 'AbilityEffect {\n            cost_comparison: None,\n            total_cost_limit: None,', content)

# Write the file back
with open('ability_resolver.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed broken PendingAbilityExecution assignments")

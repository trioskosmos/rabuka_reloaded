#!/usr/bin/env python3

# Read the file
with open('ability_resolver.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Simple string replacements to comment out problematic lines
replacements = [
    ('self.game_state.pending_ability = None;', '// self.game_state.pending_ability = None; (REMOVED)'),
    ('self.game_state.pending_choice = None;', '// self.game_state.pending_choice = None; (REMOVED)'),
]

# Apply replacements
for old, new in replacements:
    content = content.replace(old, new)

# Write the file back
with open('ability_resolver.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Applied simple replacements to ability_resolver.rs")

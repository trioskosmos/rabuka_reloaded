#!/usr/bin/env python3

# Read the file
with open('ability_resolver.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix remaining issues
replacements = [
    # Remove pending_current_assignment and pending_choice assignments
    ('self.game_state.pending_current_ability = self.current_ability.clone();', '// self.game_state.pending_current_ability = self.current_ability.clone(); (REMOVED)'),
    ('self.game_state.pending_choice = self.pending_choice.clone();', '// self.game_state.pending_choice = self.pending_choice.clone(); (REMOVED)'),
    
    # Comment out all pending_ability assignments
    ('self.game_state.pending_ability = Some(', '// self.game_state.pending_ability = Some('),
    
    # Comment out pending_ability debug prints
    ('self.game_state.pending_ability', '// self.game_state.pending_ability (REMOVED)'),
    
    # Fix missing fields in AbilityEffect default
    ('AbilityEffect {', 'AbilityEffect {\n            cost_comparison: None,\n            total_cost_limit: None,'),
]

# Apply replacements
for old, new in replacements:
    content = content.replace(old, new)

# Write the file back
with open('ability_resolver.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Applied final fixes to ability_resolver.rs")

#!/usr/bin/env python3
import re

# Read the file
with open('ability_resolver.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# Process lines to remove problematic pending_* references
fixed_lines = []
skip_mode = False
brace_count = 0

for i, line in enumerate(lines):
    # Skip lines that assign to pending_ability
    if 'self.game_state.pending_ability = Some(' in line:
        skip_mode = True
        brace_count = 0
        continue
    
    # Skip lines that assign None to pending fields
    if 'self.game_state.pending_ability = None;' in line or 'self.game_state.pending_choice = None;' in line:
        continue
    
    # Handle multi-line PendingAbilityExecution assignments
    if skip_mode:
        if '{' in line:
            brace_count += line.count('{')
        if '}' in line:
            brace_count -= line.count('}')
            if brace_count <= 0:
                skip_mode = False
                # Add a comment instead
                fixed_lines.append('// Queue system handles pending state\n')
        continue
    
    # Replace if let statements for pending_ability
    if 'if let Some(ref pending) = self.game_state.pending_ability.clone()' in line:
        # Skip this if let block entirely
        skip_mode = True
        brace_count = 0
        continue
    
    # Replace character_filter line
    if 'let character_filter = self.game_state.pending_ability.as_ref()' in line:
        fixed_lines.append('let character_filter = None;\n')
        continue
    
    fixed_lines.append(line)

# Write the file back
with open('ability_resolver.rs', 'w', encoding='utf-8') as f:
    f.writelines(fixed_lines)

print("Fixed ability_resolver.rs - removed pending_* references")

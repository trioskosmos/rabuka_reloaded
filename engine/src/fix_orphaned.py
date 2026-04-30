#!/usr/bin/env python3

# Read the file
with open('ability_resolver.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix orphaned PendingAbilityExecution assignments by removing the entire block
import re

# Pattern to match commented pending_ability line followed by the struct fields
pattern = r'// self\.game_state\.pending_ability = Some\(crate::game_state::PendingAbilityExecution \{ \(REMOVED\)\s+card_no: "[^"]+",.*?\}\);'

# Replace with just a comment
content = re.sub(pattern, '// Queue system handles pending state', content, flags=re.DOTALL)

# Also fix debug prints that reference pending_ability
content = re.sub(r'eprintln!\("Stored pending_ability with effect: \{:?\}", self\.game_state\.pending_ability\);', 
                '// eprintln!("Stored pending_ability with effect: {:?}", self.game_state.pending_ability); (REMOVED)', content)

# Fix pending_current_ability references
content = re.sub(r'self\.game_state\.pending_current_ability = self\.current_ability\.clone\(\);', 
                '// self.game_state.pending_current_ability = self.current_ability.clone(); (REMOVED)', content)
content = re.sub(r'self\.game_state\.pending_current_ability = None;', 
                '// self.game_state.pending_current_ability = None; (REMOVED)', content)

# Write the file back
with open('ability_resolver.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed orphaned PendingAbilityExecution assignments")

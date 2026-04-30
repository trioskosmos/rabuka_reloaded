#!/usr/bin/env python3
import re

# Read the file
with open('ability_resolver.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace pending_ability assignments with comments
content = re.sub(r'self\.game_state\.pending_ability = Some\(crate::game_state::PendingAbilityExecution \{[^}]+\}\);', 
                '// Queue system handles pending state', content, flags=re.DOTALL)

# Replace pending_ability = None with comments
content = re.sub(r'self\.game_state\.pending_ability = None;', 
                '// self.game_state.pending_ability = None; (REMOVED)', content)

# Replace pending_choice = None with comments  
content = re.sub(r'self\.game_state\.pending_choice = None;', 
                '// self.game_state.pending_choice = None; (REMOVED)', content)

# Replace pending_ability checks with comments
content = re.sub(r'if let Some\(ref pending\) = self\.game_state\.pending_ability\.clone\(\) \{[^}]+\}', 
                '// Queue system handles pending state', content, flags=re.DOTALL)

# Replace character_filter check
content = re.sub(r'let character_filter = self\.game_state\.pending_ability\.as_ref\(\)[^}]+\};', 
                'let character_filter = None;', content, flags=re.DOTALL)

# Write the file back
with open('ability_resolver.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed ability_resolver.rs")

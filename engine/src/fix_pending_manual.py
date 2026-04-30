#!/usr/bin/env python3

# Read the file
with open('ability_resolver.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Simple string replacements that won't break syntax
replacements = [
    # Comment out pending_ability assignments completely (including the struct)
    ('self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {\n                            card_no: "optional_cost".to_string(),\n                            player_id: "self".to_string(),\n                            action_index: 0,\n                            effect: effect.clone(),\n                            conditional_choice: None,\n                            activating_card: None,\n                            ability_index: 0,\n                            cost: None,\n                            cost_choice: None,\n                        });', '// Queue system handles pending state'),
    
    # Comment out pending_choice assignments
    ('self.game_state.pending_choice = self.pending_choice.clone();', '// self.game_state.pending_choice = self.pending_choice.clone(); (REMOVED)'),
    
    # Comment out pending_current_ability assignments
    ('self.game_state.pending_current_ability = self.current_ability.clone();', '// self.game_state.pending_current_ability = self.current_ability.clone(); (REMOVED)'),
    ('self.game_state.pending_current_ability = None;', '// self.game_state.pending_current_ability = None; (REMOVED)'),
    
    # Comment out debug prints
    ('eprintln!("Stored pending_ability with effect: {:?}", self.game_state.pending_ability);', '// eprintln!("Stored pending_ability with effect: {:?}", self.game_state.pending_ability); (REMOVED)'),
]

# Apply replacements
for old, new in replacements:
    content = content.replace(old, new)

# Write the file back
with open('ability_resolver.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Applied manual fixes to ability_resolver.rs")

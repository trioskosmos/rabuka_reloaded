import os

path = 'src/core/game_state/mod.rs'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

lines = content.split('\n')

# Find key function boundaries (0-indexed)
key_funcs = {
    'pub fn can_play_turn1_ability': None,
    'pub fn add_blade_modifier': None,
    'pub fn trigger_auto_ability': None,
}

for i, line in enumerate(lines):
    for func in key_funcs:
        if func in line and key_funcs[func] is None:
            key_funcs[func] = i

print(f"Total lines: {len(lines)}")
print(f"can_play_turn1_ability: line {key_funcs['pub fn can_play_turn1_ability']+1}")
print(f"add_blade_modifier: line {key_funcs['pub fn add_blade_modifier']+1}")
print(f"trigger_auto_ability: line {key_funcs['pub fn trigger_auto_ability']+1}")

dir_path = os.path.dirname(path)
track_start = key_funcs['pub fn can_play_turn1_ability']
modifiers_start = key_funcs['pub fn add_blade_modifier']
abilities_start = key_funcs['pub fn trigger_auto_ability']

# 1. tracking.rs — from can_play_turn1_ability to before add_blade_modifier
track_lines = []
track_lines.append('impl GameState {')
track_lines.append('')
for i in range(track_start, modifiers_start):
    track_lines.append(lines[i])
# Find move_resolution_zone_to_waitroom and add it too
for i in range(modifiers_start, abilities_start):
    if 'move_resolution_zone_to_waitroom' in lines[i]:
        j = i
        while j < abilities_start:
            track_lines.append(lines[j])
            j += 1
        break
track_lines.append('}')

with open(os.path.join(dir_path, 'tracking.rs'), 'w', encoding='utf-8') as f:
    f.write('\n'.join(track_lines))
print(f"Wrote tracking.rs: {len(track_lines)} lines")

# 2. modifiers.rs — from add_blade_modifier to before trigger_auto_ability
mod_lines = []
mod_lines.append('impl GameState {')
mod_lines.append('')
for i in range(modifiers_start, abilities_start):
    mod_lines.append(lines[i])
mod_lines.append('}')

with open(os.path.join(dir_path, 'modifiers.rs'), 'w', encoding='utf-8') as f:
    f.write('\n'.join(mod_lines))
print(f"Wrote modifiers.rs: {len(mod_lines)} lines")

# 3. abilities.rs — from trigger_auto_ability to end
abil_lines = []
abil_lines.append('impl GameState {')
abil_lines.append('')
for i in range(abilities_start, len(lines)):
    abil_lines.append(lines[i])

with open(os.path.join(dir_path, 'abilities.rs'), 'w', encoding='utf-8') as f:
    f.write('\n'.join(abil_lines))
print(f"Wrote abilities.rs: {len(abil_lines)} lines")

# 4. Rewrite mod.rs — keep lines 0 to track_start-1, add include!
new_mod = lines[:track_start]
new_mod.append('')
new_mod.append('include!("tracking.rs");')
new_mod.append('include!("modifiers.rs");')
new_mod.append('include!("abilities.rs");')

with open(path, 'w', encoding='utf-8') as f:
    f.write('\n'.join(new_mod))
print(f"Rewrote mod.rs: {len(new_mod)} lines")
print("Done!")

import json
import sys

output_file = r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\scan_report.txt'

def log(msg):
    print(msg)
    with open(output_file, 'a', encoding='utf-8') as f:
        f.write(msg + '\n')

# Clear previous output
with open(output_file, 'w', encoding='utf-8') as f:
    f.write('')

with open(r'c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
log(f'Total abilities to scan: {len(abilities)}')
log('='*80)

issues = []
warnings = []
action_types = set()
condition_types = set()
source_values = set()
dest_values = set()
target_values = set()
card_types = set()

valid_actions = {'move_cards', 'draw_card', 'gain_resource', 'look_and_select', 
                'reveal', 'select', 'appear', 'choice', 'sequential', 'modify_score',
                'look', 'change_state', 'place_energy_under_member', 'set_card_identity',
                'custom', 'conditional_alternative', 'modify_yell_count', 'null',
                'draw', 'position_change', 'temporal', 'energy_active', 'look_at'}

def check_action(act, ability_idx, path='effect'):
    action = act.get('action', '')
    action_types.add(action)
    
    if not action:
        issues.append(f'Ability {ability_idx} {path}: MISSING action type')
    elif action not in valid_actions:
        issues.append(f'Ability {ability_idx} {path}: INVALID action "{action}"')
    
    # Check empty critical fields
    for field in ['source', 'destination', 'target']:
        val = act.get(field, None)
        if val == '':
            issues.append(f'Ability {ability_idx} {path}: EMPTY {field}')
    
    # Check count
    count = act.get('count')
    if count is None and 'dynamic_count' not in act:
        # Some actions don't need count (like look)
        if action in ['move_cards', 'draw_card', 'gain_resource', 'change_state']:
            issues.append(f'Ability {ability_idx} {path}: count is None but no dynamic_count')
    
    if isinstance(count, int):
        if count == 0:
            warnings.append(f'Ability {ability_idx} {path}: count=0 (might be intentional)')
        elif count < 0:
            issues.append(f'Ability {ability_idx} {path}: NEGATIVE count={count}')
        elif count > 20:
            warnings.append(f'Ability {ability_idx} {path}: very high count={count}')
    
    # Source == Destination
    src = act.get('source')
    dst = act.get('destination')
    if src and dst and src == dst:
        issues.append(f'Ability {ability_idx} {path}: source==destination ("{src}")')
    
    # Card type
    ct = act.get('card_type')
    if ct:
        card_types.add(ct)
    
    # Collect values
    if src: source_values.add(src)
    if dst: dest_values.add(dst)
    tgt = act.get('target')
    if tgt: target_values.add(tgt)
    
    # Check nested actions
    if act.get('actions'):
        for i, sub in enumerate(act['actions']):
            check_action(sub, ability_idx, f'{path}.actions[{i}]')
    
    # Check look_action / select_action
    if act.get('look_action'):
        check_action(act['look_action'], ability_idx, f'{path}.look_action')
    if act.get('select_action'):
        check_action(act['select_action'], ability_idx, f'{path}.select_action')

for idx, ability in enumerate(abilities):
    # Check effect
    if ability.get('effect'):
        check_action(ability['effect'], idx)
        
        # Check condition
        cond = ability['effect'].get('condition')
        if cond:
            ct = cond.get('type', '')
            condition_types.add(ct)
            if not ct:
                issues.append(f'Ability {idx}: condition missing type')
    
    # Check cost
    if ability.get('cost'):
        cost = ability['cost']
        if cost.get('type') == 'move_cards':
            if not cost.get('source'):
                issues.append(f'Ability {idx}: cost move_cards missing source')
            if not cost.get('destination'):
                issues.append(f'Ability {idx}: cost move_cards missing destination')

log(f'\n=== ISSUES FOUND: {len(issues)} ===')
for issue in issues[:50]:
    log(f'  [ERROR] {issue}')
if len(issues) > 50:
    log(f'  ... and {len(issues)-50} more errors')

log(f'\n=== WARNINGS: {len(warnings)} ===')
for w in warnings[:30]:
    log(f'  [WARN] {w}')
if len(warnings) > 30:
    log(f'  ... and {len(warnings)-30} more warnings')

log(f'\n=== ACTION TYPES ({len(action_types)}) ===')
for at in sorted(action_types):
    valid = 'OK' if at in valid_actions else 'INVALID'
    log(f'  [{valid}] {at}')

log(f'\n=== CONDITION TYPES ({len(condition_types)}) ===')
for ct in sorted(condition_types):
    log(f'  {ct}')

log(f'\n=== SOURCE VALUES ({len(source_values)}) ===')
for s in sorted(source_values):
    log(f'  {s}')

log(f'\n=== DESTINATION VALUES ({len(dest_values)}) ===')
for d in sorted(dest_values):
    log(f'  {d}')

log(f'\n=== CARD TYPES ({len(card_types)}) ===')
for ct in sorted(card_types):
    log(f'  {ct}')

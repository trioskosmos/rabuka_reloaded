import sys
sys.path.insert(0, 'cards/ability_extraction')

# Directly check what happens by injecting debug into the parser module
import importlib
spec = importlib.util.spec_from_file_location('parser', 'cards/ability_extraction/parser.py')
parser = importlib.util.module_from_spec(spec)

# Read and modify the source to add debug prints
with open('cards/ability_extraction/parser.py', encoding='utf-8') as f:
    source = f.read()

# Add debug print in the dispatch loop
old = "    if match:\n            action['action'] = act"
new = "    if match:\n            action['action'] = act\n            import sys; print(f'DISPATCH MATCH: {act} cond={cond}', file=sys.stderr)\n            if 'gain_ability' in str(act) or 'custom' in str(act) or 'set_required' in str(act) or 'modify_cost' in str(act): import traceback; traceback.print_stack(file=sys.stderr)"
source_debug = source.replace(old, new)

# Execute the modified source
exec(compile(source_debug, 'cards/ability_extraction/parser_debug.py', 'exec'), parser.__dict__)

text = 'このカードのコストを+4して{{heart_05.png|heart05}}を得る。この能力では下にあるメンバーカードは3枚までしか数えない。'
r = parser.parse_action(text)
print('FINAL action:', r.get('action'))
print('FINAL ability_gain:', repr(r.get('ability_gain','')))

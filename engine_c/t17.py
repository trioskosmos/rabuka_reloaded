import importlib.util, sys
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

# Load global helpers
import pathlib
hp = pathlib.Path('engine/tests/helpers/mod.rs')
gh = gt.collect_helpers(hp.read_text(encoding='utf-8', errors='ignore'), set())
print('global helper count:', len(gh))
print('fill_decks' in gh, 'give_energy' in gh, 'select_indices' in gh, 'select_option' in gh, 'drain_auto_ability_choices' in gh, 'pass' in gh, 'set_live_card' in gh)

# Test expand_helpers
body = '    fill_decks(&mut game, filler);\n    give_energy(&mut game, 5);\n    select_indices(&mut game, &[0]);\n    select_option(&mut game, 1);\n    pass(&mut game);\n    drain_auto_ability_choices(&mut game);\n'
consts = {'filler':'PL!-sd1-010-SD'}
exp = gt.expand_helpers(body, gh, consts)
print('=== EXPANDED ===')
print(exp)
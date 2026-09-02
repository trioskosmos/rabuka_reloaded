import importlib.util, sys
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)
for body in [
    '    test_add_to_energy(&tg, 0, g.id("LL-E-001-SD"));\n',
    '    test_add_to_deck_pl(&tg, 0, v.id("PL!-sd1-010-SD"));\n',
    '    test_add_to_discard(&tg, g.new_id(DIVE));\n',
]:
    r = gt.transpile_body(body, {}, 'f')
    sys.stderr.write('IN : '+repr(body)+'\nOUT: '+repr(r)+'\n')
import importlib.util, sys, io, contextlib
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

cases = [
    '''    game.select_indices(&[0]);
    game.select_indices(&[0, 1]);
    game.select_option(1);
    game.state.mods.add_heart_modifier(p2_member, HeartColor::Heart05, 4);
''',
]
for body in cases:
    buf = io.StringIO()
    with contextlib.redirect_stderr(buf):
        r = gt.transpile_body(body, {}, 'f')
    print('OUT:')
    print(r)
    print('---')
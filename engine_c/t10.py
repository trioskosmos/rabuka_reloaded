import importlib.util, sys
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

body = '''    game.state.player1.stage.stage = [
        shizuku, filler, filler,
    ];
    game.state.player1.hand.cards.push(hand_cost_filler);
'''
import io, contextlib
buf = io.StringIO()
with contextlib.redirect_stderr(buf):
    r = gt.transpile_body(body, {}, 'f')
print('OUT:')
print(r)
print('STDERR:', buf.getvalue()[:500])
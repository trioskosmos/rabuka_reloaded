import sys
sys.argv = ['x']
import importlib.util
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
# instead just exec the function source with a probe
src = open('tools/gen_tests.py', encoding='utf-8').read()
# extract the transpile_body function and add a probe
probe = "        if 'g.id(' in stripped:\n            import sys as _s; _s.stderr.write('PROBE HIT: '+repr(stripped)+'\\n')"
src = src.replace("        # Passthrough for engine calls already substituted into the body by the\n", probe + "        # Passthrough for engine calls already substituted into the body by the\n")
ns = {}
exec(compile(src, 'gen_tests.py', 'exec'), ns)
body = '    test_add_to_energy(&tg, 0, g.id("LL-E-001-SD"));\n'
print('RESULT:', repr(ns['transpile_body'](body, {}, 'f')))
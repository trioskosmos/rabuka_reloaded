import importlib.util, sys, types
# Load module
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

# Probe transpile_body
orig = gt.transpile_body
def patched(body, consts, fn, helpers=None):
    r = orig(body, consts, fn, helpers)
    if r and 'g.id(' in r:
        sys.stderr.write('TPROBE unrewritten g.id in output:\n')
        for l in r.split('\n'):
            if 'g.id(' in l or 'g.new_id(' in l or 'v.id(' in l:
                sys.stderr.write('  '+repr(l)+'\n')
    return r
gt.transpile_body = patched

# Now run main
sys.argv = ['gen_tests.py']
gt.main()
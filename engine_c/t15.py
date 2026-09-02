import importlib.util, sys, io, contextlib, re
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

# Monkeypatch _postprocess_generated_file to print pass-3 findings
orig = gt._postprocess_generated_file
def patched(path):
    # run the real one but capture pass-3 by re-implementing the scan on the
    # pre-pass-3 lines.  Easiest: just call orig and grep the output file.
    orig(path)
    src = open(str(path), encoding='utf-8', errors='replace').read().split('\n')
    for i,l in enumerate(src):
        if 'keke1' in l or 'mei =' in l:
            print(i+1, repr(l))
gt._postprocess_generated_file = patched
gt.main()
import importlib.util, sys, re
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

# Find card! macro definitions
src = open('../engine/src/ability/abilities_gen.rs', encoding='utf-8', errors='ignore').read()
consts = {}
for m in re.finditer(r'card!\s*\(\s*"([^"]+)"\s*\)\s*=>\s*(\w+)', src):
    consts[m.group(2)] = m.group(1)
for m in re.finditer(r'card!\s*\(\s*"([^"]+)"\s*\)\s*=>\s*"?([A-Z_0-9]+)"?', src):
    consts[m.group(2)] = m.group(1)
for k in ['DIVE','P1_MEMBER','P1_LIVE','TRAPPER','P2_MEMBER','MIRAKURA_RURINO','HIMEKO','KOKO','NON_MIRAKURA_MEMBER']:
    print(k, consts.get(k))
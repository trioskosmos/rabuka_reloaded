import importlib.util, sys, io, contextlib
spec = importlib.util.spec_from_file_location('gt', 'tools/gen_tests.py')
gt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gt)

# Simulate the pass-3 token scan on a body containing the keke1 pattern.
body = """static void gen_x(void){
    TestGame tg; test_game_new(&tg);
    // action result consumed: let keke1 = test_id(&tg, "PL!SP-bp4-013-N");
    test_add_to_hand(&tg, keke1);
    test_drain_auto_choices(&tg);
}
"""
import re
known = {"tg","tg2","test_add_to_hand","test_drain_auto_choices","test_id","test_game_new"}
keywords = {"int","void","char","if","for","while","return"}
real = re.sub(r'//.*', '', body)
real = re.sub(r'/\*.*?\*/', '', real, flags=re.DOTALL)
used = set(re.findall(r'[A-Za-z_]\w*', real))
print('used tokens:', sorted(used))
declared = set(re.findall(r'\bint\s+(\w+)\b', body))
declared.add('tg')
missing = set()
for tok in used:
    if tok in known or tok in declared or tok in keywords: continue
    if not re.match(r'^[A-Za-z_]\w*$', tok): continue
    if re.search(r'\b'+re.escape(tok)+r'\s*\(', real): continue
    if re.search(r'\.'+re.escape(tok)+r'\b', real): continue
    missing.add(tok)
print('missing:', sorted(missing))
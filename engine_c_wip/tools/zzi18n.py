"""Does every canonical choice-prompt template have a Japanese translation?

engine.c:1910 i18n_self_check() is a complete, faithful port of describe.rs:1053-1127
and it has ZERO callers. Rust calls it from main.rs:65, before the server starts.
src/engine.c:1904 explicitly hands the boot call to web_server.c's owner -- which is me.

Before wiring it I check it will be SILENT, so the change cannot add stderr noise.
"""
import re, os

CR = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip'
eng = open(os.path.join(CR, 'src', 'engine.c'), encoding='utf-8').read()
lines = eng.split('\n')

# the EN table
m = re.search(r's_choice_prompt_templates_en\[\]\s*=\s*\{(.*?)\n\};', eng, re.S)
if not m:
    print('EN template table NOT FOUND'); raise SystemExit(1)
body = m.group(1)
tpls = re.findall(r'"((?:[^"\\]|\\.)*)"', body)
print('canonical EN choice-prompt templates:', len(tpls))

# the JA table inside translate_choice_prompt_en_to_ja
m2 = re.search(r'translate_choice_prompt_en_to_ja\s*\([^)]*\)\s*\{(.*?)\n\}', eng, re.S)
ja_pairs = {}
if m2:
    for mm in re.finditer(r'strcmp\(en,\s*"((?:[^"\\]|\\.)*)"\)\s*\)\s*\{?\s*'
                         r'(?:return\s+)?(?:snprintf\([^;]*?"((?:[^"\\]|\\.)*)"|\w+\s*=\s*"((?:[^"\\]|\\.)*)")',
                         m2.group(1), re.S):
        k = mm.group(1)
        ja_pairs[k] = mm.group(2) if mm.group(2) is not None else mm.group(3)

print('JA translations found in the same function:', len(ja_pairs))
missing = [t for t in tpls if t not in ja_pairs]
print('\nTEMPLATES WITH NO JA TRANSLATION (i18n_self_check would print a warning):')
if missing:
    for t in missing:
        print('   ', repr(t))
else:
    print('    NONE -- the self-check would print only the all-clear line')

print('\ncallers of i18n_self_check in the whole C tree:')
import glob
n = 0
for p in glob.glob(os.path.join(CR, 'src', '**', '*.c'), recursive=True):
    s = open(p, encoding='utf-8', errors='replace').read()
    c = len(re.findall(r'\bi18n_self_check\s*\(', s))
    if c:
        print(f'   {os.path.relpath(p, CR)}: {c} occurrence(s)')
        n += c
print('   TOTAL:', n, '(1 = the definition only => never called)')
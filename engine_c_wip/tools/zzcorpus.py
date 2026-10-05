#!/usr/bin/env python
"""Corpus measurements for the web_server pending-choice path and the trace ring.

Counts, over the real decoded corpus (the same data the engine decodes):
  A. ability-text byte lengths -> what any fixed-cap writer truncates, and whether
     the cut lands inside a UTF-8 sequence (torn character).
  B. ability texts / card names containing bytes that break an UNESCAPED
     `"%s"` JSON embed: '"', '\\', or any C0 control (a raw newline is invalid JSON).
  C. card names carrying a literal newline.
"""
import json, glob, os, sys, collections

ROOT = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded'
cands = []
for pat in ('**/cards.json', '**/cards*.json'):
    cands += glob.glob(os.path.join(ROOT, 'engine_c_wip', pat), recursive=True)
    cands += glob.glob(os.path.join(ROOT, pat), recursive=True)
cands = sorted(set(cands))
print('cards.json candidates:', len(cands))
for c in cands[:12]:
    print('   ', os.path.relpath(c, ROOT), os.path.getsize(c))

def load(p):
    d = json.load(open(p, encoding='utf-8'))
    if isinstance(d, dict):
        for k in ('cards', 'data', 'items'):
            if k in d and isinstance(d[k], list):
                return d[k]
        # dict of id -> card
        if all(isinstance(v, dict) for v in d.values()):
            return list(d.values())
    return d if isinstance(d, list) else []

cards = None
src = None
for p in cands:
    try:
        v = load(p)
    except Exception as e:
        continue
    if isinstance(v, list) and len(v) > 500:
        cards, src = v, p
        break
if cards is None:
    print('NO CORPUS FOUND'); sys.exit(1)
print('\ncorpus:', os.path.relpath(src, ROOT), 'cards =', len(cards))
print('sample keys:', sorted(cards[0].keys())[:40])

def walk_ab_texts(c):
    """Every string that is an ability's display text."""
    out = []
    for key in ('abilities', 'ability', 'auto_abilities'):
        v = c.get(key)
        if v is None:
            continue
        items = v if isinstance(v, list) else [v]
        for a in items:
            if isinstance(a, dict):
                for k in ('text', 'ability_text', 'full_text', 'description'):
                    if isinstance(a.get(k), str):
                        out.append(a[k])
    if isinstance(c.get('ability_text'), str):
        out.append(c['ability_text'])
    return out

texts, names = [], []
for c in cards:
    if not isinstance(c, dict):
        continue
    texts += walk_ab_texts(c)
    n = c.get('name')
    if isinstance(n, str):
        names.append(n)

print('\nability texts collected:', len(texts))
print('card names collected  :', len(names))

lens = sorted(len(t.encode('utf-8')) for t in texts)
print('\n--- A. ability-text byte lengths ---')
print('  max  :', lens[-1] if lens else 0)
print('  p50  :', lens[len(lens)//2] if lens else 0)
print('  p99  :', lens[int(len(lens)*0.99)] if lens else 0)

for cap in (127, 255, 511, 1023, 4096):
    over = [t for t in texts if len(t.encode('utf-8')) > cap]
    if not over:
        print(f'  cap {cap:>5}: 0 texts exceed'); continue
    torn = 0
    for t in over:
        b = t.encode('utf-8')
        # the byte at index cap is the first NOT copied by a `n < cap` loop
        if cap < len(b) and (b[cap] & 0xC0) == 0x80:
            torn += 1
    print(f'  cap {cap:>5}: {len(over):>5} texts exceed; {torn:>5} of those cuts land '
          f'INSIDE a UTF-8 sequence (torn character)')

C0 = set(range(0x00, 0x20))
def bad(t):
    return any(ord(ch) < 0x20 or ch in '"\\' for ch in t)

print('\n--- B. strings that break an UNESCAPED "%s" JSON embed ---')
bq = [t for t in texts if bad(t)]
bn = [n for n in names if bad(n)]
print(f'  ability texts with " \\ or C0 : {len(bq):>5} / {len(texts)}')
print(f'  card names   with " \\ or C0 : {len(bn):>5} / {len(names)}')
for label, arr in (('text', bq), ('name', bn)):
    kinds = collections.Counter()
    for t in arr:
        for ch in t:
            if ch == '"': kinds['dquote'] += 1
            elif ch == '\\': kinds['backslash'] += 1
            elif ord(ch) < 0x20: kinds[repr(ch)] += 1
    if arr:
        print(f'  {label} offenders by byte: {dict(kinds)}')
        for t in arr[:3]:
            print(f'    e.g. {t[:90]!r}')

print('\n--- C. card names carrying a literal newline ---')
nl = [n for n in names if '\n' in n or '\r' in n]
print(f'  {len(nl)} / {len(names)}')
for n in nl[:4]:
    print('   ', repr(n[:80]))

print('\n--- D. longest name / longest text (for buffer sizing) ---')
if names:
    ln = sorted(names, key=lambda s: -len(s.encode('utf-8')))
    print('  longest card name:', len(ln[0].encode('utf-8')), 'bytes ->', repr(ln[0][:60]))
if texts:
    lt = sorted(texts, key=lambda s: -len(s.encode('utf-8')))
    print('  longest ability text:', len(lt[0].encode('utf-8')), 'bytes')
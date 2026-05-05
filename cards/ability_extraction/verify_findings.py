"""Verify specific findings from deep_qa.py against actual JSON."""
import json, os, unicodedata

path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'abilities.json')
with open(path, 'r', encoding='utf-8') as f:
    data = json.load(f)
ab = data['unique_abilities']

def norm(s):
    return unicodedata.normalize('NFKC', s)

def find_eff(e, depth=0):
    """Print effect tree for debugging"""
    if not e: return
    a = e.get('action','')
    src = e.get('source','?')
    dst = e.get('destination','?')
    txt = e.get('text','')
    ct = e.get('card_type','?')
    if txt:
        nt = norm(txt)
        indent = "  " * depth
        print(f'{indent}action={a} src={src} dst={dst} ct={ct} text={txt[:120]}')
    if a in ('sequential',):
        for s in e.get('actions',[]):
            find_eff(s, depth+1)
    if a == 'look_and_select':
        find_eff(e.get('look_action'), depth+1)
        find_eff(e.get('select_action'), depth+1)
    if a == 'choice':
        for o in e.get('options',[]):
            find_eff(o, depth+1)

# Verify specific interesting findings
print("=" * 80)
print("CATEGORY 1: WRONG SOURCE/DESTINATION")
print("=" * 80)
for idx in [59, 116, 154, 184, 238, 298, 431, 432, 433, 455, 574]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 2: MISSING COUNT")
print("=" * 80)
for idx in [184, 261, 349, 439, 541, 570, 600]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 3: WRONG CARD_TYPE")
print("=" * 80)
for idx in [35, 99, 120, 121, 231, 261, 268, 431, 432, 433, 446, 455, 491]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 5: PER_UNIT WITHOUT PER_UNIT_COUNT")
print("=" * 80)
for idx in [371, 500]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 6: MISSING ALL FLAG")
print("=" * 80)
# Just check a representative sample
for idx in [35, 109, 213, 400, 448, 541]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 7: MISSING MULTIPLE_TARGETS")
print("=" * 80)
for idx in [62, 133, 154]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 8: CONDITIONS WITH MISSING FIELDS")
print("=" * 80)
for idx in [47, 95, 112, 123, 143, 150, 156, 168, 211, 316, 365, 377, 416, 443, 448, 487, 542]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        eff = ab[idx].get('effect', {})
        cond = eff.get('condition', {})
        print(f'  condition type={cond.get("type","?")} location={cond.get("location","?")} target={cond.get("target","?")}')
        if cond.get('type') == 'compound':
            for i, c in enumerate(cond.get('conditions',[])):
                print(f'    sub[{i}] type={c.get("type","?")} location={c.get("location","?")} target={c.get("target","?")}')

print("\n" + "=" * 80)
print("CATEGORY 9: DO_NOTHING")
print("=" * 80)
idx = 491
if idx < len(ab):
    print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
    print(f'  full_text={ab[idx]["full_text"][:200]}')
    find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 10: MISSING/EMPTY ACTION")
print("=" * 80)
for idx in [107, 144, 277, 431, 432, 433]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 11: SEQUENTIAL WITH SINGLE ACTION")
print("=" * 80)
for idx in [47, 204, 248, 257, 290, 359, 399, 495, 504, 543, 597]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        find_eff(ab[idx].get('effect', {}))

print("\n" + "=" * 80)
print("CATEGORY 12: MISSING OPERATOR")
print("=" * 80)
for idx in [23, 63, 381, 538]:
    if idx < len(ab):
        print(f'\n=== [{idx}] cards={ab[idx]["cards"]}')
        print(f'  full_text={ab[idx]["full_text"][:200]}')
        eff = ab[idx].get('effect', {})
        cond = eff.get('condition', {})
        print(f'  condition={json.dumps(cond, ensure_ascii=False)[:300]}')

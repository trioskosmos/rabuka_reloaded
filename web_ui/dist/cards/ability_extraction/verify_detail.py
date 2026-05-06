"""Verify specific findings in detail."""
import json, os, unicodedata

path = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'abilities.json')
with open(path, 'r', encoding='utf-8') as f:
    data = json.load(f)
ab = data['unique_abilities']

def norm(s):
    return unicodedata.normalize('NFKC', s)

def recurse(e, depth=0, visitor=None):
    if not e: return
    if visitor:
        visitor(e, depth)
    for s in e.get('actions', []):
        recurse(s, depth+1, visitor)
    for s in e.get('options', []):
        recurse(s, depth+1, visitor)
    if e.get('action') == 'look_and_select':
        recurse(e.get('look_action'), depth+1, visitor)
        recurse(e.get('select_action'), depth+1, visitor)

# Category 2: Check which have dynamic_count
print('=== CAT 2: Missing count - do they have dynamic_count? ===')
for idx in [184, 261, 349, 439, 541, 570, 600]:
    if idx >= len(ab): continue
    def check(e, depth):
        if e.get('action') in ('move_cards',) and (e.get('count') is None or 'count' not in e):
            dc = e.get('dynamic_count')
            print(f'  [{idx}] action={e["action"]} dynamic_count={json.dumps(dc) if dc else "MISSING"} text={e.get("text","")[:80]}')
    recurse(ab[idx].get('effect'), visitor=check)

# Category 1: Check real vs false positive
print()
print('=== CAT 1: Check real destination issues ===')
for idx in [59, 116, 154, 184, 238, 298, 431, 432, 433, 455, 574]:
    if idx >= len(ab): continue
    print(f'\n[{idx}] full_text={ab[idx]["full_text"][:150]}')
    def check(e, depth):
        if not e.get('text'): return
        a = e.get('action','?')
        s = e.get('source','?')
        d = e.get('destination','?')
        t = e.get('text','')
        nt = norm(t)
        if a in ('move_cards','draw_card','look_at','reveal'):
            print(f'  action={a} src={s} dst={d} text={t[:100]}')
    recurse(ab[idx].get('effect'), visitor=check)

# Category 3: Check card_type issues
print()
print('=== CAT 3: Check card_type issues ===')
for idx in [35, 99, 120, 121, 231, 261, 268, 431, 432, 433, 446, 455, 491]:
    if idx >= len(ab): continue
    print(f'\n[{idx}] full_text={ab[idx]["full_text"][:150]}')
    def check(e, depth):
        if not e.get('text'): return
        a = e.get('action','?')
        ct = e.get('card_type','?')
        t = e.get('text','')
        nt = norm(t)
        has_en = 'エネルギーカード' in nt
        has_lv = 'ライブカード' in nt
        has_mb = 'メンバーカード' in nt
        if has_en or has_lv or has_mb:
            issues = []
            if has_en and ct not in ('energy_card','card','?'):
                issues.append(f'energy_keyword but ct={ct}')
            if has_lv and ct not in ('live_card','card','?'):
                issues.append(f'live_keyword but ct={ct}')
            if has_mb and ct not in ('member_card','card','?'):
                issues.append(f'member_keyword but ct={ct}')
            if issues:
                print(f'  action={a} ct={ct} ISSUES: {", ".join(issues)}')
                print(f'  text={t[:100]}')
    recurse(ab[idx].get('effect'), visitor=check)

# Category 10: Check custom actions
print()
print('=== CAT 10: custom/empty actions ===')
for idx in [107, 144, 277, 431, 432, 433]:
    if idx >= len(ab): continue
    print(f'\n[{idx}] full_text={ab[idx]["full_text"][:150]}')
    def check(e, depth):
        a = e.get('action','')
        t = e.get('text','')
        if not a or a == '':
            print(f'  EMPTY action text={t[:100]}')
        elif a == 'custom':
            print(f'  CUSTOM action text={t[:100]}')
    recurse(ab[idx].get('effect'), visitor=check)

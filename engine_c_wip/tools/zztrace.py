import json, glob, os
ROOT = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded'
p = glob.glob(os.path.join(ROOT, '**', 'cards', 'cards.json'), recursive=True)[0]
cards = json.load(open(p, encoding='utf-8'))
if isinstance(cards, dict): cards = list(cards.values())

texts = [c['ability'] for c in cards if isinstance(c, dict) and isinstance(c.get('ability'), str)]
print('ability texts:', len(texts), ' max bytes:', max(len(t.encode()) for t in texts))

print('\ncap   exceed   torn-UTF8   exact-127-boundary-note')
for cap in (127, 128, 200, 255, 384, 511, 687, 1023, 2047, 4096):
    over = [t for t in texts if len(t.encode()) > cap]
    torn = 0
    for t in over:
        b = t.encode()
        if (b[cap] & 0xC0) == 0x80:
            torn += 1
    print(f'{cap:>5} {len(over):>7} {torn:>11}')

print('\nDistinct ability texts:', len({t for t in texts}))
u = {t for t in texts if len(t.encode()) > 127}
print('distinct texts over 127:', len(u))

# Does anything in the tree cap ability text at 127/128 that is NOT the trace ring?
# grep for the exact constants
print('\n--- texts that would be TORN at cap 127 (sample 5) ---')
n = 0
for t in texts:
    b = t.encode()
    if len(b) > 127 and (b[127] & 0xC0) == 0x80:
        n += 1
        if n <= 5:
            print(f'   {b[:127].decode("utf-8", "replace")[-18:]!r} | CUT {b[127:130]!r}')
print('   total torn at 127:', n)
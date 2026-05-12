
import json

ABILITIES_FILE = 'cards/abilities.json'
with open(ABILITIES_FILE, encoding='utf-8') as fh:
    data = json.load(fh)

abilities = data['unique_abilities']

def find_keys(d, key):
    vals = set()
    if isinstance(d, dict):
        if key in d:
            vals.add(str(d[key]))
        for k, v in d.items():
            if k == 'text':
                continue
            if isinstance(v, dict):
                vals.update(find_keys(v, key))
            elif isinstance(v, list):
                for item in v:
                    vals.update(find_keys(item, key))
    return vals

results = []

# 1. choudo operator check
results.append('=== 1. choudo (exactly N) ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if '\u3061\u3087\u3046\u3069' not in t:
        continue
    combined = {}
    if isinstance(a.get('cost'), dict):
        combined['cost'] = a['cost']
    if isinstance(a.get('effect'), dict):
        combined['effect'] = a['effect']
    ops = find_keys(combined, 'operator')
    results.append('#' + str(i) + ' ops=' + str(ops))
    results.append('  ' + t[:80])

# 2. blade count
results.append('')
results.append('=== 2. blade_count (blade no kazu ga) ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if '\u30d6\u30ec\u30fc\u30c9\u306e\u6570' not in t:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    bl = eff.get('blade_limit')
    ov = eff.get('original_value')
    results.append('#' + str(i) + ' blade_limit=' + str(bl) + ' original_value=' + str(ov))
    results.append('  ' + t[:80])

# 3. kagiri without as_long_as  
results.append('')
results.append('=== 3. kagiri WITHOUT as_long_as ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if '\u304b\u304e\u308a\u3001' not in t:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    durs = find_keys(eff, 'duration')
    if 'as_long_as' not in durs:
        results.append('#' + str(i) + ' duration=' + str(durs))
        results.append('  ' + t[:80])

# 4. goukei without aggregate
results.append('')
results.append('=== 4. goukei (total) WITHOUT aggregate ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if '\u5408\u8a08' not in t:
        continue
    if '\u5408\u8a08\u30b9\u30b3\u30a2' in t:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    aggs = find_keys(eff, 'aggregate')
    if 'total' not in aggs:
        results.append('#' + str(i) + ' aggregate=' + str(aggs))
        results.append('  ' + t[:80])

# 5. sorezore without multiple_targets
results.append('')
results.append('=== 5. sorezore WITHOUT multiple_targets ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if '\u305d\u308c\u305e\u308c' not in t:
        continue
    if '\u305d\u308c\u305e\u308c\u7570\u306a\u308b' in t or '\u305d\u308c\u305e\u308c\u306e' in t:
        continue
    if '\u305d\u308c\u305e\u308c\u540d\u524d' in t or '\u305d\u308c\u305e\u308c1\u4ee5\u4e0a' in t:
        continue
    if '\u305d\u308c\u305e\u308c\u597d\u304d' in t:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    mults = find_keys(eff, 'multiple_targets')
    if 'True' not in mults:
        results.append('#' + str(i) + ' multiple_targets=' + str(mults))
        results.append('  ' + t[:80])

# 6. center area without activation_position
results.append('')
results.append('=== 6. center area WITHOUT activation_position ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if '\u30bb\u30f3\u30bf\u30fc\u30a8\u30ea\u30a2' not in t:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    aps = find_keys(eff, 'activation_position')
    poss = find_keys(eff, 'position')
    if 'center' not in aps:
        results.append('#' + str(i) + ' activation_position=' + str(aps) + ' position=' + str(poss))
        results.append('  ' + t[:80])

for line in results:
    print(line)

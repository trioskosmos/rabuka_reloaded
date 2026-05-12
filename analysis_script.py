
import json, sys
sys.path.insert(0, 'cards/ability_extraction/common_phrases')
from pathlib import Path
import parser

ABILITIES_FILE = Path('cards/abilities.json')
with open(ABILITIES_FILE, encoding='utf-8') as f:
    data = json.load(f)
data = parser.process_abilities(data)
abilities = data['unique_abilities']

# Helper
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

# 1. choudo
print('=== 1. choudo (exactly N) operator check ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if chr(12385)+chr(12358)+chr(12375) not in t:  # ちょうど
        continue
    combined = {}
    if isinstance(a.get('cost'), dict):
        combined['cost'] = a['cost']
    if isinstance(a.get('effect'), dict):
        combined['effect'] = a['effect']
    ops = find_keys(combined, 'operator')
    print('#' + str(i) + ' ops=' + str(ops))
    print('  ' + t[:70])
    if i > 6:
        break

# 2. blade count
print()
print('=== 2. blade_count entries ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if chr(12502)+chr(12524)+chr(12540)+chr(12489)+chr(12398)+chr(25968) not in t:  # ブレードの数
        continue
    eff = a.get('effect', {})
    bl = eff.get('blade_limit') if isinstance(eff, dict) else None
    ov = eff.get('original_value') if isinstance(eff, dict) else None
    if bl is not None:
        print('#' + str(i) + ' HAS blade_limit=' + str(bl) + ' orig=' + str(ov))
    else:
        print('#' + str(i) + ' MISSING blade_limit orig=' + str(ov))
    print('  ' + t[:70])

# 3. kagiri without as_long_as
print()
print('=== 3. kagiri without as_long_as ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if chr(12363)+chr(12366)+chr(12426)+chr(12289) not in t:  # かぎり、
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    durs = find_keys(eff, 'duration')
    if 'as_long_as' not in durs:
        print('#' + str(i) + ' durs=' + str(durs))
        print('  ' + t[:70])

# 4. goukei without aggregate (but not 合計スコア)
print()
print('=== 4. goukei without aggregate ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if chr(21512)+chr(35336) not in t:  # 合計
        continue
    if chr(21512)+chr(35336)+chr(12473)+chr(12467)+chr(12450) in t:  # 合計スコア
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    aggs = find_keys(eff, 'aggregate')
    if 'total' not in aggs:
        print('#' + str(i) + ' agg=' + str(aggs))
        print('  ' + t[:70])

# 5. sorezore without multiple_targets (excluding condition patterns)
print()
print('=== 5. sorezore without multiple_targets ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if chr(12381)+chr(12428)+chr(12380)+chr(12428) not in t:  # それぞれ
        continue
    # Skip condition patterns
    skip_phrases = [chr(12381)+chr(12428)+chr(12380)+chr(12428)+chr(30064)+chr(12394)+chr(12427),  # それぞれ異なる
                    chr(12381)+chr(12428)+chr(12380)+chr(12428)+chr(12398),  # それぞれの
                    chr(12381)+chr(12428)+chr(12380)+chr(12428)+chr(21517)+chr(21069),  # それぞれ名前
                    chr(12381)+chr(12428)+chr(12380)+chr(12428)+chr(49)+chr(20197)+chr(19978),  # それぞれ1以上
                    chr(12381)+chr(12428)+chr(12380)+chr(12428)+chr(22909)+chr(12365)]  # それぞれ好き
    skip = False
    for sp in skip_phrases:
        if sp in t:
            skip = True
            break
    if skip:
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    mults = find_keys(eff, 'multiple_targets')
    if 'True' not in mults:
        print('#' + str(i) + ' mult=' + str(mults))
        print('  ' + t[:70])

# 6. center area without activation_position
print()
print('=== 6. center area without activation_position ===')
for i, a in enumerate(abilities):
    t = a.get('triggerless_text', '') or a.get('full_text', '')
    if chr(12475)+chr(12531)+chr(12479)+chr(12456)+chr(12522)+chr(12450) not in t:  # センターエリア
        continue
    eff = a.get('effect', {})
    if not isinstance(eff, dict):
        continue
    aps = find_keys(eff, 'activation_position')
    poss = find_keys(eff, 'position')
    if 'center' not in aps:
        print('#' + str(i) + ' activation_position=' + str(aps) + ' position=' + str(poss))
        print('  ' + t[:70])

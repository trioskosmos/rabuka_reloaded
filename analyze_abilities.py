import json
import re
from collections import Counter

with open('cards/abilities.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

abilities = data['unique_abilities']
total = len(abilities)
print('Total unique abilities:', total)
print('Total card occurrences:', sum(a['card_count'] for a in abilities))
print()

# 1. TRIGGER TYPE ANALYSIS
trigger_types = Counter()
for a in abilities:
    t = a.get('triggers') or 'none'
    trigger_types[t] += 1

print('=== TRIGGER TYPES ===')
for t, cnt in trigger_types.most_common():
    print(f'  [{t}] = {cnt}')
print()

# 2. STRUCTURAL FEATURE COUNTS
texts = [a['triggerless_text'] for a in abilities]
colon_count = sum(1 for t in texts if '\uff1a' in t)
baai_count = sum(1 for t in texts if '\u5834\u5408' in t)
toki_count = sum(1 for t in texts if '\u3068\u304d' in t)
nara_count = sum(1 for t in texts if '\u306a\u3089' in t)
sonogo_count = sum(1 for t in texts if '\u305d\u306e\u5f8c' in t)
sarani_count = sum(1 for t in texts if '\u3055\u3089\u306b' in t)
kagiri_count = sum(1 for t in texts if '\u304b\u304e\u308a' in t)
nitsuki_count = sum(1 for t in texts if '\u306b\u3064\u304d' in t)
sononaka_count = sum(1 for t in texts if '\u305d\u306e\u4e2d\u304b\u3089' in t)
choice_count = sum(1 for t in texts if '\u4ee5\u4e0b\u304b\u30891\u3064\u3092\u9078\u3076' in t or '\u4ee5\u4e0b\u304b\u3089\u3072\u3068\u3064' in t)

print('=== STRUCTURAL FEATURE PRESENCE ===')
print(f'  Colon (U+FF1A): {colon_count} ({colon_count/total*100:.1f}%)')
print(f'  Baai: {baai_count} ({baai_count/total*100:.1f}%)')
print(f'  Toki: {toki_count} ({toki_count/total*100:.1f}%)')
print(f'  Nara: {nara_count} ({nara_count/total*100:.1f}%)')
print(f'  Sonogo: {sonogo_count} ({sonogo_count/total*100:.1f}%)')
print(f'  Sarani: {sarani_count} ({sarani_count/total*100:.1f}%)')
print(f'  Kagiri: {kagiri_count} ({kagiri_count/total*100:.1f}%)')
print(f'  Nitsuki: {nitsuki_count} ({nitsuki_count/total*100:.1f}%)')
print(f'  Sononaka: {sononaka_count} ({sononaka_count/total*100:.1f}%)')
print(f'  Choice: {choice_count} ({choice_count/total*100:.1f}%)')
print()

# 3. ACTION TYPE ANALYSIS
print('=== ACTION TYPES (from parser effect.action) ===')
effect_actions = Counter()
for a in abilities:
    eff = a.get('effect')
    if isinstance(eff, dict):
        effect_actions[eff.get('action', 'unknown')] += 1
    elif isinstance(eff, list):
        effect_actions['list(' + str(len(eff)) + ')'] += 1
    else:
        effect_actions['none/missing'] += 1
for act, cnt in effect_actions.most_common():
    print(f'  {act}: {cnt}')
print()

print('=== COST TYPES (from parser cost.type) ===')
cost_actions = Counter()
for a in abilities:
    cost = a.get('cost')
    if isinstance(cost, dict):
        cost_actions[cost.get('type', 'unknown')] += 1
    elif isinstance(cost, list):
        cost_actions['list(' + str(len(cost)) + ')'] += 1
    elif cost is None:
        cost_actions['none'] += 1
    else:
        cost_actions['other'] += 1
for act, cnt in cost_actions.most_common():
    print(f'  {act}: {cnt}')
print()

# 4. STRUCTURAL TEMPLATES
def safe_str(s):
    return str(s) if s is not None else '?'

def get_template(a):
    text = a['triggerless_text']
    trigger = a.get('triggers') or 'none'
    parts = ['T:' + trigger]
    
    if '\uff1a' in text:
        parts.append('COLON')
        cost = a.get('cost')
        if isinstance(cost, dict):
            parts.append('COST:' + cost.get('type', '?'))
        elif isinstance(cost, list):
            parts.append('COST:[' + '+'.join(c.get('type', '?') for c in cost) + ']')
        else:
            parts.append('COST:?')
        
        eff = a.get('effect')
        if isinstance(eff, dict):
            ea = eff.get('action', '?')
            if ea == 'move_cards':
                parts.append('EFF:' + ea + '(' + safe_str(eff.get('source')) + '->' + safe_str(eff.get('destination')) + ')')
            elif ea == 'gain_resource':
                parts.append('EFF:' + ea + '(' + safe_str(eff.get('resource_type')) + ')')
            else:
                parts.append('EFF:' + ea)
        elif isinstance(eff, list):
            actions = [e.get('action', '?') for e in eff]
            parts.append('EFF:[' + '+'.join(actions) + ']')
        else:
            parts.append('EFF:?')
    else:
        parts.append('NOCOLON')
        structs = []
        if '\u5834\u5408' in text: structs.append('BAAI')
        if '\u3068\u304d' in text: structs.append('TOKI')
        if '\u306a\u3089' in text: structs.append('NARA')
        if '\u305d\u306e\u5f8c' in text: structs.append('SONOGO')
        if '\u3055\u3089\u306b' in text: structs.append('SARANI')
        if '\u304b\u304e\u308a' in text: structs.append('KAGIRI')
        if '\u306b\u3064\u304d' in text: structs.append('NITSUKI')
        if '\u305d\u306e\u4e2d\u304b\u3089' in text: structs.append('SONONAKA')
        if '\u4ee5\u4e0b\u304b\u30891\u3064\u3092\u9078\u3076' in text or '\u4ee5\u4e0b\u304b\u3089\u3072\u3068\u3064' in text:
            structs.append('CHOICE')
        if structs:
            parts.append('STRUCT:' + '+'.join(structs[:3]))
        
        eff = a.get('effect')
        if isinstance(eff, dict):
            ea = eff.get('action', '?')
            if ea == 'move_cards':
                parts.append('EFF:' + ea + '(' + safe_str(eff.get('source')) + '->' + safe_str(eff.get('destination')) + ')')
            elif ea == 'gain_resource':
                parts.append('EFF:' + ea + '(' + safe_str(eff.get('resource_type')) + ')')
            else:
                parts.append('EFF:' + ea)
        elif isinstance(eff, list):
            actions = [e.get('action', '?') for e in eff]
            parts.append('EFF:[' + '+'.join(actions) + ']')
        else:
            parts.append('EFF:?')
    
    return ' | '.join(parts)

templates = Counter()
for a in abilities:
    tmpl = get_template(a)
    templates[tmpl] += 1

total_templates = len(templates)
print('Total distinct structural templates:', total_templates)
print()

# Top 20
print('=== TOP 20 MOST COMMON STRUCTURAL TEMPLATES ===')
sorted_tmpl = templates.most_common()
for i, (tmpl, cnt) in enumerate(sorted_tmpl[:20], 1):
    example = ''
    for a in abilities:
        if get_template(a) == tmpl:
            example = a['triggerless_text'][:120]
            break
    print(f'{i:2d}. [{cnt:3d}] {tmpl}')
    print(f'     eg: {example}')
    print()

# 5. COVERAGE ANALYSIS
print('=== COVERAGE ANALYSIS ===')
for thresh in [50, 80, 90, 95, 99]:
    target = total * thresh / 100
    cum = 0
    nt = 0
    for tmpl, cnt in sorted_tmpl:
        cum += cnt
        nt += 1
        if cum >= target:
            break
    print(f'  {thresh}% coverage = {nt} templates ({cum}/{total} abilities)')
print()

cum = 0
for i, (tmpl, cnt) in enumerate(sorted_tmpl, 1):
    cum += cnt
    if i in [5, 10, 15, 20, 25, 30, 40, 50, 75, 100]:
        print(f'  Top {i:3d} templates: {cum:3d}/{total} = {cum/total*100:5.1f}%')
    if i > 100:
        break
print()

# 6. CONDITION STRUCTURES
print('=== CONDITIONAL STRUCTURES ===')
cond_by_type = Counter()
for a in abilities:
    text = a['triggerless_text']
    if '\u5834\u5408' in text:
        if '\u306a\u3044\u5834\u5408' in text:
            cond_by_type['baai(negative)'] += 1
        else:
            cond_by_type['baai(positive)'] += 1
    if '\u3068\u304d' in text:
        cond_by_type['toki'] += 1
    if '\u306a\u3089' in text:
        cond_by_type['nara'] += 1
for ct, cnt in cond_by_type.most_common():
    print(f'  {ct}: {cnt}')
print()

# 7. MOVE_CARDS DETAIL
print('=== MOVE_CARDS SOURCE->DESTINATION (effect) ===')
move_pairs = Counter()
for a in abilities:
    eff = a.get('effect')
    if isinstance(eff, dict) and eff.get('action') == 'move_cards':
        src = safe_str(eff.get('source'))
        dst = safe_str(eff.get('destination'))
        move_pairs[src + '->' + dst] += 1
    elif isinstance(eff, list):
        for e in eff:
            if isinstance(e, dict) and e.get('action') == 'move_cards':
                src = safe_str(e.get('source'))
                dst = safe_str(e.get('destination'))
                move_pairs[src + '->' + dst] += 1
for pair, cnt in move_pairs.most_common(15):
    print(f'  {pair}: {cnt}')
print()

# 8. NON-COLON BREAKDOWN
print('=== NON-COLON STRUCTURES DETAILED ===')
nocolon_abilities = [a for a in abilities if '\uff1a' not in a['triggerless_text']]
print(f'  Total non-colon: {len(nocolon_abilities)}/{total}')
nocolon_classes = Counter()
for a in nocolon_abilities:
    text = a['triggerless_text']
    tags = []
    if '\u5834\u5408' in text or '\u3068\u304d' in text or '\u306a\u3089' in text:
        tags.append('conditional')
    if '\u304b\u304e\u308a' in text:
        tags.append('duration')
    if '\u306b\u3064\u304d' in text:
        tags.append('per_unit')
    if '\u305d\u306e\u5f8c' in text or '\u3055\u3089\u306b' in text:
        tags.append('sequential')
    if '\u4ee5\u4e0b\u304b\u30891\u3064\u3092\u9078\u3076' in text or '\u4ee5\u4e0b\u304b\u3089\u3072\u3068\u3064' in text:
        tags.append('choice')
    if '\u305d\u306e\u4e2d\u304b\u3089' in text:
        tags.append('look_select')
    if '\u3001' in text:
        tags.append('comma_seq')
    if not tags:
        tags.append('simple')
    nocolon_classes[' + '.join(tags)] += 1
for cls, cnt in nocolon_classes.most_common():
    print(f'  {cls}: {cnt}')
print()

# 9. MULTI-SENTENCE
print('=== MULTI-SENTENCE EFFECTS ===')
multi_sent = []
for a in abilities:
    sents = [s.strip() for s in re.split(r'[\u3002]', a['triggerless_text']) if s.strip()]
    if len(sents) > 1:
        multi_sent.append(a)
print(f'  Total: {len(multi_sent)}/{total}')
for m in multi_sent[:8]:
    sents = [s.strip() for s in re.split(r'[\u3002]', m['triggerless_text']) if s.strip()]
    print(f'  [{m["triggers"]}] ({len(sents)} sents): {m["triggerless_text"][:120]}')
print()

# 10. SINGLETON / DISTRIBUTION ANALYSIS
singletons = sum(1 for _, c in sorted_tmpl if c == 1)
doubletons = sum(1 for _, c in sorted_tmpl if c == 2)
tripletons = sum(1 for _, c in sorted_tmpl if c == 3)
print('=== TEMPLATE DISTRIBUTION ===')
print(f'  Singleton templates (1 ability): {singletons}')
print(f'  Doubleton templates (2 abilities): {doubletons}')
print(f'  Tripleton templates (3 abilities): {tripletons}')
print(f'  Templates with 4+ abilities: {total_templates - singletons - doubletons - tripletons}')
print()

# Show count distribution
dist = Counter()
for _, c in sorted_tmpl:
    if c <= 5: dist[c] += 1
    elif c <= 10: dist['6-10'] += 1
    elif c <= 20: dist['11-20'] += 1
    elif c <= 50: dist['21-50'] += 1
    else: dist['51+'] += 1
print('Template size distribution:')
for k in sorted(dist.keys(), key=lambda x: (isinstance(x, int), x)):
    print(f'  {k} abilities: {dist[k]} templates')
print()

# 11. RARE ACTIONS
print('=== RARE ACTION TYPES (< 15 occurrences) ===')
rare_actions = Counter()
for a in abilities:
    eff = a.get('effect')
    if isinstance(eff, dict):
        rare_actions[eff.get('action', '?')] += 1
    elif isinstance(eff, list):
        for e in eff:
            if isinstance(e, dict):
                rare_actions[e.get('action', '?')] += 1
for act, cnt in rare_actions.most_common():
    if cnt < 15:
        print(f'  {act}: {cnt}')
        for a in abilities:
            eff = a.get('effect')
            if isinstance(eff, dict) and eff.get('action') == act:
                print(f'    eg: {a["triggerless_text"][:100]}')
                break
            elif isinstance(eff, list):
                for e in eff:
                    if isinstance(e, dict) and e.get('action') == act:
                        print(f'    eg: {a["triggerless_text"][:100]}')
                        break
print()

# 12. FINAL SUMMARY
print('=' * 60)
print('FINAL SUMMARY')
print('=' * 60)
print('Total unique abilities:', total)
print('Total cards with abilities:', data['statistics']['cards_with_abilities'])
print('Total abilities (incl. duplicates):', data['statistics']['total_abilities'])
print('Distinct structural templates:', total_templates)
print('Abilities-per-template ratio:', round(total/total_templates, 2))
print()

cum = 0
for i, (tmpl, cnt) in enumerate(sorted_tmpl, 1):
    cum += cnt
    if i in [10, 20, 30, 50]:
        print(f'Top {i:2d} templates: {cum:3d}/{total} = {cum/total*100:5.1f}%')

print()
# Find coverage thresholds
t80 = next(i for i, (_, c) in enumerate(sorted_tmpl, 1) if sum(x[1] for x in sorted_tmpl[:i]) >= total*0.8)
t90 = next(i for i, (_, c) in enumerate(sorted_tmpl, 1) if sum(x[1] for x in sorted_tmpl[:i]) >= total*0.9)
t95 = next(i for i, (_, c) in enumerate(sorted_tmpl, 1) if sum(x[1] for x in sorted_tmpl[:i]) >= total*0.95)
print(f'Templates to cover 80%: ~{t80}')
print(f'Templates to cover 90%: ~{t90}')
print(f'Templates to cover 95%: ~{t95}')
print()

if total_templates <= 200:
    verdict = 'HIGHLY FEASIBLE'
elif total_templates <= 400:
    verdict = 'FEASIBLE'
elif total_templates <= 500:
    verdict = 'MODERATE'
else:
    verdict = 'CHALLENGING'
print(f'Verdict: {verdict}')
print(f'  {total_templates} templates for {total} abilities = {total/total_templates:.1f} abilities/template')
print(f'  {singletons} singletons ({singletons/total_templates*100:.1f}% of templates)')

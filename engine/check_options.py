import json

with open('../cards/abilities.json', encoding='utf-8') as f:
    abilities = json.load(f)

ab_list = abilities.get('unique_abilities', [])
ab = ab_list[402]
eff = ab.get('effect', {})
options = eff.get('options', [])
print('Options count:', len(options))
for i, opt in enumerate(options):
    if isinstance(opt, dict):
        keys = list(opt.keys())
        print('Option {}: keys={}'.format(i, keys))
        print('  action={}'.format(opt.get('action')))
        print('  target={}'.format(opt.get('target')))
        text = opt.get('text', '')[:60]
        print('  text={}'.format(text))
        # Check sub-actions
        if 'actions' in opt:
            acts = opt['actions']
            print('  sub-actions: {}'.format(len(acts) if isinstance(acts, list) else 'not list'))
            for j, sa in enumerate(acts if isinstance(acts, list) else []):
                if isinstance(sa, dict):
                    print('    [{}] action={} target={} source={} dest={}'.format(
                        j, sa.get('action'), sa.get('target'), 
                        sa.get('source', '?'), sa.get('destination', '?')))

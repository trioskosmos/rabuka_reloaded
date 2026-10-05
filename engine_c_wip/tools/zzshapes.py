import glob, os, re
for s in ('audit_fields', 'parity_zone_source', 'parity_zones_position',
          'harness_self', 'mechanics'):
    p = os.path.join('zzlogs', f'ZZ_{s}.txt')
    t = open(p, 'rb').read().decode('utf-8', 'replace')
    print('=' * 70)
    print(s)
    print('=' * 70)
    lines = [l for l in t.split('\n') if re.search(r'\d', l)]
    for l in lines[-14:]:
        print('   ', l[:110])
    print()
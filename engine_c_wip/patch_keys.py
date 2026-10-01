import io
p = 'tests/test_audit_conditions.c'
s = io.open(p, encoding='utf-8').read()
s = s.replace('RB_COND_COMPARISON, "comparison_type", &c',
              'RB_COND_COMPARISON, "all", &c')
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('remaining comparison_type keys:', s.count('"comparison_type"'))
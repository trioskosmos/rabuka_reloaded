src = open('tests/test_ported_generated.c', encoding='utf-8').read().split('\n')
for i, l in enumerate(src):
    if 'g.id(' in l or 'g.new_id(' in l or 'v.id(' in l or 'g2.id(' in l:
        if 'test_id' not in l:
            print(i+1, repr(l))
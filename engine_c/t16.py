import os, re
for root,_,fs in os.walk('engine/tests/test_modules'):
    for f in fs:
        if 'nijigasaki_bp1_006r' in f:
            p=os.path.join(root,f)
            print(p)
            print(open(p,encoding='utf-8',errors='ignore').read()[:2000])
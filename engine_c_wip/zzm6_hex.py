import re
s=open('zzs/BASE/describe_golden.err','rb').read().decode('utf-8','replace')
for m in re.finditer(r'(rust|c)\s*\((en|ja)\): ([0-9a-f]+)',s):
    who,lang,hx=m.group(1),m.group(2),m.group(3)
    b=bytes.fromhex(hx)
    print('%-4s %s  %r'%(who,lang,b.decode('utf-8',errors='replace')))
    print('      cp:', ' '.join('U+%04X'%ord(c) for c in b.decode('utf-8',errors='replace')))
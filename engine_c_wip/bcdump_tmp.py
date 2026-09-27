import re, io, sys, os
ROOT = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded'
CWP = os.path.join(ROOT, 'engine_c_wip')

gd = io.open(os.path.join(CWP, 'src/core/generated/gen_data.c'), encoding='utf-8', errors='replace').read()

def grab_array(name):
    m = re.search(re.escape(name) + r'\s*\[[^\]]*\]\s*=\s*\{(.*?)\};', gd, re.S)
    if not m: return None
    body = m.group(1)
    return [int(x) for x in re.findall(r'-?\d+', body)]

offs = grab_array('RBKA_STRINGS_OFFSETS')
deltas = grab_array('RBKA_OFFSET_DELTAS')
print('strings_offsets', len(offs) if offs else None, 'deltas', len(deltas) if deltas else None)

blob = open(os.path.join(CWP, 'src/abilities_strings.bin'), 'rb').read()
bc_txt = io.open(os.path.join(CWP, 'src/core/generated/bytecode_blob.c'), encoding='utf-8', errors='replace').read()
m = re.search(r'RBKA_BYTECODE\[\]\s*=\s*\{(.*?)\};', bc_txt, re.S)
bc = bytes(int(x) for x in re.findall(r'\d+', m.group(1)))
print('blob', len(blob), 'bc', len(bc))

strings = []
for i in range(len(offs) - 1):
    a, b = offs[i], offs[i + 1]
    strings.append(blob[a:b].decode('utf-8', 'replace'))

def gs(i):
    return strings[i] if 0 <= i < len(strings) else ''

# tag constants
TAG_NULL, TAG_TRUE, TAG_FALSE, TAG_I64, TAG_STR, TAG_F64, TAG_ARRAY, TAG_OBJECT, TAG_OBJVAR = 0,1,2,3,6,7,8,9,9
print('tags guessed; verify below')

class R:
    def __init__(s, b, p=0): s.b=b; s.p=p
    def u8(s):
        v=s.b[s.p]; s.p+=1; return v
    def u16(s):
        v=s.b[s.p]|(s.b[s.p+1]<<8); s.p+=2; return v
    def u32(s):
        v=int.from_bytes(s.b[s.p:s.p+4],'little'); s.p+=4; return v
    def i64(s):
        v=int.from_bytes(s.b[s.p:s.p+8],'little',signed=True); s.p+=8; return v
    def ln(s):
        b=s.u8()
        return b if b<0xFE else s.u16()
    def idx(s):
        b=s.u8()
        return s.u16() if b==0xFE else b
    def integer(s):
        b=s.u8()
        if b<=0xFD: return b
        if b==0xFE: return s.u16()
        if b==0xFF: return int.from_bytes(bytes([s.b[s.p]]),'little',signed=True) if False else (lambda x: x)(0)
        return s.i64()
    def int_(s):
        b=s.u8()
        if b<=0xFD: return b
        if b==0xFE: return s.u16()
        if b==0xFF:
            v=int.from_bytes(s.b[s.p:s.p+4],'little',signed=True); s.p+=4; return v
        return s.i64()

def skip_val(r, tag):
    if tag in (0,1,2): return
    if tag==3: r.int_(); return
    if tag==4: r.u32(); return
    if tag==5: r.f64(); return
    if tag==6: r.idx(); return
    if tag==7: r.p+=8; return
    if tag==8:
        n=r.ln()
        for _ in range(n): skip_val(r, r.u8())
        return
    if tag==9:
        n=r.ln()
        for _ in range(n):
            r.idx(); skip_val(r, r.u8())
        return
    raise Exception('unknown tag %d at %d' % (tag, r.p))

def rd_objvar(r):
    # returns list of (key, tag, payload) shallow
    variant = r.u8()
    n = r.ln()
    out=[]
    for _ in range(n):
        k = gs(r.idx())
        t = r.u8()
        out.append((k,t,r))
    return variant, out

def dump_ability(idx):
    start = sum(deltas[:idx]); ln = deltas[idx]
    r = R(bc, start)
    tag = r.u8(); n = r.ln()
    print('--- ability', idx, 'slice', start, ln, 'tag', tag, 'nfields', n)
    for _ in range(n):
        k = gs(r.idx()); t = r.u8()
        print('   field %-22s tag=%d' % (k, t), end='')
        if t==6:
            si = r.idx(); print(' ->', repr(gs(si)))
        elif t==3:
            v = r.int_(); print(' -> i64', v)
        elif t==1: print(' -> true')
        elif t==2: print(' -> false')
        elif t==9:
            variant = r.u8(); m = r.ln()
            print(' -> OBJVAR variant=%d nfields=%d' % (variant, m))
            for _ in range(m):
                kk = gs(r.idx()); tt = r.u8()
                pre = '        %-20s tag=%d' % (kk, tt)
                if tt==6:
                    si=r.idx(); print(pre, '->', repr(gs(si)))
                elif tt==3:
                    print(pre, '-> i64', r.int_())
                elif tt==1: print(pre, '-> true')
                elif tt==2: print(pre, '-> false')
                elif tt==8:
                    nn=r.ln(); vals=[]
                    for _ in range(nn):
                        st=r.u8()
                        if st==6: vals.append(gs(r.idx()))
                        elif st==3: vals.append(r.int_())
                        else: skip_val(r,st); vals.append('?')
                    print(pre, '-> ARRAY', vals)
                elif tt==9:
                    print(pre, '-> OBJVAR (nested)')
                else:
                    skip_val(r,tt); print(pre,'-> skipped')
        else:
            skip_val(r,t); print(' -> skipped')

if len(sys.argv)>1:
    for a in sys.argv[1:]:
        dump_ability(int(a))
else:
    print('pass ability indices')



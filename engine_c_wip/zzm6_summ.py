import glob,os,re,sys
tag=sys.argv[1]
d=os.path.join('zzs',tag)
rows=[]
for f in sorted(glob.glob(os.path.join(d,'*.out'))):
    b=os.path.basename(f)[:-4]
    if b=='summary': continue
    s=open(f,'rb').read().decode('utf-8','replace')
    e=open(f[:-4]+'.err','rb').read().decode('utf-8','replace')
    both=s+e
    v=re.findall(r'^(?:PASSED|FAILED): (\S+)\s+\[exit=(\d+), (\d+) result line\(s\)(?:: assert=(\d+) vacuous=(\d+) setup-bug=(\d+) crash=(\d+) test-bug=(\d+) fatal=(\d+) renormalised=(\d+))?',both,re.M)
    red=re.findall(r'^(RED|GREEN|YELLOW): (\d+) checks, (\d+) failures',both,re.M)
    info=re.findall(r'^info: .*$',both,re.M)
    # raw FAIL: lines that survived (the verdict runner rewrites the SUITE prefix; FAIL: case lines keep their own)
    rawfail=len(re.findall(r'^FAIL: ',both,re.M))
    rows.append((b,v,red,info,rawfail))
for b,v,red,info,rawfail in rows:
    verdict=v[-1] if v else None
    if verdict:
        nm,ex,rl=verdict[0],verdict[1],verdict[2]
        det=('assert=%s vac=%s renorm=%s'%(verdict[3],verdict[4],verdict[9])) if len(verdict)>3 else ''
        print('%-28s verdict=%-6s exit=%s result_lines=%-4s %s rawFAIL=%d' % (b,nm,ex,rl,det,rawfail))
    for r in red: print('%-28s   %s %s checks / %s failures'%(b,r[0],r[1],r[2]))
    for i in info: print('%-28s   %s'%(b,i))
    if not v: print('%-28s   NO VERDICT LINE (dead suite?)'%b)
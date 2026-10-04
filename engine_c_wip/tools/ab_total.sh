#!/bin/bash
# Total ^FAIL across an ab_run.sh sweep. Reads FAIL_OUT=/FAIL_ERR= and, where the
# shell emitted a bare extra "0" line (grep -c prints 0 and the `|| echo 0`
# fallback adds another), takes the LAST numeric token on the logical record.
# Case-SENSITIVE, no colon, stdout AND stderr -- the documented counting rule.
f="$1"
tot=0; br=0; sil=0; n=0
python - "$f" <<'PY'
import re,sys
tot=0;br=0;sil=0;n=0
txt=open(sys.argv[1],encoding='utf-8',errors='replace').read().replace('\r','')
for line in txt.split('\n'):
    if not line.startswith('parity_') and not line.startswith('audit_'): continue
    n+=1
    m=re.search(r'BUILD_RED',line)
    if m: br+=1; continue
    o=re.search(r'FAIL_OUT=(\d+)',line); e=re.search(r'FAIL_ERR=(\d+)',line)
    ov=int(o.group(1)) if o else 0
    # the record may carry a trailing bare "0" on the NEXT line; ignore extra lines
    ev=int(e.group(1)) if e else 0
    tot+=ov+ev
    if 'SILENT=1' in line: sil+=1
print("suites=%d BUILD_RED=%d SILENT=%d TOTAL_FAIL=%d"%(n,br,sil,tot))
PY
"""Re-cite the stale `phases.rs:N` references in src/turn/phase.c.

COMMENT-ONLY. Zero behavioural change: no executable byte in phase.c is touched,
only the digits inside `/* ... */` and `//` prose. That matters because the whole
point of this edit is to stop the next reader from following a line that now
points at a different function -- the defect that made
rb_resolver_handle_heart_selection cite choice.rs:3354 (which is
trigger_auto_abilities_for_movement_current) and sent earlier attempts after the
wrong literal.

Ground rules honoured:
  * read/write with encoding='utf-8', newline='' (never Get-Content/Set-Content)
  * SHA-256 before and after; the script ABORTS if any line's executable text moved
  * every replacement is counted and asserted; a zero count is a failure, not a pass
"""
import hashlib, io, os, re, sys

P = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine_c_wip\src\turn\phase.c'
RS = r'C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\turn\phases.rs'


def sha(b):
    return hashlib.sha256(b).hexdigest()


raw = open(P, 'rb').read()
before = sha(raw)
text = raw.decode('utf-8')
# phase.c is CRLF. Splitting on '\n' leaves the '\r' at each line's end and
# joining with '\n' reconstructs the file byte-for-byte, so the \r needs no
# special handling -- but writing with newline='' below is what guarantees
# Python performs NO newline translation of its own.
CRLF = '\r\n' in text
print(f'newline style  : {"CRLF" if CRLF else "LF"}')

ph = open(RS, encoding='utf-8').read().split('\n')
real = {}
for i, l in enumerate(ph, 1):
    m = re.match(r'\s*(pub(?:\([^)]*\))?\s+)?fn\s+([a-z_0-9]+)', l)
    if m and m.group(2) not in real:
        real[m.group(2)] = i

CIT = re.compile(r'([a-z_0-9]{4,})(\s*\((phases\.rs):)(\d+)(?:-(\d+))?(\))')

counts = {}
out = []
for line in text.split('\n'):
    def rep(m):
        sym, mid, fname, lo, hi, close = m.groups()
        if sym not in real:
            return m.group(0)
        r = real[sym]
        lo_i = int(lo)
        if lo_i == r:
            return m.group(0)
        # a range `A-B` shifts as a unit, preserving the original span length
        if hi is not None:
            span = int(hi) - lo_i
            new = f'{sym}{mid}{r}-{r + span}{close}'
        else:
            new = f'{sym}{mid}{r}{close}'
        counts[sym] = counts.get(sym, 0) + 1
        return new
    out.append(CIT.sub(rep, line))

new_text = '\n'.join(out)

# --- safety: no EXECUTABLE byte may change ---
def strip_comments_and_numbers(s):
    # remove block comments, line comments, then all digit runs
    s = re.sub(r'/\*.*?\*/', ' ', s, flags=re.S)
    s = re.sub(r'//[^\n]*', ' ', s)
    return re.sub(r'\d+', '#', s)

a, b = strip_comments_and_numbers(text), strip_comments_and_numbers(new_text)
if a != b:
    print('ABORT: executable text changed')
    for i, (x, y) in enumerate(zip(a.split('\n'), b.split('\n'))):
        if x != y:
            print(f'  line {i+1}:\n    was {x[:120]}\n    now {y[:120]}')
            break
    sys.exit(1)

# --- also: the ONLY textual difference must be inside a comment ---
if new_text.count('phases.rs') != text.count('phases.rs'):
    print('ABORT: citation count changed')
    sys.exit(1)

open(P, 'w', encoding='utf-8', newline='').write(new_text)

raw2 = open(P, 'rb').read()
print(f'sha256 before : {before}')
print(f'sha256 after  : {sha(raw2)}')
print(f'bytes         : {len(raw)} -> {len(raw2)}')
print(f'executable text identical : YES (verified)')
print(f're-cited symbols ({len(counts)} distinct, {sum(counts.values())} citations):')
for k in sorted(counts):
    print(f'   {k:<44} x{counts[k]}  -> phases.rs:{real[k]}')
if sum(counts.values()) == 0:
    print('ABORT: zero replacements -- a zero count is a FAILURE here, not a pass')
    sys.exit(1)